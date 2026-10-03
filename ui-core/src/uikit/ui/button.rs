use super::flex::invalidate_layout;
use crate::Prop;
use crate::apple::action_target::ActionTarget;
use crate::widgets::{self, NativeView};
use objc2::rc::Retained;
use objc2::{MainThreadMarker, sel};
use objc2_foundation::NSString;
use objc2_ui_kit::{UIButton, UIButtonType, UIControlEvents, UIControlState, UIView};
use reactive_core::Signal;

pub type Button = NativeView<Retained<UIView>, Retained<UIButton>>;

pub static PROP_TITLE: Prop<Button, Retained<UIButton>, String> = Prop::new(|view, title| {
    view.setTitle_forState(Some(&NSString::from_str(&title)), UIControlState::Normal);
    invalidate_layout(view);
});

pub static PROP_ENABLED: Prop<Button, Retained<UIButton>, bool> =
    Prop::new(|view, enabled| view.setEnabled(enabled));

impl widgets::Button for Button {
    fn new(title: impl Signal<Value = String> + 'static, on_click: impl Fn() + 'static) -> Self {
        NativeView::from_parts(
            move |_| {
                let mtm = MainThreadMarker::new().expect("must be on main thread");
                let button = UIButton::buttonWithType(UIButtonType::System, mtm);
                let target = ActionTarget::new(move |_| on_click(), mtm);
                unsafe {
                    button.addTarget_action_forControlEvents(
                        Some(target.as_object()),
                        sel!(performAction:),
                        UIControlEvents::PrimaryActionTriggered,
                    );
                }
                // UIControl does not retain its targets.
                ActionTarget::attach_to(target, &button);
                button
            },
            |button| button.into_super().into_super(),
            |_, _| {},
            Default::default(),
            &super::VIEW_REGISTRY_KEY,
        )
        .bind(PROP_TITLE, title)
    }

    fn enabled(self, value: impl Signal<Value = bool> + 'static) -> Self {
        self.bind(PROP_ENABLED, value)
    }
}
