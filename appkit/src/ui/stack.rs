use reactive_core::{BoxedComponent, Component, SetupContext, Signal};
use ui_core::widgets;
use ui_core::widgets::Alignment;

pub struct Stack {
    children: Vec<BoxedComponent>,
    alignment: Option<Box<dyn Signal<Value = Alignment>>>,
}

impl widgets::Stack for Stack {
    fn new() -> Self {
        Self {
            children: Vec::new(),
            alignment: None,
        }
    }

    fn alignment(mut self, alignment: impl Signal<Value = Alignment> + 'static) -> Self {
        self.alignment = Some(Box::new(alignment));
        self
    }

    fn child(mut self, child: impl Component + 'static) -> Self {
        self.children.push(Box::new(child));
        self
    }
}

impl Component for Stack {
    fn setup(self: Box<Self>, _ctx: &mut SetupContext) {
        todo!("AppKit Stack layout is not implemented yet");
    }
}
