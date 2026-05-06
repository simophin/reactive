use crate::Prop;
use crate::widgets;
use crate::widgets::{Modifier, NativeView, WithModifier};
use glib::object::Cast;
use reactive_core::{Component, SetupContext, Signal, SignalExt};

pub enum ProgressIndicator {
    Bar(NativeView<gtk4::Widget, gtk4::ProgressBar>),
    Spinner(NativeView<gtk4::Widget, gtk4::Spinner>),
}

static PROP_FRACTION: Prop<ProgressIndicator, gtk4::ProgressBar, f64> =
    Prop::new(|widget, fraction| {
        widget.set_fraction(fraction);
    });

fn new_bar_widget(value: impl Signal<Value = f64> + 'static) -> ProgressIndicator {
    ProgressIndicator::Bar(
        NativeView::new(
            |_| gtk4::ProgressBar::new(),
            |w| w.upcast(),
            |_, _| {},
            Default::default(),
            &super::VIEW_REGISTRY_KEY,
        )
        .bind(PROP_FRACTION, value),
    )
}

fn new_spinner_widget() -> ProgressIndicator {
    ProgressIndicator::Spinner(NativeView::new(
        |_| {
            let spinner = gtk4::Spinner::new();
            spinner.start();
            spinner
        },
        |w| w.upcast(),
        |_, _| {},
        Default::default(),
        &super::VIEW_REGISTRY_KEY,
    ))
}

impl Component for ProgressIndicator {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        match *self {
            ProgressIndicator::Bar(bar) => {
                bar.setup_in_component(ctx);
            }
            ProgressIndicator::Spinner(spinner) => {
                spinner.setup_in_component(ctx);
            }
        }
    }
}

impl WithModifier for ProgressIndicator {
    fn modifier(self, modifier: Modifier) -> Self {
        match self {
            ProgressIndicator::Bar(bar) => ProgressIndicator::Bar(bar.modifier(modifier)),
            ProgressIndicator::Spinner(spinner) => {
                ProgressIndicator::Spinner(spinner.modifier(modifier))
            }
        }
    }
}

impl widgets::ProgressIndicator for ProgressIndicator {
    fn new_bar(value: impl Signal<Value = usize> + 'static) -> Self {
        new_bar_widget(value.map_value(|v| v as f64 / 100.0))
    }

    fn new_spinner() -> Self {
        new_spinner_widget()
    }
}
