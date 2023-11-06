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

impl Component for Option<BoxedComponent> {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        if let Some(child) = *self {
            child.setup(ctx);
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

impl Component for Vec<BoxedComponent> {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        for child in *self {
            child.setup(ctx);
        }
    }
}

pub trait ComponentFactory: 'static {
    type C: Component;

    fn create(&mut self) -> Self::C;
}

impl<F, C> ComponentFactory for F
where
    F: FnMut() -> C + 'static,
    C: Component,
{
    type C = C;

    fn create(&mut self) -> Self::C {
        (self)()
    }
}
