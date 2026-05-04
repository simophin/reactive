use std::ops::Range;

use crate::Prop;
use crate::apple::action_target::ActionTarget;
use crate::widgets;
use crate::widgets::NativeView;
use objc2::rc::Retained;
use objc2::{ClassType, sel};
use objc2_app_kit::{NSSlider, NSView};
use objc2_foundation::MainThreadMarker;
use reactive_core::Signal;

pub type Slider = NativeView<Retained<NSView>, Retained<NSSlider>>;

// NSSlider works in f64, and `range` maps to two setters — neither fits
// view_props! directly, so we use custom statics.
static PROP_VALUE: Prop<Slider, Retained<NSSlider>, usize> = Prop::new(|view, value| {
    view.setDoubleValue(value as f64);
});

static PROP_RANGE: Prop<Slider, Retained<NSSlider>, Range<usize>> = Prop::new(|view, range| {
    view.setMinValue(range.start as f64);
    view.setMaxValue(range.end as f64);
});

impl widgets::Slider for Slider {
    fn new(
        value: impl Signal<Value = usize> + 'static,
        range: impl Signal<Value = Range<usize>> + 'static,
        on_change: impl Fn(usize) + 'static,
    ) -> Self {
        NativeView::new(
            move |_| {
                let mtm = MainThreadMarker::new().expect("must be on main thread");
                let target = ActionTarget::new(
                    move |sender| {
                        if let Some(slider) = sender.downcast_ref::<NSSlider>() {
                            on_change(slider.doubleValue() as usize);
                        }
                    },
                    mtm,
                );
                let slider = unsafe {
                    NSSlider::sliderWithValue_minValue_maxValue_target_action(
                        0.0,
                        0.0,
                        1.0,
                        Some(target.as_object()),
                        Some(sel!(performAction:)),
                        mtm,
                    )
                };
                ActionTarget::attach_to(target, slider.as_super().as_super());
                slider
            },
            |slider| slider.into_super().into_super(),
            move |_, _| {},
            Default::default(),
            &super::VIEW_REGISTRY_KEY,
        )
        .bind(PROP_VALUE, value)
        .bind(PROP_RANGE, range)
    }
}
