use apple::ActionTarget;
use apple::ViewBuilder;
use apple::bindable::BindableView;
use objc2::{ClassType, sel};
use objc2_app_kit::*;
use objc2_foundation::*;
use reactive_core::{Component, SetupContext, Signal};

use super::context::PARENT_VIEW;

apple::view_props! {
    Button on NSButton {
        title: String;
        enabled: bool;
        highlighted: bool;
    }
}

pub struct Button {
    builder: ViewBuilder<NSButton>,
}

impl BindableView<NSButton> for Button {
    fn get_builder(&mut self) -> &mut ViewBuilder<NSButton> {
        &mut self.builder
    }
}

impl Button {
    pub fn new(
        title: impl Signal<Value = String> + 'static,
        on_click: impl Fn() + 'static,
    ) -> Self {
        let mut builder = ViewBuilder::new(move |_| {
            let mtm = MainThreadMarker::new().expect("must be on main thread");
            let target = ActionTarget::new(on_click, mtm);

            let button = unsafe {
                NSButton::buttonWithTitle_target_action(
                    &NSString::new(),
                    Some(target.as_object()),
                    Some(sel!(performAction:)),
                    mtm,
                )
            };

            ActionTarget::attach_to(target, button.as_super().as_super());
            button
        });
        builder.bind(PROP_TITLE, title);
        Self { builder }
    }
}

impl Component for Button {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let button = self.builder.setup(ctx);

        if let Some(parent) = ctx.use_context(&PARENT_VIEW) {
            parent
                .read()
                .add_child(button.clone().into_super().into_super());
        }

        ctx.on_cleanup(move || {
            let _button = button;
        });
    }
}
