use crate::Prop;
use crate::widgets::{self, NativeView};
use objc2::rc::Retained;
use objc2::{MainThreadMarker, MainThreadOnly};
use objc2_ui_kit::{
    UIActivityIndicatorView, UIActivityIndicatorViewStyle, UIProgressView, UIProgressViewStyle,
    UIView,
};
use reactive_core::{Component, SetupContext, Signal, SignalExt};

/// A `UIProgressView` bar (value in percent, 0–100, as on GTK) or an animating
/// `UIActivityIndicatorView` spinner.
pub enum ProgressIndicator {
    Bar(NativeView<Retained<UIView>, Retained<UIProgressView>>),
    Spinner(NativeView<Retained<UIView>, Retained<UIActivityIndicatorView>>),
}

pub static PROP_PROGRESS: Prop<ProgressIndicator, Retained<UIProgressView>, f32> =
    Prop::new(|view, progress| view.setProgress(progress));

impl widgets::ProgressIndicator for ProgressIndicator {
    fn new_bar(value: impl Signal<Value = usize> + 'static) -> Self {
        Self::Bar(
            NativeView::from_parts(
                |_| {
                    let mtm = MainThreadMarker::new().expect("must be on main thread");
                    UIProgressView::initWithProgressViewStyle(
                        UIProgressView::alloc(mtm),
                        UIProgressViewStyle::Default,
                    )
                },
                |view| view.into_super(),
                |_, _| {},
                Default::default(),
                &super::VIEW_REGISTRY_KEY,
            )
            .bind(
                PROP_PROGRESS,
                value.map_value(|v| (v as f32 / 100.0).clamp(0.0, 1.0)),
            ),
        )
    }

    fn new_spinner() -> Self {
        Self::Spinner(NativeView::from_parts(
            |_| {
                let mtm = MainThreadMarker::new().expect("must be on main thread");
                let view = UIActivityIndicatorView::initWithActivityIndicatorStyle(
                    UIActivityIndicatorView::alloc(mtm),
                    UIActivityIndicatorViewStyle::Medium,
                );
                view.startAnimating();
                view
            },
            |view| view.into_super(),
            |_, _| {},
            Default::default(),
            &super::VIEW_REGISTRY_KEY,
        ))
    }
}

impl Component for ProgressIndicator {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        match *self {
            Self::Bar(bar) => {
                bar.setup_in_component(ctx);
            }
            Self::Spinner(spinner) => {
                spinner.setup_in_component(ctx);
            }
        }
    }
}
