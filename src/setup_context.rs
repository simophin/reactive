use async_broadcast::Sender;

use crate::{
    clean_up::{BoxedCleanUp, CleanUp},
    component::BoxedComponent,
    effect::{BoxedEffect, Effect},
    react_context::{new_node_id, new_signal_id, NodeID, SignalID, SignalNotifier},
    signal::{signal_pair, SignalReader, SignalWriter},
};

pub struct SetupContext {
    node_id: NodeID,
    signal_change_sender: Sender<SignalID>,
    pub effects: Vec<BoxedEffect>,
    pub clean_ups: Vec<BoxedCleanUp>,
    pub children: Vec<BoxedComponent>,
}

impl SetupContext {
    pub fn new(signal_change_sender: Sender<SignalID>) -> Self {
        Self {
            signal_change_sender,
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
        self.effects.push(Box::new(effect));
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
