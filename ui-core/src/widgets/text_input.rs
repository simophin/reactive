use crate::widgets::WithModifier;
use futures::channel::mpsc;
use reactive_core::{Component, Signal};
use std::fmt::Display;
use std::ops::Range;

pub enum TextCommand<T> {
    SetText(T),
}

pub trait PlatformTextType: Display + for<'a> From<&'a str> + Send + Sync + Eq + 'static {
    type RefType<'a>;

    fn len(&self) -> usize;
    fn replace(&self, range: Range<usize>, with: &Self::RefType<'_>) -> Self;
    fn as_str(&self) -> Option<&str>;
}

pub trait TextInput: Component + WithModifier + Sized + 'static {
    type PlatformTextType: PlatformTextType;

    fn new() -> Self;

    fn with_commander(self, rx: mpsc::Receiver<TextCommand<Self::PlatformTextType>>) -> Self;

    fn with_on_text_changed(
        self,
        on_change: impl FnMut(<Self::PlatformTextType as PlatformTextType>::RefType<'_>) + 'static,
    ) -> Self;
    fn with_on_selection_changed(
        self,
        on_selection_changed: impl FnMut(Range<usize>) + 'static,
    ) -> Self;

    fn font_size(self, size: impl Signal<Value = f64> + 'static) -> Self;
}
