use crate::native::AppKitNativeView;
use apple::Prop;
use objc2::rc::Retained;
use objc2::{MainThreadOnly, msg_send};
use objc2_app_kit::*;
use objc2_foundation::*;
use reactive_core::{Signal, SignalExt};
use ui_core::widgets::NativeView;

pub type ProgressIndicator = AppKitNativeView<NSProgressIndicator, ()>;

apple::view_props! {
    ProgressIndicator on NSProgressIndicator {
        pub double_value: f64;
        pub min_value: f64;
        pub max_value: f64;
    }
}

pub static PROP_INDETERMINATE: Prop<ProgressIndicator, NSProgressIndicator, bool> =
    Prop::new(|pi, indeterminate| {
        pi.setIndeterminate(indeterminate);
        if indeterminate {
            unsafe { pi.startAnimation(None) };
        } else {
            unsafe { pi.stopAnimation(None) };
        }
    });

impl ProgressIndicator {
    fn build_bar(_value: impl Signal<Value = f64> + 'static) -> Self {
        Self(
            NativeView::new(
                |_| {
                    let mtm = MainThreadMarker::new().expect("must be on main thread");
                    let pi: Retained<NSProgressIndicator> =
                        unsafe { msg_send![NSProgressIndicator::alloc(mtm), init] };
                    pi.setStyle(NSProgressIndicatorStyle::Bar);
                    pi
                },
                |view| view.into_super(),
                move |_, _| {},
                Default::default(),
                &super::VIEW_REGISTRY_KEY,
            ),
            Default::default(),
        )
    }

    fn build_spinner() -> Self {
        Self(
            NativeView::new(
                |_| {
                    let mtm = MainThreadMarker::new().expect("must be on main thread");
                    let pi: Retained<NSProgressIndicator> =
                        unsafe { msg_send![NSProgressIndicator::alloc(mtm), init] };
                    pi.setStyle(NSProgressIndicatorStyle::Spinning);
                    pi.setIndeterminate(true);
                    unsafe { pi.startAnimation(None) };
                    pi
                },
                |view| view.into_super(),
                move |_, _| {},
                Default::default(),
                &super::VIEW_REGISTRY_KEY,
            ),
            Default::default(),
        )
    }

    pub fn new_bar(value: impl Signal<Value = f64> + 'static) -> Self {
        Self::build_bar(value)
    }

    pub fn new_spinner() -> Self {
        Self::build_spinner()
    }
}

impl ui_core::widgets::ProgressIndicator for ProgressIndicator {
    fn new_bar(value: impl Signal<Value = usize> + 'static) -> Self {
        Self::build_bar(value.map_value(|v| v as f64))
    }

    fn new_spinner() -> Self {
        Self::build_spinner()
    }
}
