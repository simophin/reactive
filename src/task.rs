use crate::component::BoxedComponent;

pub enum Task {
    ReplaceAllChildren(ReplaceAllChildrenTask),
}

pub struct ReplaceAllChildrenTask(pub Vec<BoxedComponent>);
