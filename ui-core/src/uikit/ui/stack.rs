use objc2::rc::Retained;
use objc2::{MainThreadOnly, msg_send};
use objc2_foundation::*;
use objc2_ui_kit::*;
use reactive_core::{BoxedComponent, Component, SetupContext};

use super::context::{PARENT_VIEW, ViewParent};

fn setup_stack(
    axis: UILayoutConstraintAxis,
    spacing: f64,
    children: Vec<BoxedComponent>,
    ctx: &mut SetupContext,
) {
    let mtm = MainThreadMarker::new().expect("must be on main thread");
    let stack: Retained<UIStackView> = unsafe { msg_send![UIStackView::alloc(mtm), init] };
    stack.setAxis(axis);
    stack.setSpacing(spacing as objc2::ffi::CGFloat);

    if let Some(parent) = ctx.use_context(&PARENT_VIEW) {
        parent
            .read()
            .add_child(stack.clone().into_super().into_super());
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
            UILayoutConstraintAxis::Vertical,
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
            UILayoutConstraintAxis::Horizontal,
            self.spacing,
            self.children,
            ctx,
        );
    }
}
