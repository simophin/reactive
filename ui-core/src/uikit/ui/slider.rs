use crate::Prop;
use crate::apple::action_target::ActionTarget;
use crate::widgets::{self, NativeView};
use objc2::rc::Retained;
use objc2::{MainThreadMarker, sel};
use objc2_ui_kit::{UIControlEvents, UISlider, UIView};
use reactive_core::Signal;
use std::ops::Range;

pub type Slider = NativeView<Retained<UIView>, Retained<UISlider>>;

static PROP_VALUE: Prop<Slider, Retained<UISlider>, usize> =
    Prop::new(|view, value| view.setValue(value as f32));

static PROP_RANGE: Prop<Slider, Retained<UISlider>, Range<usize>> = Prop::new(|view, range| {
    view.setMinimumValue(range.start as f32);
    view.setMaximumValue(range.end as f32);
});

impl widgets::Slider for Slider {
    fn new(
        value: impl Signal<Value = usize> + 'static,
        range: impl Signal<Value = Range<usize>> + 'static,
        on_change: impl Fn(usize) + 'static,
    ) -> Self {
        NativeView::from_parts(
            move |_| {
                let mtm = MainThreadMarker::new().expect("must be on main thread");
                let slider = UISlider::new(mtm);
                let target = ActionTarget::new(
                    move |sender| {
                        if let Some(slider) = sender.downcast_ref::<UISlider>() {
                            on_change(slider.value().round().max(0.0) as usize);
                        }
                    },
                    mtm,
                );
                unsafe {
                    slider.addTarget_action_forControlEvents(
                        Some(target.as_object()),
                        sel!(performAction:),
                        UIControlEvents::ValueChanged,
                    );
                }
                // UIControl does not retain its targets.
                ActionTarget::attach_to(target, &slider);
                slider
            },
            |slider| slider.into_super().into_super(),
            |_, _| {},
            Default::default(),
            &super::VIEW_REGISTRY_KEY,
        )
        // Range first, so the initial value is not clamped to the default 0..1.
        .bind(PROP_RANGE, range)
        .bind(PROP_VALUE, value)
    }
}
