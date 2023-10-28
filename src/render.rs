use std::collections::{BTreeMap, HashSet};
use std::rc::{Rc, Weak};
use std::{cell::RefCell, collections::HashMap};

use crate::component::BoxedComponent;
use crate::signal::SignalState;

thread_local! {
    static CURRENT_SCOPES: RefCell<Option<Scope>> = RefCell::new(Default::default());
    static CURRENT_REGISTRY: RefCell<Option<Rc<RefCell<Registry>>>> = RefCell::new(None);
}

pub struct RenderContext;

impl RenderContext {
    pub fn new(root: BoxedComponent) -> Initialized {
        Initialized(root)
    }
}

type SignalID = usize;
type EffectID = usize;

struct SignalRegistryState {
    state: SignalState,
    dependant_effects: HashSet<EffectID>,
}

#[derive(Default)]
pub struct Registry {
    signals: BTreeMap<SignalID, RefCell<SignalRegistryState>>,
    effects: BTreeMap<EffectID, RefCell<Box<dyn FnMut() + 'static>>>,
}

impl Registry {
    pub fn current() -> Rc<RefCell<Self>> {
        CURRENT_REGISTRY.with(|cell| {
            cell.borrow()
                .as_ref()
                .expect("Registry::current() is only accessible during mounting")
                .clone()
        })
    }

    fn register_signal(&mut self, state: SignalState) -> SignalID {
        let signal_id = self
            .signals
            .last_key_value()
            .map(|(id, _)| id + 1)
            .unwrap_or(0);

        self.signals.insert(
            signal_id,
            RefCell::new(SignalRegistryState {
                state,
                dependant_effects: Default::default(),
            }),
        );

        signal_id
    }

    fn unregister_signal(&mut self, signal_id: SignalID) {
        self.signals.remove(&signal_id);
    }

    fn unregister_effect(&mut self, effect_id: EffectID) {
        self.effects.remove(&effect_id);
        self.signals.iter_mut().for_each(|(_, state)| {
            state.borrow_mut().dependant_effects.remove(&effect_id);
        });
    }

    fn register_effect(&mut self, effect: impl FnMut() + 'static) -> EffectID {
        let effect_id = self
            .effects
            .last_key_value()
            .map(|(id, _)| id + 1)
            .unwrap_or(0);

        self.effects
            .insert(effect_id, RefCell::new(Box::new(effect)));

        effect_id
    }
}

pub struct Initialized(BoxedComponent);

impl Initialized {
    pub fn mount(self) -> Mounted {
        let component = self.0;

        CURRENT_REGISTRY.with(|cell| {
            cell.replace(Default::default());
        });

        let scope = Self::mount_component(&component);

        let context = CURRENT_REGISTRY.with(|cell| {
            cell.borrow_mut()
                .take()
                .expect("context should be set during mounting")
        });

        Mounted {
            component,
            scope,
            context,
        }
    }

    fn mount_component(component: &BoxedComponent) -> Scope {
        CURRENT_SCOPES.with(|cell| {
            let mut borrow = cell.borrow_mut();
            borrow.replace(Default::default());
        });

        let mut children = Default::default();
        component.render(&mut children);

        let mut scope = CURRENT_SCOPES.with(|cell| {
            let mut borrow = cell.borrow_mut();
            borrow.take().expect("scope stack is empty")
        });

        for child in children {
            scope.children.push(Self::mount_component(&child));
        }

        scope
    }
}

pub struct Mounted {
    component: BoxedComponent,
    scope: Scope,
    context: Rc<RefCell<Registry>>,
}

impl Mounted {
    pub fn unmount(self) -> Initialized {
        Initialized(self.component)
    }
}

#[derive(Default)]
pub struct Scope {
    registry: Rc<RefCell<Registry>>,

    children: Vec<Scope>,
    cleanups: Vec<Box<dyn FnOnce() + 'static>>,
    effects: Vec<EffectID>,
    signals: Vec<SignalID>,
}

impl Scope {
    pub fn with_current<T>(f: impl FnOnce(Option<&mut Scope>) -> T) -> T {
        CURRENT_SCOPES.with(|cell| {
            let mut borrow = cell.borrow_mut();
            f(borrow.as_mut())
        })
    }

    pub fn add_clean_up_func(&mut self, func: impl FnOnce() + 'static) {
        self.cleanups.push(Box::new(func));
    }

    pub fn add_effect(&mut self, func: impl Fn() + 'static) {
        self.effects
            .push(self.registry.borrow_mut().register_effect(func));
    }

    pub fn add_signal(&mut self, state: SignalState) {
        let signal_id = self.registry.borrow_mut().register_signal(state);
        self.signals.push(signal_id);
    }
}

impl Drop for Scope {
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
