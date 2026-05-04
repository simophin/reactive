use crate::apple::action_target::ActionTarget;
use crate::widgets::NativeView;
use crate::{apple_view_props, widgets};
use objc2::rc::Retained;
use objc2::{ClassType, sel};
use objc2_app_kit::{NSButton, NSView};
use objc2_foundation::{MainThreadMarker, NSString};
use reactive_core::Signal;

pub type Button = NativeView<Retained<NSView>, Retained<NSButton>>;

apple_view_props! {
    Button on NSButton {
        title: String;
        pub enabled: bool;
        pub highlighted: bool;
    }
}

impl widgets::Button for Button {
    fn new(title: impl Signal<Value = String> + 'static, on_click: impl Fn() + 'static) -> Self {
        NativeView::new(
            move |_| {
                let mtm = MainThreadMarker::new().expect("must be on main thread");
                let target = ActionTarget::new(move |_| on_click(), mtm);
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
            },
            |button| button.into_super().into_super(),
            move |_, _| {},
            Default::default(),
            &super::VIEW_REGISTRY_KEY,
        )
        .bind(PROP_TITLE, title)
    }

    fn enabled(self, value: impl Signal<Value = bool> + 'static) -> Self {
        self.bind(PROP_ENABLED, value)
    }
}
