use crate::Prop;
use crate::widgets::{NativeView, PlatformTextType, TextCommand, TextInput};
use futures::channel::mpsc::Receiver;
use gtk4::prelude::*;
use reactive_core::Signal;
use std::cell::{Cell, RefCell};
use std::fmt;
use std::ops::Range;
use std::rc::Rc;

pub struct GtkTextInputWidget {
    rx: Receiver<TextCommand<GtkTextType>>,
    on_text_changed: Option<Box<dyn FnMut(&str)>>,
    on_selection_changed: Option<Box<dyn FnMut(Range<usize>)>>,
    font_size: Option<Box<dyn Signal<Value = f64>>>,
}

/// GTK's native text type: a UTF-8 `String` with codepoint-indexed offsets,
/// matching GTK4's `GtkTextIter` offset semantics.
#[derive(Clone, PartialEq, Eq)]
pub struct GtkTextType(pub String);

impl fmt::Display for GtkTextType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for GtkTextType {
    fn from(s: &str) -> Self {
        GtkTextType(s.to_owned())
    }
}

impl PlatformTextType for GtkTextType {
    type RefType<'a> = &'a str;

    fn len(&self) -> usize {
        self.0.chars().count()
    }

    fn replace(&self, range: Range<usize>, with: &Self::RefType<'_>) -> Self {
        let byte_start = self
            .0
            .char_indices()
            .nth(range.start)
            .map(|(i, _)| i)
            .unwrap_or(self.0.len());
        let byte_end = self
            .0
            .char_indices()
            .nth(range.end)
            .map(|(i, _)| i)
            .unwrap_or(self.0.len());
        let mut s = self.0.clone();
        s.replace_range(byte_start..byte_end, *with);
        GtkTextType(s)
    }

    fn as_str(&self) -> Option<&str> {
        Some(&self.0)
    }
}

pub static PROP_FONT_SIZE: Prop<GtkTextInputWidget, gtk4::TextView, f64> =
    Prop::new(|view, size| {
        use gtk4::prelude::*;
        let css = gtk4::CssProvider::new();
        css.load_from_data(&format!("textview {{ font-size: {size}pt; }}"));
        view.style_context()
            .add_provider(&css, gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION);
    });

impl TextInput for GtkTextInputWidget {
    type PlatformTextType = GtkTextType;

    fn new() -> Self {
        todo!()
    }

    fn with_commander(self, rx: Receiver<TextCommand<Self::PlatformTextType>>) -> Self {
        todo!()
    }

    fn with_on_text_changed(
        self,
        on_change: impl FnMut(<Self::PlatformTextType as PlatformTextType>::RefType<'_>) + 'static,
    ) -> Self {
        todo!()
    }

    fn with_on_selection_changed(
        self,
        on_selection_changed: impl FnMut(Range<usize>) + 'static,
    ) -> Self {
        todo!()
    }

    fn font_size(self, size: impl Signal<Value = f64> + 'static) -> Self {
        self.bind(PROP_FONT_SIZE, size)
    }
}
