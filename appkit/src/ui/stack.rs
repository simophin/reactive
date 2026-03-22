use objc2::{MainThreadOnly, msg_send};
use objc2::rc::Retained;
use objc2_app_kit::*;
use objc2_foundation::*;
use reactive_core::{BoxedComponent, Component, SetupContext, Signal};

use super::context::{PARENT_VIEW, ViewParent};

fn setup_stack(
    orientation: NSUserInterfaceLayoutOrientation,
    spacing: f64,
    children: Vec<BoxedComponent>,
    ctx: &mut SetupContext,
) {
    let mtm = MainThreadMarker::new().expect("must be on main thread");
    let stack: Retained<NSStackView> = unsafe { msg_send![NSStackView::alloc(mtm), init] };
    stack.setOrientation(orientation);
    stack.setSpacing(spacing);

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

pub struct VStack {
    spacing: f64,
    children: Vec<BoxedComponent>,
}

impl VStack {
    pub fn new() -> Self {
        Self {
            spacing: 8.0,
            children: Vec::new(),
        }
    }

    pub fn spacing(mut self, s: f64) -> Self {
        self.spacing = s;
        self
    }

    pub fn child(mut self, c: impl Component + 'static) -> Self {
        self.children.push(Box::new(c));
        self
    }
}

impl Component for VStack {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        setup_stack(
            NSUserInterfaceLayoutOrientation::Vertical,
            self.spacing,
            self.children,
            ctx,
        );
    }
}

pub struct HStack {
    spacing: f64,
    children: Vec<BoxedComponent>,
}

impl HStack {
    pub fn new() -> Self {
        Self {
            spacing: 8.0,
            children: Vec::new(),
        }
    }

    pub fn spacing(mut self, s: f64) -> Self {
        self.spacing = s;
        self
    }

    pub fn child(mut self, c: impl Component + 'static) -> Self {
        self.children.push(Box::new(c));
        self
    }
}

impl Component for HStack {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        setup_stack(
            NSUserInterfaceLayoutOrientation::Horizontal,
            self.spacing,
            self.children,
            ctx,
        );
    }
}
