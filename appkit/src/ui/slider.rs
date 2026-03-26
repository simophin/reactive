use apple::ActionTarget;
use objc2::{ClassType, sel};
use objc2_app_kit::*;
use objc2_foundation::*;
use reactive_core::Signal;

use super::view_component::AppKitViewComponent;

pub type Slider = AppKitViewComponent<NSSlider, ()>;

apple::view_props! {
    Slider on NSSlider {
        pub min_value: f64;
        pub max_value: f64;
        pub double_value: f64;
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
                let target = ActionTarget::new(
                    move |sender| {
                        if let Some(slider) = sender.downcast_ref::<NSSlider>() {
                            on_change(slider.doubleValue());
                        }
                    },
                    mtm,
                );
                let slider = unsafe {
                    NSSlider::sliderWithValue_minValue_maxValue_target_action(
                        0.0,
                        min,
                        max,
                        Some(target.as_object()),
                        Some(sel!(performAction:)),
                        mtm,
                    )
                };
                ActionTarget::attach_to(target, slider.as_super().as_super());
                slider
            },
            |view| view.into_super().into_super(),
        );
        c.as_mut().bind(PROP_DOUBLE_VALUE, value);
        c
    }
}
