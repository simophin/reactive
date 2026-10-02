use super::VIEW_REGISTRY_KEY;
use crate::Prop;
use crate::android::{Callback, Node};
use crate::encoding::Utf16String;
use crate::widgets::{self, Modifier, NativeView, PlatformTextType, TextCommand, WithModifier};
use futures::StreamExt;
use futures::channel::mpsc::Receiver;
use jni::objects::JString;
use reactive_core::{Component, SetupContext, Signal};
use std::ops::Range;

pub struct TextInput {
    rx: Option<Receiver<TextCommand<Utf16String>>>,
    on_text_changed: Option<Box<dyn FnMut(&str)>>,
    on_selection_changed: Option<Box<dyn FnMut(Range<usize>)>>,
    font_size: Option<Box<dyn Signal<Value = f64>>>,
    modifier: Modifier,
}

pub static PROP_FONT_SIZE: Prop<TextInput, Node, f64> =
    Prop::new(|n, v| n.set("setFontSize", v as f32));

impl Component for TextInput {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let Self {
            rx,
            mut on_text_changed,
            mut on_selection_changed,
            font_size,
            modifier,
        } = *self;

        let view = NativeView::new(
            move |ctx| {
                let node = Node::new("com/reactive/TextFieldNode");
                // `selection` packs UTF-16 `start << 32 | end`; `text` is null when unchanged.
                Callback::attach(ctx, &node, "setOnChange", move |env, selection, text| {
                    if !text.is_null()
                        && let Some(on_text_changed) = &mut on_text_changed
                    {
                        let text: String = env
                            .get_string(&JString::from(text))
                            .expect("read text field text")
                            .into();
                        on_text_changed(&text);
                    }
                    if let Some(on_selection_changed) = &mut on_selection_changed {
                        let start = (selection >> 32) as usize;
                        let end = (selection & 0xffff_ffff) as usize;
                        on_selection_changed(start.min(end)..start.max(end));
                    }
                    0
                });
                node
            },
            |node| node,
            |_, _| {},
            modifier,
            &VIEW_REGISTRY_KEY,
        );
        let view = match font_size {
            Some(font_size) => view.bind(PROP_FONT_SIZE, font_size),
            None => view,
        };
        let node = view.setup_in_component(ctx);

        if let Some(rx) = rx {
            let mut rx = Some(rx);
            let command = ctx.create_stream(
                None::<Utf16String>,
                || (),
                move |_| {
                    rx.take()
                        .expect("text command stream should only be created once")
                        .map(|TextCommand::SetText(text)| Some(text))
                },
            );
            ctx.create_effect(move |_, _| {
                if let Some(text) = command.read() {
                    node.set("setText", text.0.as_str());
                }
            });
        }
    }
}

impl WithModifier for TextInput {
    fn modifier(mut self, modifier: Modifier) -> Self {
        self.modifier = modifier;
        self
    }
}

impl widgets::TextInput for TextInput {
    type PlatformTextType = Utf16String;

    fn new() -> Self {
        Self {
            rx: None,
            on_text_changed: None,
            on_selection_changed: None,
            font_size: None,
            modifier: Modifier::default(),
        }
    }

    fn with_commander(mut self, rx: Receiver<TextCommand<Utf16String>>) -> Self {
        self.rx = Some(rx);
        self
    }

    fn with_on_text_changed(
        mut self,
        on_change: impl FnMut(<Utf16String as PlatformTextType>::RefType<'_>) + 'static,
    ) -> Self {
        self.on_text_changed = Some(Box::new(on_change));
        self
    }

    fn with_on_selection_changed(
        mut self,
        on_selection_changed: impl FnMut(Range<usize>) + 'static,
    ) -> Self {
        self.on_selection_changed = Some(Box::new(on_selection_changed));
        self
    }

    fn font_size(mut self, size: impl Signal<Value = f64> + 'static) -> Self {
        self.font_size = Some(Box::new(size));
        self
    }
}
