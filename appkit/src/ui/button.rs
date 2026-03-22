use super::action_target::ActionTarget;
use super::context::PARENT_VIEW;
use crate::ui::prop::Prop;
use objc2::rc::Retained;
use objc2::{ClassType, sel};
use objc2_app_kit::*;
use objc2_foundation::*;
use reactive_core::{BoxedSignal, Component, SetupContext, Signal, ext::SignalExt};

pub struct Button {
    title: BoxedSignal<String>,
    on_click: Box<dyn Fn()>,
    prop_bind_runs: Vec<Box<dyn FnOnce(&mut SetupContext, Retained<NSButton>)>>,
}

static PROP_TITLE: &Prop<Button, NSButton, String> = &Prop::new(|button, text| {
    button.setTitle(&NSString::from_str(&text));
});

pub static PROP_ENABLED: &Prop<Button, NSButton, bool> = &Prop::new(|button, enabled| {
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
            prop_bind_runs: Vec::new(),
        }
    }

    pub fn bind<T>(
        mut self,
        props: &'static Prop<Button, NSButton, T>,
        signal: impl Signal<Value = T> + 'static,
    ) -> Self {
        self.prop_bind_runs
            .push(Box::new(move |ctx, view| props.bind(ctx, view, signal)));
        self
    }
}

impl Component for Button {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let mtm = MainThreadMarker::new().expect("must be on main thread");
        let target = ActionTarget::new(self.on_click, mtm);

        let button = unsafe {
            NSButton::buttonWithTitle_target_action(
                &NSString::new(),
                Some(target.as_object()),
                Some(sel!(performAction:)),
                mtm,
            )
        };

        PROP_TITLE.bind(ctx, button.clone(), self.title);

        for run in self.prop_bind_runs {
            run(ctx, button.clone());
        }

        ActionTarget::attach_to(target, button.as_super().as_super());

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
