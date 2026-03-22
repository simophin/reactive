use crate::ReactiveScope;
use crate::signal::{BoxedStoredSignal, SignalId};
use crate::sorted_vec::SortedVec;
use slotmap::new_key_type;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

new_key_type! {
    pub struct ComponentId;
}

pub type ContextKeyId = *const ();

pub struct ContextKey<T>(PhantomData<fn() -> T>);

impl<T> ContextKey<T> {
    pub const fn new() -> Self {
        Self(PhantomData)
    }

    pub const fn id(&'static self) -> ContextKeyId {
        self as *const ContextKey<T> as *const ()
    }
}

pub(crate) struct EffectState {
    pub signal_accessed: SortedVec<SignalId>,
    /// A future produced by this effect run, always transferred to `Effect::in_flight`
    /// immediately after the effect fn returns. Always `None` on a stored `Effect`.
    pub pending_future: Option<InFlightFuture>,
}

pub(crate) type BoxedEffectFn = Box<dyn FnMut(&mut ReactiveScope) -> EffectState>;

pub(crate) struct InFlightFuture {
    pub future: Pin<Box<dyn Future<Output = ()>>>,
    pub woken: Arc<AtomicBool>,
}

pub(crate) struct Effect {
    pub effect_fn: BoxedEffectFn,
    pub effect_state: EffectState,
    pub in_flight: Option<InFlightFuture>,
}

#[derive(Default)]
pub struct ComponentScope {
    pub(crate) parent: Option<ComponentId>,
    pub(crate) effects: Vec<Effect>,
    pub(crate) cleanup: Vec<Box<dyn FnOnce()>>,
    pub(crate) context: HashMap<ContextKeyId, BoxedStoredSignal>,
    pub(crate) children: Vec<ComponentId>,
}
