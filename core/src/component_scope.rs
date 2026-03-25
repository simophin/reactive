use crate::ReactiveScope;
use crate::signal::BoxedStoredSignal;
use crate::signal::stored::SignalId;
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

pub(crate) type ContextKeyId = *const ();

pub struct ContextKey<T>(PhantomData<fn() -> T>);

impl<T> ContextKey<T> {
    pub const fn new() -> Self {
        Self(PhantomData)
    }

    pub(crate) const fn id(&'static self) -> ContextKeyId {
        self as *const ContextKey<T> as *const ()
    }
}

pub(crate) struct InFlightFuture {
    pub future: Pin<Box<dyn Future<Output = ()>>>,
    pub woken: Arc<AtomicBool>,
}

pub(crate) type BoxedEffectFn =
    Box<dyn FnMut(&ReactiveScope) -> (SortedVec<SignalId>, Option<InFlightFuture>)>;

pub(crate) struct Effect {
    pub effect_fn: BoxedEffectFn,
    pub signal_accessed: SortedVec<SignalId>,
    pub in_flight: Option<InFlightFuture>,
}

impl ComponentScope {
    pub(crate) fn push_effect(&mut self, effect: Effect) {
        if effect.signal_accessed.is_empty() && effect.in_flight.is_none() {
            self.inert_effects.push(effect.effect_fn);
        } else {
            self.active_effects.push(effect);
        }
    }
}

#[derive(Default)]
pub struct ComponentScope {
    pub(crate) parent: Option<ComponentId>,
    pub(crate) active_effects: Vec<Effect>,
    /// Closures with no signal deps and no future — can never be re-triggered,
    /// kept only so their captured state lives as long as the component.
    pub(crate) inert_effects: Vec<BoxedEffectFn>,
    pub(crate) cleanup: Vec<Box<dyn FnOnce()>>,
    pub(crate) context: HashMap<ContextKeyId, BoxedStoredSignal>,
    pub(crate) children: Vec<ComponentId>,
}
