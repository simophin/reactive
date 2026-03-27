use super::view_component::AppKitViewBuilder;
use crate::view_component::{AppKitViewComponent, NoChildView};
use apple::{ActionTarget, Prop};
use objc2::{ClassType, sel};
use objc2_app_kit::*;
use objc2_foundation::*;
use reactive_core::Signal;

pub type Checkbox = AppKitViewComponent<NSButton, NoChildView>;

apple::view_props! {
    Checkbox on NSButton {
        title: String;
        pub enabled: bool;
    }
}

pub static PROP_CHECKED: &Prop<Checkbox, NSButton, bool> = &Prop::new(|btn, checked| {
    btn.setState(if checked {
        NSControlStateValueOn
    } else {
        NSControlStateValueOff
    });
});

impl Checkbox {
    pub fn new_checkbox(
        label: impl Signal<Value = String> + 'static,
        checked: impl Signal<Value = bool> + 'static,
        on_change: impl Fn() + 'static,
    ) -> Self {
        Self(
            AppKitViewBuilder::create_no_child(
                move |_| {
                    let mtm = MainThreadMarker::new().expect("must be on main thread");
                    let target = ActionTarget::new(move |_| on_change(), mtm);
                    let checkbox = unsafe {
                        NSButton::checkboxWithTitle_target_action(
                            &NSString::new(),
                            Some(target.as_object()),
                            Some(sel!(performAction:)),
                            mtm,
                        )
                    };
                    ActionTarget::attach_to(target, checkbox.as_super().as_super());
                    checkbox
                },
                |view| view.into_super().into_super(),
            )
            .bind(PROP_TITLE, label)
            .bind(PROP_CHECKED, checked),
        )
    }
}
