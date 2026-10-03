//! Stand-in for widgets the UIKit backend does not implement yet. Renders an
//! empty `UIView` so layouts stay intact, and logs a warning.

use crate::widgets::{
    Alignment, Modifier, NativeView, PlatformTextType, Stack, TextCommand, TextInput, WithModifier,
};
use futures::channel::mpsc;
use objc2::MainThreadMarker;
use objc2_ui_kit::UIView;
use reactive_core::{Component, SetupContext, Signal};
use std::fmt;
use std::ops::Range;

pub struct Unsupported {
    widget: &'static str,
    modifier: Modifier,
}

impl Unsupported {
    fn new(widget: &'static str) -> Self {
        Self {
            widget,
            modifier: Modifier::default(),
        }
    }
}

impl WithModifier for Unsupported {
    fn modifier(mut self, modifier: Modifier) -> Self {
        self.modifier = modifier;
        self
    }
}

impl Component for Unsupported {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        tracing::warn!("{} is not implemented on UIKit yet", self.widget);
        NativeView::from_parts(
            |_| UIView::new(MainThreadMarker::new().expect("must be on main thread")),
            |view| view,
            |_, _| {},
            self.modifier,
            &super::VIEW_REGISTRY_KEY,
        )
        .setup_in_component(ctx);
    }
}

impl Stack for Unsupported {
    fn new() -> Self {
        Self::new("Stack")
    }

    fn alignment(self, _alignment: impl Signal<Value = Alignment> + 'static) -> Self {
        self
    }

    fn child(self, _child: impl Component + 'static) -> Self {
        self
    }
}

impl TextInput for Unsupported {
    type PlatformTextType = UnsupportedText;

    fn new() -> Self {
        Self::new("TextInput")
    }

    fn with_commander(self, _rx: mpsc::Receiver<TextCommand<UnsupportedText>>) -> Self {
        self
    }

    fn with_on_text_changed(self, _on_change: impl FnMut(&str) + 'static) -> Self {
        self
    }

    fn with_on_selection_changed(self, _on_change: impl FnMut(Range<usize>) + 'static) -> Self {
        self
    }

    fn font_size(self, _size: impl Signal<Value = f64> + 'static) -> Self {
        self
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct UnsupportedText(String);

impl fmt::Display for UnsupportedText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for UnsupportedText {
    fn from(s: &str) -> Self {
        Self(s.to_owned())
    }
}

impl PlatformTextType for UnsupportedText {
    type RefType<'a> = &'a str;

    fn len(&self) -> usize {
        self.0.len()
    }

    fn replace(&self, range: Range<usize>, with: &&str) -> Self {
        let mut s = self.0.clone();
        s.replace_range(range, with);
        Self(s)
    }

    fn as_str(&self) -> Option<&str> {
        Some(&self.0)
    }
}
