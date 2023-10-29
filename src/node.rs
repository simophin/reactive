use std::{cell::RefCell, rc::Rc};

use crate::registry::{EffectID, Registry, SignalID};

thread_local! {
    static CURRENT_NODE: RefCell<Option<Node>> = Default::default();
}

pub struct Node {
    registry: Rc<RefCell<Registry>>,
    children: Vec<Node>,
    cleanups: Vec<Box<dyn FnOnce() + 'static>>,
    effects: Vec<EffectID>,
    signals: Vec<SignalID>,
}

impl Node {
    pub fn new(registry: Rc<RefCell<Registry>>) -> Self {
        Self {
            registry,
            children: Default::default(),
            cleanups: Default::default(),
            effects: Default::default(),
            signals: Default::default(),
        }
    }

    pub fn set_current(node: Option<Self>) -> Option<Self> {
        CURRENT_NODE.with(move |cell| match node {
            Some(node) => cell.borrow_mut().replace(node),
            None => cell.borrow_mut().take(),
        })
    }

    pub fn with_current<T>(f: impl FnOnce(Option<&mut Node>) -> T) -> T {
        CURRENT_NODE.with(|cell| {
            let mut borrow = cell.borrow_mut();
            f(borrow.as_mut())
        })
    }

    pub fn add_clean_up_func(&mut self, func: impl FnOnce() + 'static) {
        self.cleanups.push(Box::new(func));
    }

    pub fn add_effect(&mut self, func: impl FnMut() + 'static) {
        self.effects
            .push(self.registry.borrow_mut().register_effect(func));
    }

    pub fn add_signal(&mut self) -> SignalID {
        let signal_id = self.registry.borrow_mut().register_signal();
        self.signals.push(signal_id);
        signal_id
    }

    pub fn append_child(&mut self, child: Self) {
        self.children.push(child);
    }
}

impl Drop for Node {
    fn drop(&mut self) {
        // Clear children first
        self.children.clear();

        // Unregister signals and effects
        let mut registry = self.registry.borrow_mut();
        for signal_id in self.signals.drain(..) {
            registry.unregister_signal(signal_id);
        }

        for effect_id in self.effects.drain(..) {
            registry.unregister_effect(effect_id);
        }

        for cleanup in self.cleanups.drain(..) {
            cleanup();
        }
    }
}
