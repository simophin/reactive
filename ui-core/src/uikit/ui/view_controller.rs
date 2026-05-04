use objc2_foundation::MainThreadMarker;
use objc2_ui_kit::UIViewController;
use reactive_core::{BoxedComponent, Component, SetupContext};

use super::context::{PARENT_VIEW, ViewParent};

pub struct ViewController {
    children: Vec<BoxedComponent>,
}

impl ViewController {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
        }
    }

    pub fn child(mut self, c: impl Component + 'static) -> Self {
        self.children.push(Box::new(c));
        self
    }
}

impl Component for ViewController {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let mtm = MainThreadMarker::new().expect("must be on main thread");
        let vc = UIViewController::new(mtm);
        let view = vc.view();

        ctx.provide_context(&PARENT_VIEW, ViewParent::Window(view.clone()));

        ctx.on_cleanup(move || {
            let _ = vc;
        });

        for child in self.children {
            let mut child_ctx = ctx.new_child();
            child.setup(&mut child_ctx);
        }
    }
}
