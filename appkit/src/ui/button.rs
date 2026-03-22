use apple::ActionTarget;
use objc2::{ClassType, sel};
use objc2_app_kit::*;
use objc2_foundation::*;
use reactive_core::Signal;

use super::view_component::AppKitViewComponent;

pub type Button = AppKitViewComponent<NSButton, ()>;

apple::view_props! {
    Button on NSButton {
        title: String;
        enabled: bool;
        highlighted: bool;
    }
}

impl Button {
    pub fn new_button(
        title: impl Signal<Value = String> + 'static,
        on_click: impl Fn() + 'static,
    ) -> Self {
        let mut c = AppKitViewComponent::create(
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
            |view| view.into_super().into_super(),
        );
        c.as_mut().bind(PROP_TITLE, title);
        c
    }
}
