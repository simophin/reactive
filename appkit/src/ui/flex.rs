use reactive_core::{
    BoxedComponent, Component, IntoSignal, SetupContext, Signal, SignalExt, TypedBoxedSignal,
};
use ui_core::layout::CrossAxisAlignment;

pub struct Flex {
    vertical: bool,
    children: Vec<BoxedComponent>,
    spacing: Option<Box<dyn Signal<Value = usize>>>,
    cross_axis_alignment: CrossAxisAlignment,
}

impl Flex {
    fn new(vertical: bool) -> Self {
        Self {
            vertical,
            spacing: None,
            children: Vec::new(),
            cross_axis_alignment: CrossAxisAlignment::Start,
        }
    }
}

impl ui_core::widgets::Row for Flex {
    fn new() -> Self {
        Self::new(false)
    }
    fn spacing(mut self, spacing: impl Signal<Value = usize> + 'static) -> Self {
        self.spacing.replace(Box::new(spacing));
        self
    }
    fn cross_axis_alignment(mut self, alignment: CrossAxisAlignment) -> Self {
        self.cross_axis_alignment = alignment;
        self
    }
    fn child(mut self, child: impl Component + 'static) -> Self {
        self.children.push(Box::new(child));
        self
    }
}

impl ui_core::widgets::Column for Flex {
    fn new() -> Self {
        Self::new(true)
    }

    fn spacing(mut self, spacing: impl Signal<Value = usize> + 'static) -> Self {
        self.spacing.replace(Box::new(spacing));
        self
    }
    fn cross_axis_alignment(mut self, alignment: CrossAxisAlignment) -> Self {
        self.cross_axis_alignment = alignment;
        self
    }
    fn child(mut self, child: impl Component + 'static) -> Self {
        self.children.push(Box::new(child));
        self
    }
}

impl Component for Flex {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        todo!()
    }
}
