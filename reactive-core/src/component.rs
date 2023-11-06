use crate::setup_context::SetupContext;

pub trait Component: 'static {
    fn setup(self: Box<Self>, ctx: &mut SetupContext);

    fn content_type(&self) -> Option<&'static str> {
        None
    }
}

pub type BoxedComponent = Box<dyn Component>;

pub fn boxed_component(component: impl Component) -> BoxedComponent {
    Box::new(component)
}

impl<F, C> Component for F
where
    F: for<'a> FnOnce(&'a mut SetupContext) -> C + 'static,
    C: Component,
{
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let c = self(ctx);
        ctx.children.push(Box::new(c));
    }
}

impl Component for BoxedComponent {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        (*self).setup(ctx)
    }
}

impl<C: Component> Component for (C, &'static str) {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        ctx.children.push(Box::new(self.0));
    }

    fn content_type(&self) -> Option<&'static str> {
        Some(self.1)
    }
}

impl Component for () {
    fn setup(self: Box<Self>, _ctx: &mut SetupContext) {}
}

impl Component for bool {
    fn setup(self: Box<Self>, _ctx: &mut SetupContext) {}
}

impl<C: Component> Component for Option<C> {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        if let Some(child) = *self {
            Box::new(child).setup(ctx);
        }
    }
}

impl<C: Component> Component for Vec<C> {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        for child in *self {
            Box::new(child).setup(ctx);
        }
    }
}

pub struct ComponentFactory(Box<dyn FnMut() -> BoxedComponent + 'static>);

impl ComponentFactory {
    pub fn create(&mut self) -> BoxedComponent {
        (self.0)()
    }

    pub fn empty() -> ComponentFactory {
        ComponentFactory(Box::new(|| Box::new(())))
    }
}

impl<F, C> From<F> for ComponentFactory
where
    F: FnMut() -> C + 'static,
    C: Component,
{
    fn from(mut value: F) -> Self {
        Self(Box::new(move || Box::new(value())))
    }
}
