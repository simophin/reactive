use std::rc::Rc;

use derive_builder::Builder;

use crate::{context::ContextKey, Component, SetupContext, Signal};

#[derive(Builder)]
#[builder(pattern = "owned")]
pub struct Provider<V, C, T>
where
    T: Clone + 'static,
    V: Signal,
    C: Component,
{
    key: &'static ContextKey<T>,
    value: V,
    child: C,
}

impl<V, C, T> Component for Provider<V, C, T>
where
    T: Clone + 'static,
    V: Signal<Value = T>,
    C: Component,
{
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let Self { value, child, key } = *self;
        let mut new_context_map = ctx.data.context_map.as_ref().clone();
        new_context_map.insert(key, value);
        ctx.data.context_map = Rc::new(new_context_map);
        ctx.children.push(Box::new(child));
    }
}
