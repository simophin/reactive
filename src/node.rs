use crate::{clean_up::BoxedCleanUp, effect_run::EffectRun, react_context::NodeID};

pub struct Node {
    pub id: NodeID,
    pub effects: Vec<EffectRun>,
    pub clean_ups: Vec<BoxedCleanUp>,
    pub children: Vec<Node>,
}

impl Node {
    pub fn find_by(&mut self, id: NodeID) -> Option<&mut Self> {
        if self.id == id {
            return Some(self);
        }

        for child in &mut self.children {
            if let Some(found) = child.find_by(id) {
                return Some(found);
            }
        }

        None
    }
}

impl Drop for Node {
    fn drop(&mut self) {
        self.children.clear();

        for clean_up in self.clean_ups.drain(..) {
            clean_up.clean_up();
        }
    }
}
