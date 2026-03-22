use std::ffi::c_void;

use objc2::ffi::{OBJC_ASSOCIATION_RETAIN_NONATOMIC, objc_setAssociatedObject};
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{ClassType, DefinedClass, MainThreadOnly, define_class, msg_send};
use objc2_foundation::*;

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = Box<dyn Fn(&AnyObject)>]
    #[name = "ActionTarget"]
    pub struct ActionTarget;

    unsafe impl NSObjectProtocol for ActionTarget {}

    impl ActionTarget {
        #[unsafe(method(performAction:))]
        fn perform_action(&self, sender: &AnyObject) {
            self.ivars()(sender);
        }
    }
);

static ACTION_TARGET_KEY: u8 = 0;

impl ActionTarget {
    pub fn new(callback: impl Fn(&AnyObject) + 'static, mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(Box::new(callback) as Box<dyn Fn(&AnyObject)>);
        unsafe { msg_send![super(this), init] }
    }

    pub fn as_object(&self) -> &AnyObject {
        self.as_super().as_super()
    }

    pub fn attach_to(this: Retained<Self>, owner: &AnyObject) {
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
