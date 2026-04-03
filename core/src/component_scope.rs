use crate::signal::stored::SignalId;
use crate::sorted_vec::SortedVec;
use crate::{ReactiveScope, Signal, TypeErasedSignal};
use slotmap::new_key_type;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, LazyLock};

new_key_type! {
    pub struct ComponentId;
}

pub(crate) type ContextKeyId = usize;

pub struct ContextKey<T> {
    id: LazyLock<ContextKeyId>,
    _marker: PhantomData<fn() -> T>,
}

impl<T> ContextKey<T> {
    pub const fn new() -> Self {
        Self {
            id: LazyLock::new(|| {
                static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
                NEXT_ID.fetch_add(1, Ordering::SeqCst)
            }),
            _marker: PhantomData,
        }
    }

    pub(crate) fn id(&'static self) -> ContextKeyId {
        self.id.read()
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
    pub(crate) context: HashMap<ContextKeyId, TypeErasedSignal>,
    pub(crate) children: Vec<ComponentId>,
}
