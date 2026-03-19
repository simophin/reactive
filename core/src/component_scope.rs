use crate::ReactiveScope;
use crate::signal::{BoxedStoredSignal, SignalId};
use crate::sorted_vec::SortedVec;
use slotmap::new_key_type;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::pin::Pin;
use std::rc::Rc;
use std::task::Context;

new_key_type! {
    pub struct ComponentId;
}

pub type ContextKeyId = *const ();

pub struct ContextKey<T>(PhantomData<fn() -> T>);

impl<T> ContextKey<T> {
    pub const fn new() -> Self {
        Self(PhantomData)
    }

    pub const fn id(&self) -> ContextKeyId {
        self as *const ContextKey<T> as *const ()
    }
}

pub(crate) struct EffectState {
    pub signal_accessed: SortedVec<SignalId>,
    pub pending_future: Option<Pin<Box<dyn Future<Output = ()>>>>,
}

pub(crate) struct Effect {
    pub effect_fn: Box<dyn FnMut(&mut ReactiveScope) -> EffectState>,
    pub effect_state: EffectState,
}

pub struct ComponentScope {
    pub(crate) parent: Option<ComponentId>,
    pub(crate) children: Vec<ComponentId>,
    pub(crate) effects: Vec<Effect>,
    pub(crate) cleanup: Vec<Box<dyn FnOnce()>>,
    pub(crate) context: Rc<HashMap<ContextKeyId, BoxedStoredSignal>>,
}

impl ComponentScope {
    pub(crate) fn new(parent: Option<ComponentId>) -> Self {
        Self {
            parent,
            children: Vec::new(),
            effects: Vec::new(),
            cleanup: Vec::new(),
            context: Rc::new(HashMap::new()),
        }
    }
}
