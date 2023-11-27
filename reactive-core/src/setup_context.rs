use std::{any::Any, rc::Rc};

use futures::Future;

use crate::{
    clean_up::{BoxedCleanUp, CleanUp},
    component::BoxedComponent,
    context::{ContextKey, ContextMap},
    data::{UserDataKey, UserDataMap},
    effect::Effect,
    effect_run::new_effect_run,
    memory_run::new_memory_run,
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
    pub(crate) clean_ups: Vec<BoxedCleanUp>,
    pub children: Vec<BoxedComponent>,
    pub(crate) user_data: UserDataMap,
}

impl SetupContext {
    pub(crate) fn new(data: SetupContextData) -> Self {
        Self {
            data,
            clean_ups: Default::default(),
            children: Default::default(),
            user_data: Default::default(),
        }
    }

    pub fn mount_node(mut self, component: BoxedComponent) -> Node {
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
            clean_ups: self.clean_ups,
            children,
            user_data: self.user_data,
        }
    }
}

impl SetupContext {
    pub fn node_id(&self) -> NodeID {
        self.data.node_id
    }

    pub fn create_effect(&mut self, effect: impl Effect) {
        new_effect_run(self, effect);
    }

    pub fn create_effect_fn<F>(&mut self, effect: F)
    where
        F: for<'a> FnMut(&'a mut EffectContext) -> () + 'static,
    {
        new_effect_run(self, effect);
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

    pub fn create_memo<F, T>(&mut self, factory: F) -> impl Signal<Value = T>
    where
        T: 'static,
        F: FnMut() -> T + 'static,
    {
        new_memory_run(self, factory)
    }

    pub fn on_clean_up(&mut self, clean_up: impl CleanUp) {
        self.clean_ups.push(Box::new(clean_up));
    }

    pub fn scoped_object<T: 'static>(&mut self, obj: T) {
        let obj: Box<dyn Any> = Box::new(obj);
        self.on_clean_up(obj);
    }

    pub fn set_user_data<T>(&mut self, key: &'static UserDataKey<T>, value: T) {
        self.user_data.put(key, value);
    }
}

pub struct ResourceResult<TF, T> {
    pub trigger: TF,
    pub state: SignalReader<Resource<T>>,
    pub update: SignalWriter<Resource<T>>,
}
