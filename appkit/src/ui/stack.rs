use apple::bindable::BindableView;
use objc2::rc::Retained;
use objc2::{MainThreadOnly, msg_send};
use objc2_app_kit::*;
use objc2_foundation::*;
use reactive_core::{BoxedComponent, Component, IntoSignal, SetupContext, Signal};

use super::context::{PARENT_VIEW, ViewParent};

apple::view_props! {
    Stack on NSStackView {
        orientation: NSUserInterfaceLayoutOrientation;
        spacing: f64;
    }
}

fn finish_stack_setup(
    stack: Retained<NSStackView>,
    children: Vec<BoxedComponent>,
    ctx: &mut SetupContext,
) {
    if let Some(parent) = ctx.use_context(&PARENT_VIEW) {
        parent.read().add_child(stack.clone().into_super());
    }
    ctx.provide_context(&PARENT_VIEW, ViewParent::Stack(stack.clone()));
    ctx.on_cleanup(move || {
        let _ = stack;
    });
    for child in children {
        let mut child_ctx = ctx.new_child();
        child.setup(&mut child_ctx);
    }
}

pub struct Stack {
    builder: apple::ViewBuilder<NSStackView>,
    children: Vec<BoxedComponent>,
}

impl BindableView<NSStackView> for Stack {
    fn get_builder(&mut self) -> &mut apple::ViewBuilder<NSStackView> {
        &mut self.builder
    }
}

impl Stack {
    pub fn new(orientation: NSUserInterfaceLayoutOrientation) -> Self {
        let mut builder = apple::ViewBuilder::new(|_| {
            let mtm = MainThreadMarker::new().expect("must be on main thread");
            let stack: Retained<NSStackView> = unsafe { msg_send![NSStackView::alloc(mtm), init] };
            stack
        });
        builder.bind(PROP_ORIENTATION, orientation.into_signal());
        Self {
            builder,
            children: Vec::new(),
        }
    }

    pub fn vertical() -> Self {
        Self::new(NSUserInterfaceLayoutOrientation::Vertical)
    }

    pub fn horizontal() -> Self {
        Self::new(NSUserInterfaceLayoutOrientation::Horizontal)
    }

    pub fn child(mut self, c: impl Component + 'static) -> Self {
        self.children.push(Box::new(c));
        self
    }
}

impl Component for Stack {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let stack = self.builder.setup(ctx);
        finish_stack_setup(stack, self.children, ctx);
    }
}
