use std::rc::Rc;

use futures::Future;

use crate::{
    clean_up::{BoxedCleanUp, CleanUp},
    component::BoxedComponent,
    context::{ContextKey, ContextMap},
    effect::Effect,
    effect_run::EffectRun,
    node::Node,
    react_context::{new_node_id, new_signal_id, NodeID, SignalNotifier},
    resource::{new_resource_effect, Resource, ResourceFactory},
    signal::{signal, SignalReader, SignalWriter},
    tasks_queue::TaskQueueRef,
    util::signal_broadcast::Sender,
    EffectContext, Signal, SignalGetter,
};

#[derive(Clone)]
pub(crate) struct SetupContextData {
    pub node_id: NodeID,
    pub queue: TaskQueueRef,
    pub signal_sender: Sender,
    pub context_map: Rc<ContextMap>,
}

pub struct SetupContext {
    pub(crate) data: SetupContextData,
    pub(crate) effects: Vec<EffectRun>,
    pub(crate) clean_ups: Vec<BoxedCleanUp>,
    pub(crate) children: Vec<BoxedComponent>,
}

impl SetupContext {
    pub(crate) fn new(data: SetupContextData) -> Self {
        Self {
            data,
            effects: Default::default(),
            clean_ups: Default::default(),
            children: Default::default(),
        }
    }

    pub fn mount_node(mut self, component: BoxedComponent) -> Node {
        let content_type = component.content_type();
        component.setup(&mut self);

        let node_id = self.data.node_id;

        // Set up children first
        let children = self
            .children
            .into_iter()
            .map(|c| {
                SetupContext::new(SetupContextData {
                    node_id: new_node_id(),
                    ..self.data.clone()
                })
                .mount_node(c)
            })
            .collect();

        Node {
            id: node_id,
            effects: self.effects,
            clean_ups: self.clean_ups,
            children,
            content_type,
        }
    }
}

impl SetupContext {
    pub fn node_id(&self) -> NodeID {
        self.data.node_id
    }

    pub fn create_effect(&mut self, effect: impl Effect) {
        self.effects.push(EffectRun::new(self.data.clone(), effect));
    }

    pub fn create_effect_fn<F>(&mut self, effect: F)
    where
        F: for<'a> FnMut(&'a mut EffectContext) -> () + 'static,
    {
        self.effects.push(EffectRun::new(self.data.clone(), effect));
    }

    pub fn create_effect_simple<F>(&mut self, mut effect: F)
    where
        F: FnMut() -> () + 'static,
    {
        self.create_effect_fn(move |_ctx| effect());
    }

    pub fn create_signal<T: 'static>(
        &mut self,
        initial_value: T,
    ) -> (SignalReader<T>, SignalWriter<T>) {
        let id = new_signal_id();
        signal(
            initial_value,
            SignalNotifier::new(id, self.data.signal_sender.clone()),
        )
    }

    pub fn create_resource<S, F, T, FutT>(
        &mut self,
        input_signal: S,
        factory: F,
    ) -> ResourceResult<impl FnMut() + Clone + 'static, T>
    where
        S: Signal,
        <S as Signal>::Value: Clone,
        F: ResourceFactory<S::Value, FutT>,
        FutT: Future<Output = T> + 'static,
        T: 'static,
    {
        let (state_r, state_w) = self.create_signal(Resource::default());
        let (trigger_r, mut trigger_w) = self.create_signal(());

        let input_signal = move || {
            let _ = trigger_r.get();
            input_signal.get()
        };

        self.create_effect(new_resource_effect(input_signal, state_w.clone(), factory));

        ResourceResult {
            trigger: move || trigger_w.update(()),
            state: state_r,
            update: state_w,
        }
    }

    pub fn create_resource_fn<S, F, O, FutO>(
        &mut self,
        input_signal: S,
        factory: F,
    ) -> ResourceResult<impl FnMut() + Clone + 'static, O>
    where
        S: Signal,
        <S as Signal>::Value: Clone,
        F: FnMut(S::Value) -> FutO + 'static,
        FutO: Future<Output = O> + 'static,
        O: 'static,
    {
        self.create_resource(input_signal, factory)
    }

    pub fn use_context<T: 'static>(
        &self,
        key: &'static ContextKey<T>,
    ) -> Option<impl Signal<Value = T>> {
        self.data.context_map.get(key)
    }

    pub fn require_context<T: 'static>(
        &self,
        key: &'static ContextKey<T>,
    ) -> impl Signal<Value = T> {
        self.use_context(key).expect("Context not found")
    }

    pub fn on_clean_up(&mut self, clean_up: impl CleanUp) {
        self.clean_ups.push(Box::new(clean_up));
    }
}

pub struct ResourceResult<TF, T> {
    pub trigger: TF,
    pub state: SignalReader<Resource<T>>,
    pub update: SignalWriter<Resource<T>>,
}
