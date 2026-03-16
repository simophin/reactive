use crate::signal::SignalId;
use crate::sorted_vec::SortedVec;
use slotmap::new_key_type;
use std::any::Any;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::pin::Pin;
use std::rc::Rc;

new_key_type! {
    pub struct ComponentId;
}

pub type ContextKeyId = *const ();

pub struct ContextKey<T>(PhantomData<T>);

impl<T> ContextKey<T> {
    pub const fn new() -> Self {
        Self(PhantomData)
    }

    pub const fn id(&self) -> ContextKeyId {
        self as *const ContextKey<T> as *const ()
    }
}

pub(crate) type BoxedEffectFn = Box<
    dyn for<'a> FnMut(&'a mut crate::EffectContext<'_>, Option<&mut Box<dyn Any>>) -> Box<dyn Any>,
>;

pub(crate) struct Effect {
    pub effect_fn: BoxedEffectFn,
    pub last_value: Box<dyn Any>,
    pub last_accessed_signals: SortedVec<SignalId>,
}

pub(crate) type ResourceProducerFn = Box<
    dyn for<'a> FnMut(
        &'a mut crate::EffectContext<'_>,
    ) -> Pin<Box<dyn Future<Output = Box<dyn Any>>>>,
>;

pub(crate) struct Resource {
    pub signal_id: SignalId,
    pub deps: SortedVec<SignalId>,
    pub producer: ResourceProducerFn,
    pub pending_future: Option<Pin<Box<dyn Future<Output = Box<dyn Any>>>>>,
}

pub(crate) struct StreamSubscription {
    pub signal_id: SignalId,
    pub stream: Pin<Box<dyn futures::Stream<Item = Box<dyn Any>>>>,
}

pub struct ComponentScope {
    pub(crate) parent: Option<ComponentId>,
    pub(crate) children: Vec<ComponentId>,
    pub(crate) effects: Vec<Effect>,
    pub(crate) resources: Vec<Resource>,
    pub(crate) streams: Vec<StreamSubscription>,
    pub(crate) cleanup: Vec<Box<dyn FnOnce()>>,
    pub(crate) context: Rc<HashMap<ContextKeyId, SignalId>>,
}

impl ComponentScope {
    pub(crate) fn new(parent: Option<ComponentId>) -> Self {
        Self {
            parent,
            children: Vec::new(),
            effects: Vec::new(),
            resources: Vec::new(),
            streams: Vec::new(),
            cleanup: Vec::new(),
            context: Rc::new(HashMap::new()),
        }
    }
}
