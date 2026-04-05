use crate::view_component::{GtkViewBuilder, GtkViewComponent, NoChildWidget};
use gtk4::prelude::*;
use reactive_core::Signal;
use std::ops::Range;
use ui_core::Prop;
use ui_core::widgets;

pub type Slider = GtkViewComponent<gtk4::Scale, NoChildWidget>;

pub static PROP_VALUE: &Prop<Slider, gtk4::Scale, usize> =
    &Prop::new(|scale, value| scale.set_value(value as f64));

pub static PROP_RANGE: &Prop<Slider, gtk4::Scale, Range<usize>> = &Prop::new(|scale, range| {
    scale.set_range(range.start as f64, range.end as f64);
});

impl widgets::Slider for Slider {
    fn new(
        value: impl Signal<Value = usize> + 'static,
        range: impl Signal<Value = Range<usize>> + 'static,
        on_change: impl Fn(usize) + 'static,
    ) -> Self {
        Self(
            GtkViewBuilder::create_no_child(
                move |_| {
                    let scale =
                        gtk4::Scale::new(gtk4::Orientation::Horizontal, gtk4::Adjustment::NONE);
                    scale.set_draw_value(false);
                    scale.connect_value_changed(move |s| {
                        on_change(s.value() as usize);
                    });
                    scale
                },
                |s| s.upcast(),
            )
            .bind(PROP_VALUE, value)
            .bind(PROP_RANGE, range),
        )
    }
}
