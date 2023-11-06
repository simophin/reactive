use crate::{
    clean_up::{BoxedCleanUp, CleanUp},
    component::BoxedComponent,
    effect_context::EffectContext,
    effect_run::EffectRun,
    node::Node,
    react_context::{new_node_id, new_signal_id, NodeID, SignalNotifier},
    // resource::{ResourceFactory, ResourceRun},
    signal::{signal, SignalReader, SignalWriter},
    tasks_queue::TaskQueueRef,
    util::signal_broadcast::Sender,
};

pub struct SetupContext {
    node_id: NodeID,
    queue: TaskQueueRef,
    signal_sender: Sender,
    pub effects: Vec<EffectRun>,
    pub clean_ups: Vec<BoxedCleanUp>,
    // pub resources: Vec<ResourceRun>,
    pub children: Vec<BoxedComponent>,
}

impl SetupContext {
    pub fn new(signal_sender: Sender, queue: TaskQueueRef) -> Self {
        Self {
            signal_sender,
            queue,
            node_id: new_node_id(),
            // resources: Default::default(),
            effects: Default::default(),
            clean_ups: Default::default(),
            children: Default::default(),
        }
    }

    pub fn mount_node(mut self, component: BoxedComponent) -> Node {
        let content_type = component.content_type();
        component.setup(&mut self);

        // Set up children first
        let children = self
            .children
            .into_iter()
            .map(|c| {
                SetupContext::new(self.signal_sender.clone(), self.queue.clone()).mount_node(c)
            })
            .collect();

        Node {
            id: self.node_id,
            effects: self.effects,
            clean_ups: self.clean_ups,
            children,
            content_type,
        }
    }
}

impl SetupContext {
    pub fn node_id(&self) -> NodeID {
        self.node_id
    }

    pub fn create_effect(&mut self, effect: impl FnMut(&mut EffectContext) + 'static) {
        self.effects.push(EffectRun::new(
            self.node_id,
            self.signal_sender.clone(),
            &self.queue,
            effect,
        ));
    }

    pub fn create_effect_simple<F>(&mut self, mut effect: F)
    where
        F: FnMut() -> () + 'static,
    {
        self.create_effect(move |_ctx| effect());
    }

    pub fn create_signal<T: 'static>(
        &mut self,
        initial_value: T,
    ) -> (SignalReader<T>, SignalWriter<T>) {
        let id = new_signal_id();
        signal(
            initial_value,
            SignalNotifier::new(id, self.signal_sender.clone()),
        )
    }

    // pub fn create_resource<S, F, T>(
    //     &mut self,
    //     signal: S,
    //     factory: F,
    // ) -> ResourceResult<impl Fn() + 'static, impl for<'a> Signal<Value<'a> = Option<&'a T>>>
    // where
    //     T: 'static,
    //     S: for<'a> Signal<Value<'a> = &'a S>,
    //     F: ResourceFactory<S, T> + 'static,
    // {
    //     let (run, trigger) =
    //         ResourceRun::new(&self.queue, self.signal_sender.subscribe(), signal, factory);

    //     let result = ResourceResult {
    //         trigger: move || {
    //             let _ = trigger.try_send(());
    //         },
    //         result: run.as_signal(),
    //     };

    //     self.resources.push(run);
    //     result
    // }

    pub fn on_clean_up(&mut self, clean_up: impl CleanUp) {
        self.clean_ups.push(Box::new(clean_up));
    }
}

pub struct ResourceResult<TRI, S> {
    pub trigger: TRI,
    pub result: S,
}
