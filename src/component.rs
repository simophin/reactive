pub trait Component: 'static {
    fn render(&self, output: &mut Vec<BoxedComponent>);
}

pub type BoxedComponent = Box<dyn Component>;

impl<F, C> Component for F
where
    F: Fn() -> C + 'static,
    C: Component,
{
    fn render(&self, output: &mut Vec<BoxedComponent>) {
        self().render(output);
    }
}

impl Component for BoxedComponent {
    fn render(&self, output: &mut Vec<BoxedComponent>) {
        self.as_ref().render(output);
    }
}

impl Component for () {
    fn render(&self, _output: &mut Vec<BoxedComponent>) {}
}

impl Component for bool {
    fn render(&self, _output: &mut Vec<BoxedComponent>) {}
}

impl<C: Component> Component for Option<C> {
    fn render(&self, output: &mut Vec<BoxedComponent>) {
        if let Some(child) = self {
            child.render(output);
        }
    }
}

impl<C: Component> Component for Vec<C> {
    fn render(&self, output: &mut Vec<BoxedComponent>) {
        for child in self {
            child.render(output);
        }
    }
}
impl<C: Component, const N: usize> Component for [C; N] {
    fn render(&self, output: &mut Vec<BoxedComponent>) {
        for child in self {
            child.render(output);
        }
    }
}
