use crate::ReactiveScope;

pub trait Component {
    fn setup(&self, scope: &mut ReactiveScope);
}
