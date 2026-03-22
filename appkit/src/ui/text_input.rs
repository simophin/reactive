use apple::Prop;
use objc2::ffi::{OBJC_ASSOCIATION_RETAIN_NONATOMIC, objc_setAssociatedObject};
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{ClassType, DefinedClass, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::*;
use objc2_foundation::*;
use reactive_core::Signal;
use std::ffi::c_void;

use super::view_component::AppKitViewComponent;

pub type TextInput = AppKitViewComponent<NSTextField, ()>;

/// Bindable placeholder hint text.
pub static PROP_PLACEHOLDER: &Prop<TextInput, NSTextField, String> = &Prop::new(|tf, value| {
    tf.setPlaceholderString(Some(&NSString::from_str(&value)));
});

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = Box<dyn Fn(String)>]
    #[name = "TextInputActionTarget"]
    struct TextInputActionTarget;

    unsafe impl NSObjectProtocol for TextInputActionTarget {}

    impl TextInputActionTarget {
        #[unsafe(method(performAction:))]
        fn perform_action(&self, sender: &AnyObject) {
            if let Some(tf) = sender.downcast_ref::<NSTextField>() {
                self.ivars()(tf.stringValue().to_string());
            }
        }
    }
);

static TEXT_INPUT_TARGET_KEY: u8 = 0;

impl TextInputActionTarget {
    fn new(callback: impl Fn(String) + 'static, mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(Box::new(callback) as Box<dyn Fn(String)>);
        unsafe { msg_send![super(this), init] }
    }

    fn attach_to(this: Retained<Self>, owner: &AnyObject) {
        let key = &TEXT_INPUT_TARGET_KEY as *const u8 as *const c_void;
        let value = this.as_super().as_super() as *const AnyObject as *mut AnyObject;
        unsafe {
            objc_setAssociatedObject(
                owner as *const AnyObject as *mut AnyObject,
                key,
                value,
                OBJC_ASSOCIATION_RETAIN_NONATOMIC,
            );
        }
    }
}

impl TextInput {
    /// Creates an editable text field. `on_submit` is called with the current text
    /// when the user presses Return.
    pub fn new_text_input(
        placeholder: impl Signal<Value = String> + 'static,
        on_submit: impl Fn(String) + 'static,
    ) -> Self {
        let mut c = AppKitViewComponent::create(
            move |_| {
                let mtm = MainThreadMarker::new().expect("must be on main thread");
                let target = TextInputActionTarget::new(on_submit, mtm);
                let tf = NSTextField::textFieldWithString(&NSString::new(), mtm);
                unsafe {
                    tf.setTarget(Some(target.as_super().as_super()));
                    tf.setAction(Some(sel!(performAction:)));
                }
                TextInputActionTarget::attach_to(target, tf.as_super().as_super());
                tf
            },
            |view: Retained<NSTextField>| view.into_super().into_super(),
        );
        c.as_mut().bind(PROP_PLACEHOLDER, placeholder);
        c
    }
}
