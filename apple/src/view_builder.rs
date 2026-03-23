use crate::prop::Prop;
use objc2::Message;
use objc2::rc::Retained;
use reactive_core::{SetupContext, Signal};

pub struct ViewBuilder<V> {
    create_view: Box<dyn FnOnce(&mut SetupContext) -> Retained<V>>,
    property_binders: Vec<Box<dyn FnOnce(&mut SetupContext, Retained<V>)>>,
}

impl<V> ViewBuilder<V>
where
    V: Message,
{
    pub fn new(creator: impl FnOnce(&mut SetupContext) -> Retained<V> + 'static) -> Self {
        Self {
            create_view: Box::new(creator),
            property_binders: vec![],
        }
    }

    pub fn bind<RustType, T>(
        &mut self,
        props: &'static Prop<RustType, V, T>,
        signal: impl Signal<Value = T> + 'static,
    ) {
        self.property_binders
            .push(Box::new(move |ctx, view| props.bind(ctx, view, signal)));
    }

    pub fn push_binder(&mut self, binder: impl FnOnce(&mut SetupContext, Retained<V>) + 'static) {
        self.property_binders.push(Box::new(binder));
    }

    pub fn setup(self, ctx: &mut SetupContext) -> Retained<V> {
        let view = (self.create_view)(ctx);
        for binder in self.property_binders {
            binder(ctx, view.clone());
        }
        view
    }
}
