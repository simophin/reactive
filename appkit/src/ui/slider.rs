use objc2::ffi::{OBJC_ASSOCIATION_RETAIN_NONATOMIC, objc_setAssociatedObject};
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{ClassType, DefinedClass, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::*;
use objc2_foundation::*;
use reactive_core::Signal;
use std::ffi::c_void;

use super::view_component::AppKitViewComponent;

pub type Slider = AppKitViewComponent<NSSlider, ()>;

apple::view_props! {
    Slider on NSSlider {
        min_value: f64;
        max_value: f64;
        double_value: f64;
    }
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = Box<dyn Fn(f64)>]
    #[name = "SliderActionTarget"]
    struct SliderActionTarget;

    unsafe impl NSObjectProtocol for SliderActionTarget {}

    impl SliderActionTarget {
        #[unsafe(method(performAction:))]
        fn perform_action(&self, sender: &AnyObject) {
            if let Some(slider) = sender.downcast_ref::<NSSlider>() {
                self.ivars()(slider.doubleValue());
            }
        }
    }
);

static SLIDER_TARGET_KEY: u8 = 0;

impl SliderActionTarget {
    fn new(callback: impl Fn(f64) + 'static, mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(Box::new(callback) as Box<dyn Fn(f64)>);
        unsafe { msg_send![super(this), init] }
    }

    fn attach_to(this: Retained<Self>, owner: &AnyObject) {
        let key = &SLIDER_TARGET_KEY as *const u8 as *const c_void;
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

impl Slider {
    pub fn new_slider(
        value: impl Signal<Value = f64> + 'static,
        min: f64,
        max: f64,
        on_change: impl Fn(f64) + 'static,
    ) -> Self {
        let mut c = AppKitViewComponent::create(
            move |_| {
                let mtm = MainThreadMarker::new().expect("must be on main thread");
                let target = SliderActionTarget::new(on_change, mtm);
                let slider = unsafe {
                    NSSlider::sliderWithValue_minValue_maxValue_target_action(
                        0.0,
                        min,
                        max,
                        Some(target.as_super().as_super()),
                        Some(sel!(performAction:)),
                        mtm,
                    )
                };
                SliderActionTarget::attach_to(target, slider.as_super().as_super());
                slider
            },
            |view| view.into_super().into_super(),
        );
        c.as_mut().bind(PROP_DOUBLE_VALUE, value);
        c
    }
}
