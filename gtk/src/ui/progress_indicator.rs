use crate::view_component::{GtkViewBuilder, GtkViewComponent, NoChildWidget};
use gtk4::prelude::*;
use reactive_core::{Signal, SignalExt};
use ui_core::Prop;
use ui_core::widgets;

pub type ProgressIndicator = GtkViewComponent<gtk4::Widget, NoChildWidget>;

pub static PROP_FRACTION: &Prop<ProgressIndicator, gtk4::Widget, f64> =
    &Prop::new(|widget, fraction| {
        if let Some(bar) = widget.downcast_ref::<gtk4::ProgressBar>() {
            bar.set_fraction(fraction);
        }
    });

fn new_bar_widget(value: impl Signal<Value = f64> + 'static) -> ProgressIndicator {
    GtkViewComponent(
        GtkViewBuilder::create_no_child(
            |_| {
                let bar = gtk4::ProgressBar::new();
                bar.upcast::<gtk4::Widget>()
            },
            |w| w,
        )
        .bind(PROP_FRACTION, value),
    )
}

fn new_spinner_widget() -> ProgressIndicator {
    GtkViewComponent(GtkViewBuilder::create_no_child(
        |_| {
            let spinner = gtk4::Spinner::new();
            spinner.start();
            spinner.upcast::<gtk4::Widget>()
        },
        |w| w,
    ))
}

impl widgets::ProgressIndicator for ProgressIndicator {
    fn new_bar(value: impl Signal<Value = usize> + 'static) -> Self {
        new_bar_widget(value.map_value(|v| v as f64 / 100.0))
    }

    fn new_spinner() -> Self {
        new_spinner_widget()
    }
}
