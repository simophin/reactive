use crate::{clean_up::BoxedCleanUp, react_context::NodeID};
use crate::data::UserDataMap;

pub struct Node {
    pub id: NodeID,
    pub clean_ups: Vec<BoxedCleanUp>,
    pub children: Vec<Node>,
    pub user_data: UserDataMap,
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
