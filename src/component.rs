pub trait Component: 'static {
    fn render(&mut self, output: &mut Vec<BoxedComponent>);
}

pub type BoxedComponent = Box<dyn Component>;

pub fn boxed_component(component: impl Component + 'static) -> BoxedComponent {
    Box::new(component)
}

impl<F, C> Component for F
where
    F: FnMut() -> C + 'static,
    C: Component + 'static,
{
    fn render(&mut self, output: &mut Vec<BoxedComponent>) {
        let child: BoxedComponent = Box::new(self());
        output.push(child);
    }
}

impl Component for BoxedComponent {
    fn render(&mut self, output: &mut Vec<BoxedComponent>) {
        self.as_mut().render(output);
    }
}

impl Component for () {
    fn render(&mut self, _output: &mut Vec<BoxedComponent>) {}
}

impl Component for bool {
    fn render(&mut self, _output: &mut Vec<BoxedComponent>) {}
}

impl<C: Component> Component for Option<C> {
    fn render(&mut self, output: &mut Vec<BoxedComponent>) {
        if let Some(child) = self {
            child.render(output);
        }
    }
}

impl<C: Component> Component for Vec<C> {
    fn render(&mut self, output: &mut Vec<BoxedComponent>) {
        for child in self {
            child.render(output);
        }
    }
}
impl<C: Component, const N: usize> Component for [C; N] {
    fn render(&mut self, output: &mut Vec<BoxedComponent>) {
        for child in self {
            child.render(output);
        }
    }
}
