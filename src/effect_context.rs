use crate::{component::BoxedComponent, task::WeakTask, util::mpsc::Sender};

pub struct EffectContext {
    task_scheduler: Sender<WeakTask>,
}

impl EffectContext {
    pub fn new() -> Self {
        Self {}
    }

    pub fn queue_children_replacement(&mut self, children: Vec<BoxedComponent>) {
        //todo
    }
}
