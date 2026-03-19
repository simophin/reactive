use crate::ReactiveScope;
use crate::signal::{BoxedStoredSignal, SignalId};
use crate::sorted_vec::SortedVec;
use slotmap::new_key_type;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::pin::Pin;
use std::rc::Rc;

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

pub(crate) type BoxedEffectFn = Box<dyn FnMut(&mut ReactiveScope) -> EffectState>;

pub(crate) struct Effect {
    pub effect_fn: BoxedEffectFn,
    pub effect_state: EffectState,
}

#[derive(Default)]
pub struct ComponentScope {
    pub(crate) effects: Vec<Effect>,
    pub(crate) cleanup: Vec<Box<dyn FnOnce()>>,
    pub(crate) context: Rc<HashMap<ContextKeyId, BoxedStoredSignal>>,
    pub(crate) children: Vec<ComponentId>,
}
