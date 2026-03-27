use crate::view_component::{AppKitViewBuilder, AppKitViewComponent, NoChildView};
use apple::ActionTarget;
use objc2::{ClassType, sel};
use objc2_app_kit::NSButton;
use objc2_foundation::{MainThreadMarker, NSString};
use reactive_core::Signal;
use ui_core::widgets;

pub type Button = AppKitViewComponent<NSButton, NoChildView>;

apple::view_props! {
    Button on NSButton {
        title: String;
        pub enabled: bool;
        pub highlighted: bool;
    }
}

impl widgets::Button for Button {
    fn new(title: impl Signal<Value = String> + 'static, on_click: impl Fn() + 'static) -> Self {
        Self(
            AppKitViewBuilder::create_no_child(
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
            )
            .bind(PROP_TITLE, title),
        )
    }

    fn enabled(self, value: impl Signal<Value = bool> + 'static) -> Self {
        Self(self.0.bind(PROP_ENABLED, value))
    }
}
