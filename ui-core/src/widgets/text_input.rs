use reactive_core::{Component, Signal};
use std::fmt::Display;
use std::ops::Range;

#[derive(Clone, Eq, PartialEq)]
pub struct TextInputState<S> {
    pub text: S,
    pub selection: Range<usize>,
}

pub trait PlatformTextType: Display + for<'a> From<&'a str> + Send + Sync + Eq + 'static {
    fn len(&self) -> usize;
    fn replace(&self, range: Range<usize>, with: &Self) -> Self;
    fn as_str(&self) -> Option<&str>;
}

pub enum TextChange<S> {
    Replacement { replace: Range<usize>, with: S },
    SetSelection { selection: Range<usize> },
}

pub trait TextInput: Component + Sized + 'static {
    type PlatformTextType: PlatformTextType;

    fn new(
        value: impl Signal<Value = TextInputState<Self::PlatformTextType>> + 'static,
        on_change: impl Fn(TextChange<Self::PlatformTextType>) + 'static,
    ) -> Self;

    fn font_size(self, size: impl Signal<Value = f64> + 'static) -> Self;
}

impl<S> TextInputState<S> {
    pub fn apply_change(&mut self, c: &TextChange<S>)
    where
        S: PlatformTextType,
    {
        match c {
            TextChange::Replacement { replace, with } => {
                self.text = self.text.replace(replace.clone(), with);
            }
            TextChange::SetSelection { selection } => {
                self.selection = selection.clone();
            }
        }
    }
}
