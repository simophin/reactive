use crate::{
    clean_up::{BoxedCleanUp, CleanUp},
    component::BoxedComponent,
    effect::Effect,
    effect_run::EffectRun,
    react_context::{new_node_id, new_signal_id, NodeID, SignalNotifier},
    signal::{signal_pair, SignalReader, SignalWriter},
    tasks_queue::TaskQueueRef,
    util::signal_broadcast::Sender,
};

pub struct SetupContext {
    node_id: NodeID,
    queue: TaskQueueRef,
    signal_sender: Sender,
    pub effects: Vec<EffectRun>,
    pub clean_ups: Vec<BoxedCleanUp>,
    pub children: Vec<BoxedComponent>,
}

impl SetupContext {
    pub fn new(signal_sender: Sender, queue: TaskQueueRef) -> Self {
        Self {
            signal_sender,
            queue,
            node_id: new_node_id(),
            effects: Default::default(),
            clean_ups: Default::default(),
            children: Default::default(),
        }
    }
}

impl SetupContext {
    pub fn node_id(&self) -> NodeID {
        self.node_id
    }

    pub fn create_effect(&mut self, effect: impl Effect) {
        self.effects.push(EffectRun::new(
            &self.queue,
            self.signal_sender.subscribe(),
            effect,
        ));
    }

    pub fn create_signal<T: 'static>(
        &mut self,
        initial_value: T,
    ) -> (SignalReader<T>, SignalWriter<T>) {
        let id = new_signal_id();
        signal_pair(
            initial_value,
            SignalNotifier::new(id, self.signal_sender.clone()),
        )
    }

    pub fn on_clean_up(&mut self, clean_up: impl CleanUp) {
        self.clean_ups.push(Box::new(clean_up));
    }

    pub fn set_children<'a>(&mut self, children: impl Iterator<Item = BoxedComponent> + 'a) {
        self.children = children.collect();
    }
}
