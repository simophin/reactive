use async_broadcast::{Receiver, Sender};

use crate::{
    clean_up::{BoxedCleanUp, CleanUp},
    component::BoxedComponent,
    effect::Effect,
    effect_run::EffectRun,
    react_context::{new_node_id, new_signal_id, NodeID, SignalID, SignalNotifier},
    signal::{signal_pair, SignalReader, SignalWriter},
    tasks_queue::TaskQueueRef,
};

pub struct SetupContext {
    node_id: NodeID,
    queue: TaskQueueRef,
    signal_change_sender: Sender<SignalID>,
    signal_change_receiver: Receiver<SignalID>,
    pub effects: Vec<EffectRun>,
    pub clean_ups: Vec<BoxedCleanUp>,
    pub children: Vec<BoxedComponent>,
}

impl SetupContext {
    pub fn new(
        signal_change_sender: Sender<SignalID>,
        signal_change_receiver: Receiver<SignalID>,
        queue: TaskQueueRef,
    ) -> Self {
        Self {
            signal_change_sender,
            signal_change_receiver,
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
            self.signal_change_receiver.clone(),
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
            SignalNotifier::new(id, self.signal_change_sender.clone()),
        )
    }

    pub fn on_clean_up(&mut self, clean_up: impl CleanUp) {
        self.clean_ups.push(Box::new(clean_up));
    }

    pub fn set_children<'a>(&mut self, children: impl Iterator<Item = BoxedComponent> + 'a) {
        self.children = children.collect();
    }
}
