use crate::ui::gtk_view::GtkViewComponent;
use glib::object::Cast;
use gtk4::prelude::{RangeExt, ScaleExt};
use reactive_core::Signal;
use std::ops::Range;
use ui_core::Prop;
use ui_core::widgets;
use ui_core::widgets::NativeView;

pub type Slider = GtkViewComponent<gtk4::Scale>;

pub static PROP_VALUE: Prop<Slider, gtk4::Scale, usize> =
    Prop::new(|scale, value| scale.set_value(value as f64));

pub static PROP_RANGE: Prop<Slider, gtk4::Scale, Range<usize>> = Prop::new(|scale, range| {
    scale.set_range(range.start as f64, range.end as f64);
});

impl widgets::Slider for Slider {
    fn new(
        value: impl Signal<Value = usize> + 'static,
        range: impl Signal<Value = Range<usize>> + 'static,
        on_change: impl Fn(usize) + 'static,
    ) -> Self {
        Self(
            NativeView::new(
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
                |_, _| {},
                Default::default(),
                &super::VIEW_REGISTRY_KEY,
            )
            .bind(PROP_VALUE, value)
            .bind(PROP_RANGE, range),
        )
    }
}
