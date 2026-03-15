use crate::Scope;

pub trait Component {
    fn setup(&self, scope: &mut Scope);
}
