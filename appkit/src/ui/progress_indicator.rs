use super::view_component::AppKitViewBuilder;
use super::view_component::AppKitViewComponent;
use crate::view_component::NoChildView;
use apple::Prop;
use objc2::rc::Retained;
use objc2::{MainThreadOnly, msg_send};
use objc2_app_kit::*;
use objc2_foundation::*;
use reactive_core::Signal;

pub type ProgressIndicator = AppKitViewComponent<NSProgressIndicator, NoChildView>;

apple::view_props! {
    ProgressIndicator on NSProgressIndicator {
        pub double_value: f64;
        pub min_value: f64;
        pub max_value: f64;
    }
}

pub static PROP_INDETERMINATE: &Prop<ProgressIndicator, NSProgressIndicator, bool> =
    &Prop::new(|pi, indeterminate| {
        pi.setIndeterminate(indeterminate);
        if indeterminate {
            unsafe { pi.startAnimation(None) };
        } else {
            unsafe { pi.stopAnimation(None) };
        }
    });

impl ProgressIndicator {
    pub fn new_bar(value: impl Signal<Value = f64> + 'static) -> Self {
        Self(AppKitViewBuilder::create_no_child(
            |_| {
                let mtm = MainThreadMarker::new().expect("must be on main thread");
                let pi: Retained<NSProgressIndicator> =
                    unsafe { msg_send![NSProgressIndicator::alloc(mtm), init] };
                pi.setStyle(NSProgressIndicatorStyle::Bar);
                pi
            },
            |view: Retained<NSProgressIndicator>| view.into_super(),
        ))
    }

    pub fn new_spinner() -> Self {
        Self(AppKitViewBuilder::create_no_child(
            |_| {
                let mtm = MainThreadMarker::new().expect("must be on main thread");
                let pi: Retained<NSProgressIndicator> =
                    unsafe { msg_send![NSProgressIndicator::alloc(mtm), init] };
                pi.setStyle(NSProgressIndicatorStyle::Spinning);
                pi.setIndeterminate(true);
                unsafe { pi.startAnimation(None) };
                pi
            },
            |view: Retained<NSProgressIndicator>| view.into_super(),
        ))
    }
}
