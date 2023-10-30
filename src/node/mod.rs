mod init;
mod mounting;

use std::{
    cell::{Ref, RefCell, RefMut},
    rc::{Rc, Weak},
};

use crate::{
    clean_up::CleanUp,
    component::BoxedComponent,
    effect::Effect,
    registry::{self, EffectID, EffectStateRef, Registry, RegistryRef, SignalID},
    tracker::Tracker,
    util::diff::{self, diff_sorted, DiffResult},
};

thread_local! {
    static CURRENT_NODE: RefCell<Option<NodeRef>> = Default::default();
}

#[derive(Clone)]
pub struct NodeRef(Rc<RefCell<Node>>);

#[derive(Clone)]
pub struct WeakNodeRef(Weak<RefCell<Node>>);

struct Node {
    is_mounted: bool,
    component: Option<BoxedComponent>,
    registry: RegistryRef,
    children: Vec<NodeRef>,
    cleanups: Vec<Box<dyn CleanUp>>,
    effects: Vec<EffectID>,
    signals: Vec<SignalID>,
}

impl NodeRef {
    pub fn set_current(node: Option<NodeRef>) -> Option<NodeRef> {
        CURRENT_NODE.with(move |cell| match node {
            Some(node) => cell.borrow_mut().replace(node),
            None => cell.borrow_mut().take(),
        })
    }

    pub fn require_current() -> WeakNodeRef {
        CURRENT_NODE.with(|cell| cell.borrow().as_ref().expect("No current node").downgrade())
    }

    pub fn with_current<T>(f: impl FnOnce(Option<&NodeRef>) -> T) -> T {
        CURRENT_NODE.with(|cell| {
            let mut borrow = cell.borrow();
            f(borrow.as_ref())
        })
    }
}

impl NodeRef {
    fn borrow(&self) -> Ref<Node> {
        self.0.borrow()
    }

    fn borrow_mut(&self) -> RefMut<Node> {
        self.0.borrow_mut()
    }

    pub fn new(registry: Rc<RefCell<Registry>>, component: BoxedComponent) -> NodeRef {
        Self(Rc::new(RefCell::new(Node {
            registry,
            is_mounted: false,
            component: Some(component),
            children: Default::default(),
            cleanups: Default::default(),
            effects: Default::default(),
            signals: Default::default(),
        })))
    }

    pub fn registry(&self) -> RegistryRef {
        self.borrow().registry.clone()
    }

    pub fn downgrade(&self) -> WeakNodeRef {
        WeakNodeRef(Rc::downgrade(&self.0))
    }

    pub fn is_mounted(&self) -> bool {
        self.borrow().is_mounted
    }

    pub fn mount(&self) {
        assert!(!self.is_mounted(), "Node is already mounted.");

        let mut children = Vec::new();
        let mut component = self
            .borrow_mut()
            .component
            .take()
            .expect("Component must not be taken");
        assert!(
            Self::set_current(Some(self.clone())).is_none(),
            "There's already a current node."
        );
        component.render(&mut children);
        Self::set_current(None)
            .expect("There's no current node.")
            .borrow_mut()
            .component
            .replace(component);

        self.borrow_mut().is_mounted = true;

        // Add children
        assert!(
            self.borrow().children.is_empty(),
            "Children must be empty during mount."
        );
        for child in children {
            self.append_child(child);
        }

        // Run all effects
        let mut dependency_changes = vec![];
        let this = self.borrow();
        for effect in this
            .registry
            .borrow()
            .get_effects(this.effects.iter().cloned())
            .collect::<Vec<_>>()
        {
            let mut effect = effect.borrow_mut();
            let Some(diff_result) = effect.run() else {
                continue;
            };

            dependency_changes.push((effect.id, diff_result));
        }
        let registry = this.registry.clone();
        drop(this);

        // Apply dependency changes
        registry
            .borrow_mut()
            .apply_dependency_changes(dependency_changes.into_iter());
    }

    pub fn unmount(&self) {
        let mut this = self.borrow_mut();
        assert!(this.is_mounted, "Node is not mounted.");

        // Unmount children
        for child in this.children.drain(..) {
            child.unmount();
        }

        // Unregister signals and effects
        let registry = this.registry.clone();
        let mut registry = registry.borrow_mut();

        for signal_id in this.signals.drain(..) {
            registry.remove_signal(signal_id);
        }

        for effect_id in this.effects.drain(..) {
            registry.remove_effect(effect_id);
        }

        for mut cleanup in this.cleanups.drain(..) {
            cleanup.clean_up();
        }

        this.is_mounted = false;
    }

    pub fn add_clean_up_func(&self, func: impl CleanUp) {
        self.borrow_mut().cleanups.push(Box::new(func));
    }

    pub fn add_effect(&self, effect: impl Effect) {
        let node = self.downgrade();
        let mut this = self.borrow_mut();
        let id = this.registry.borrow_mut().add_effect(node, effect).0;
        this.effects.push(id);
    }

    pub fn add_signal(&self) -> SignalID {
        let mut this = self.borrow_mut();

        let signal_id = this.registry.borrow_mut().add_signal();
        this.signals.push(signal_id);
        signal_id
    }

    pub fn append_child(&self, child: BoxedComponent) -> NodeRef {
        let mut this = self.borrow_mut();
        let child = Self::new(this.registry.clone(), child);
        this.children.push(child.clone());

        if this.is_mounted {
            child.mount();
        }
        child
    }

    pub fn remove_child(&self, child: &NodeRef) {
        let mut this = self.borrow_mut();
        let Some(index) = this
            .children
            .iter()
            .position(|c| Rc::ptr_eq(&c.0, &child.0))
        else {
            return;
        };

        this.children.remove(index);

        if child.is_mounted() {
            child.unmount();
        }
    }

    pub fn remove_all_children(&self) {
        let mut this = self.borrow_mut();
        for child in this.children.drain(..) {
            if child.is_mounted() {
                child.unmount();
            }
        }
    }

    pub fn notify_signal_changed(&self, id: SignalID) {
        let effects_to_run: Vec<_> = {
            let registry = self.registry();
            let registry = registry.borrow();
            if let Some(state) = registry.get_signal(id) {
                registry
                    .get_effects(state.borrow().tracked_effects.iter().cloned())
                    .collect()
            } else {
                Default::default()
            }
        };

        let mut changes = vec![];

        for effect in effects_to_run {
            let mut effect = effect.borrow_mut();
            let Some(result) = effect.run() else {
                continue;
            };

            changes.push((effect.id, result));
        }

        self.registry()
            .borrow_mut()
            .apply_dependency_changes(changes.into_iter());
    }
}

impl WeakNodeRef {
    pub fn upgrade(&self) -> Option<NodeRef> {
        self.0.upgrade().map(NodeRef)
    }
}
