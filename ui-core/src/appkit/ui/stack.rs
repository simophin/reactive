use crate::widgets;
use crate::widgets::{Alignment, Modifier, NativeViewRegistry};
use objc2::rc::Retained;
use objc2_app_kit::{NSStackView, NSView};
use reactive_core::{BoxedComponent, Component, ComponentId, SetupContext, Signal};

pub struct Stack {
    children: Vec<BoxedComponent>,
    alignment: Option<Box<dyn Signal<Value = Alignment>>>,
}

struct StackViewRegistry {
    my_view: NSStackView,
}

impl NativeViewRegistry<Retained<NSView>> for StackViewRegistry {
    fn update_view(&self, component_id: ComponentId, view: Retained<NSView>, modifier: Modifier) {
        self.my_view.addSubview(&view);
    }

    fn clear_view(&self, component_id: ComponentId, view: Retained<NSView>) {
        self.my_view.removeView(&view);
    }
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
