mod init;
mod mounting;

use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use crate::{
    clean_up::CleanUp,
    component::Component,
    effect::Effect,
    registry::{EffectID, Registry, SignalID},
};

use derive_more::{Deref, DerefMut};

thread_local! {
    static CURRENT_NODE: RefCell<Option<Node>> = Default::default();
}

#[derive(Clone, Deref, DerefMut)]
pub struct Node(Rc<RefCell<Inner>>);

#[derive(Clone)]
pub struct WeakNode(Weak<RefCell<Inner>>);

struct Inner {
    is_mounted: bool,
    component: Box<dyn Component>,
    registry: Rc<RefCell<Registry>>,
    children: Vec<Node>,
    cleanups: Vec<Box<dyn CleanUp>>,
    effects: Vec<EffectID>,
    signals: Vec<SignalID>,
}

impl Node {
    pub fn set_current(node: Option<Node>) -> Option<Node> {
        CURRENT_NODE.with(move |cell| match node {
            Some(node) => cell.borrow_mut().replace(node),
            None => cell.borrow_mut().take(),
        })
    }

    pub fn require_current() -> WeakNode {
        CURRENT_NODE.with(|cell| cell.borrow().as_ref().expect("No current node").downgrade())
    }

    pub fn with_current<T>(f: impl FnOnce(Option<&Node>) -> T) -> T {
        CURRENT_NODE.with(|cell| {
            let mut borrow = cell.borrow();
            f(borrow.as_ref())
        })
    }
}

impl Node {
    pub fn new(registry: Rc<RefCell<Registry>>, component: Box<dyn Component>) -> Node {
        Self(Rc::new(RefCell::new(Inner {
            registry,
            is_mounted: false,
            component,
            children: Default::default(),
            cleanups: Default::default(),
            effects: Default::default(),
            signals: Default::default(),
        })))
    }

    pub fn downgrade(&self) -> WeakNode {
        WeakNode(Rc::downgrade(&self.0))
    }

    pub fn is_mounted(&self) -> bool {
        self.borrow().is_mounted
    }

    pub fn mount(&self) {
        assert!(!self.is_mounted(), "Node is already mounted.");

        self.borrow_mut().is_mounted = true;
    }

    pub fn umount(&self) {
        assert!(self.is_mounted(), "Node is not mounted.");

        self.borrow_mut().is_mounted = false;
    }

    pub fn add_clean_up_func(&self, func: impl CleanUp) {
        self.borrow_mut().cleanups.push(Box::new(func));
    }

    pub fn add_effect(&self, effect: impl Effect) {
        let mut this = self.borrow_mut();
        this.effects
            .push(this.registry.borrow_mut().add_effect(effect));
    }

    pub fn add_signal(&self) -> SignalID {
        let mut this = self.borrow_mut();

        let signal_id = this.registry.borrow_mut().add_signal();
        this.signals.push(signal_id);
        signal_id
    }

    pub fn append_child(&self, child: Node) {
        assert!(
            !child.is_mounted(),
            "Cannot append an already mounted child."
        );

        let mut this = self.borrow_mut();
        this.children.push(child);
        if this.is_mounted {
            child.mount();
        }
    }

    pub fn remove_child(&self, child: &Node) {
        let mut this = self.borrow_mut();
        let Some(index) = this.children.iter().position(|c| Rc::ptr_eq(&c, &child)) else {
            return;
        };

        this.children.remove(index).umount();
    }

    pub fn notify_signal_changed(&self, id: SignalID) {}
}

impl Drop for Inner {
    fn drop(&mut self) {
        // Clear children first
        self.children.clear();

        // Unregister signals and effects
        let mut registry = self.registry.borrow_mut();
        for signal_id in self.signals.drain(..) {
            registry.remove_signal(signal_id);
        }

        for effect_id in self.effects.drain(..) {
            registry.remove_effect(effect_id);
        }

        for mut cleanup in self.cleanups.drain(..) {
            cleanup.clean_up();
        }
    }
}
