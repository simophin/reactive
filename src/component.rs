use crate::setup_context::SetupContext;

pub trait Component: 'static {
    fn setup(&mut self, ctx: &mut SetupContext);

    fn name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}

pub type BoxedComponent = Box<dyn Component>;

pub fn boxed_component(component: impl Component) -> BoxedComponent {
    Box::new(component)
}

impl<F, C> Component for F
where
    F: for<'a> Fn(&'a mut SetupContext) -> C + 'static,
    C: Component,
{
    fn setup(&mut self, ctx: &mut SetupContext) {
        let c = self(ctx);
        ctx.children.push(Box::new(c));
    }
}

impl Component for BoxedComponent {
    fn setup(&mut self, ctx: &mut SetupContext) {
        self.as_mut().setup(ctx);
    }
}

impl Component for () {
    fn setup(&mut self, ctx: &mut SetupContext) {}
}

impl Component for bool {
    fn setup(&mut self, ctx: &mut SetupContext) {}
}

impl<C: Component> Component for Option<C> {
    fn setup(&mut self, ctx: &mut SetupContext) {
        if let Some(child) = self {
            child.setup(ctx);
        }
    }
}

impl<C: Component> Component for Vec<C> {
    fn setup(&mut self, ctx: &mut SetupContext) {
        for child in self {
            child.setup(ctx);
        }
    }
}
impl<C: Component, const N: usize> Component for [C; N] {
    fn setup(&mut self, ctx: &mut SetupContext) {
        for child in self {
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
