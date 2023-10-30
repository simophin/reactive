use crate::setup::SetupContext;

pub trait Component: 'static {
    fn setup(&mut self);
}

pub type BoxedComponent = Box<dyn Component>;

pub fn boxed_component(component: impl Component + 'static) -> BoxedComponent {
    Box::new(component)
}

impl<F, C> Component for F
where
    F: Fn() -> C + 'static,
    C: Component + 'static,
{
    fn setup(&mut self) {
        let child: BoxedComponent = Box::new(self());
        SetupContext::with_current(|ctx| ctx.children.push(child));
    }
}

impl Component for BoxedComponent {
    fn setup(&mut self) {
        self.as_mut().setup();
    }
}

impl Component for () {
    fn setup(&mut self) {}
}

impl Component for bool {
    fn setup(&mut self) {}
}

impl<C: Component> Component for Option<C> {
    fn setup(&mut self) {
        if let Some(child) = self {
            child.setup();
        }
    }
}

impl<C: Component> Component for Vec<C> {
    fn setup(&mut self) {
        for child in self {
            child.setup();
        }
    }
}
impl<C: Component, const N: usize> Component for [C; N] {
    fn setup(&mut self) {
        for child in self {
            child.setup();
        }
    }
}
