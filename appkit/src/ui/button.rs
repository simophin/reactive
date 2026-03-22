use std::ffi::c_void;

use objc2::ffi::{OBJC_ASSOCIATION_RETAIN_NONATOMIC, objc_setAssociatedObject};
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{ClassType, DefinedClass, MainThreadOnly, define_class, msg_send, sel};
use reactive_core::Signal;
use objc2_app_kit::*;
use objc2_foundation::*;
use reactive_core::{Component, SetupContext};

use super::context::PARENT_VIEW;

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = Box<dyn Fn()>]
    #[name = "ActionTarget"]
    struct ActionTarget;

    unsafe impl NSObjectProtocol for ActionTarget {}

    impl ActionTarget {
        #[unsafe(method(performAction:))]
        fn perform_action(&self, _sender: &AnyObject) {
            self.ivars()();
        }
    }
);

// Unique address used as the association key.
static ACTION_TARGET_KEY: u8 = 0;

impl ActionTarget {
    fn new(callback: impl Fn() + 'static, mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(Box::new(callback) as Box<dyn Fn()>);
        unsafe { msg_send![super(this), init] }
    }

    fn as_object(&self) -> &AnyObject {
        self.as_super().as_super()
    }

    fn attach_to(this: Retained<Self>, owner: &AnyObject) {
        let key = &ACTION_TARGET_KEY as *const u8 as *const c_void;
        let value = this.as_object() as *const AnyObject as *mut AnyObject;
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

pub struct Button<F> {
    title: String,
    on_click: F,
}

impl<F: Fn() + 'static> Button<F> {
    pub fn new(title: impl Into<String>, on_click: F) -> Self {
        Self {
            title: title.into(),
            on_click,
        }
    }
}

impl<F: Fn() + 'static> Component for Button<F> {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let mtm = MainThreadMarker::new().expect("must be on main thread");
        let target = ActionTarget::new(self.on_click, mtm);

        let button = unsafe {
            NSButton::buttonWithTitle_target_action(
                &NSString::from_str(&self.title),
                Some(target.as_object()),
                Some(sel!(performAction:)),
                mtm,
            )
        };

        ActionTarget::attach_to(target, button.as_super().as_super());

        if let Some(parent) = ctx.use_context(&PARENT_VIEW) {
            parent.read().add_child(button.clone().into_super().into_super());
        }

        ctx.on_cleanup(move || {
            let _button = button;
        });
    }
}
