use apple::{ActionTarget, Prop, ViewBuilder};
use objc2::ClassType;
use objc2::rc::Retained;
use objc2::{msg_send, sel};
use objc2_foundation::*;
use objc2_ui_kit::*;
use reactive_core::{BoxedSignal, Component, SetupContext, Signal, ext::SignalExt};

use super::context::PARENT_VIEW;

pub struct Button {
    title: BoxedSignal<String>,
    on_click: Box<dyn Fn()>,
    builder: ViewBuilder<UIButton>,
}

pub static PROP_ENABLED: &Prop<Button, UIButton, bool> = &Prop::new(|button, enabled| {
    button.setEnabled(enabled);
});

impl Button {
    pub fn new(
        title: impl Signal<Value = String> + 'static,
        on_click: impl Fn() + 'static,
    ) -> Self {
        Self {
            title: Box::new(title),
            on_click: Box::new(on_click),
            builder: ViewBuilder::new(),
        }
    }

    pub fn bind<T>(
        mut self,
        props: &'static Prop<Button, UIButton, T>,
        signal: impl Signal<Value = T> + 'static,
    ) -> Self {
        self.builder = self.builder.bind(props, signal);
        self
    }
}

impl Component for Button {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let mtm = MainThreadMarker::new().expect("must be on main thread");
        let target = ActionTarget::new(self.on_click, mtm);

        let button = UIButton::buttonWithType(UIButtonType::System, mtm);

        let title = self.title;
        let button_ref = button.clone();
        ctx.create_effect(move |_, _: Option<()>| {
            unsafe {
                msg_send![&*button_ref, setTitle: &*NSString::from_str(&title.read()), forState: UIControlState::Normal]
            }
        });

        self.builder.setup(ctx, &button);

        button.addTarget_action_forControlEvents(
            Some(target.as_object()),
            Some(sel!(performAction:)),
            UIControlEvents::TouchUpInside,
        );
        ActionTarget::attach_to(target, button.as_super().as_super().as_super());

        if let Some(parent) = ctx.use_context(&PARENT_VIEW) {
            parent
                .read()
                .add_child(button.clone().into_super().into_super().into_super());
        }

        ctx.on_cleanup(move || {
            let _button = button;
        });
    }
}
