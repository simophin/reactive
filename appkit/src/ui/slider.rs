use std::ops::Range;

use crate::view_component::{AppKitViewBuilder, AppKitViewComponent, NoChildView};
use apple::{ActionTarget, Prop};
use objc2::{ClassType, sel};
use objc2_app_kit::NSSlider;
use objc2_foundation::MainThreadMarker;
use reactive_core::Signal;
use ui_core::widgets;

pub type Slider = AppKitViewComponent<NSSlider, NoChildView>;

// NSSlider works in f64, and `range` maps to two setters — neither fits
// view_props! directly, so we use custom statics.
static PROP_VALUE: &Prop<Slider, NSSlider, usize> = &Prop::new(|view, value| {
    view.setDoubleValue(value as f64);
});

static PROP_RANGE: &Prop<Slider, NSSlider, Range<usize>> = &Prop::new(|view, range| {
    view.setMinValue(range.start as f64);
    view.setMaxValue(range.end as f64);
});

impl widgets::Slider for Slider {
    fn new(
        value: impl Signal<Value = usize> + 'static,
        range: impl Signal<Value = Range<usize>> + 'static,
        on_change: impl Fn(usize) + 'static,
    ) -> Self {
        Self(
            AppKitViewBuilder::create_no_child(
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
            )
            .bind(PROP_VALUE, value)
            .bind(PROP_RANGE, range),
        )
    }
}
