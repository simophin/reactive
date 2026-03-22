use apple::{ActionTarget, Prop};
use objc2::rc::Retained;
use objc2::{ClassType, sel};
use objc2_app_kit::*;
use objc2_foundation::*;
use reactive_core::Signal;

use super::view_component::AppKitViewComponent;

pub type TextInput = AppKitViewComponent<NSTextField, ()>;

apple::view_props! {
    TextInput on NSTextField {
        enabled: bool;
    }
}

/// Bindable placeholder hint text. Uses a custom Prop because
/// `setPlaceholderString:` takes `Option<&NSString>` rather than `&NSString`.
pub static PROP_PLACEHOLDER: &Prop<TextInput, NSTextField, String> = &Prop::new(|tf, value| {
    tf.setPlaceholderString(Some(&NSString::from_str(&value)));
});

impl TextInput {
    /// Creates an editable text field. `on_submit` is called with the current
    /// text when the user presses Return.
    pub fn new_text_input(
        placeholder: impl Signal<Value = String> + 'static,
        on_submit: impl Fn(String) + 'static,
    ) -> Self {
        let mut c = AppKitViewComponent::create(
            move |_| {
                let mtm = MainThreadMarker::new().expect("must be on main thread");
                let target = ActionTarget::new(
                    move |sender| {
                        if let Some(tf) = sender.downcast_ref::<NSTextField>() {
                            on_submit(tf.stringValue().to_string());
                        }
                    },
                    mtm,
                );
                let tf = NSTextField::textFieldWithString(&NSString::new(), mtm);
                unsafe {
                    tf.setTarget(Some(target.as_object()));
                    tf.setAction(Some(sel!(performAction:)));
                }
                ActionTarget::attach_to(target, tf.as_super().as_super());
                tf
            },
            |view: Retained<NSTextField>| view.into_super().into_super(),
        );
        c.as_mut().bind(PROP_PLACEHOLDER, placeholder);
        c
    }
}
