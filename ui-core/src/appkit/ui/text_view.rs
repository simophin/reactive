use crate::widgets::{Modifier, NativeView, PlatformTextType, TextCommand, WithModifier};
use derive_more::Display;
use futures::channel::mpsc::Receiver;
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::NSTextView;
use objc2_foundation::{NSMutableCopying, NSRange, NSString};
use reactive_core::{Component, ConstantSignal, IntoSignal, SetupContext, Signal};
use std::ops::Range;
use std::rc::Rc;

#[derive(Default)]
pub struct TextInput {
    on_text_changed: Option<Box<dyn FnMut(&NSString)>>,
    on_selection_changed: Option<Box<dyn FnMut(Range<usize>)>>,
    commander: Option<Receiver<TextCommand<AppkitString>>>,
    font_size: Option<Box<dyn Signal<Value = f64>>>,
    modifier: Modifier,
}

#[derive(Clone, Display, PartialEq, Eq, Hash)]
pub struct AppkitString(Retained<NSString>);

impl<'a> From<&'a str> for AppkitString {
    fn from(value: &'a str) -> Self {
        Self(NSString::from_str(value))
    }
}

unsafe impl Send for AppkitString {}
unsafe impl Sync for AppkitString {}

impl PlatformTextType for AppkitString {
    type RefType<'a> = &'a NSString;

    fn len(&self) -> usize {
        self.0.len()
    }

    fn replace(&self, range: Range<usize>, with: &Self::RefType<'_>) -> Self {
        let s = self.0.mutableCopy();
        s.replaceCharactersInRange_withString(NSRange::new(range.start, range.end), with);
        Self(s.into_super())
    }

    fn as_str(&self) -> Option<&str> {
        None
    }
}

impl Component for TextInput {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let Self {
            on_text_changed,
            on_selection_changed,
            commander,
            font_size,
            modifier,
        } = *self;

        let text_view = NativeView::new(
            |_| NSTextView::new(MainThreadMarker::new().unwrap()),
            |v| v.into_super().into_super(),
            |_, _| {},
            Default::default(),
            &super::VIEW_REGISTRY_KEY,
        )
        .setup_in_component(ctx);

        //TODO: hook up on_selection_changed & on_text_changed
    }
}

impl WithModifier for TextInput {
    fn modifier(mut self, modifier: Modifier) -> Self {
        self.modifier = modifier;
        self
    }
}

impl crate::widgets::TextInput for TextInput {
    type PlatformTextType = AppkitString;

    fn new() -> Self {
        Self::default()
    }

    fn with_commander(mut self, rx: Receiver<TextCommand<Self::PlatformTextType>>) -> Self {
        self.commander.replace(rx);
        self
    }

    fn with_on_text_changed(
        mut self,
        on_change: impl FnMut(<Self::PlatformTextType as PlatformTextType>::RefType<'_>) + 'static,
    ) -> Self {
        self.on_text_changed.replace(Box::new(on_change));
        self
    }

    fn with_on_selection_changed(
        mut self,
        on_selection_changed: impl FnMut(Range<usize>) + 'static,
    ) -> Self {
        self.on_selection_changed
            .replace(Box::new(on_selection_changed));
        self
    }

    fn font_size(mut self, size: impl Signal<Value = f64> + 'static) -> Self {
        self.font_size.replace(Box::new(size));
        self
    }
}
