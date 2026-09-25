use crate::widgets::{
    Modifier, NativeView, PlatformTextType, TextCommand, TextInput, WithModifier,
};
use futures::StreamExt;
use futures::channel::mpsc::Receiver;
use gtk4::prelude::*;
use reactive_core::{Component, SetupContext, Signal};
use std::cell::{Cell, RefCell};
use std::fmt;
use std::ops::Range;
use std::rc::Rc;

pub struct GtkTextInputWidget {
    rx: Option<Receiver<TextCommand<GtkTextType>>>,
    on_text_changed: Option<Box<dyn FnMut(&str)>>,
    on_selection_changed: Option<Box<dyn FnMut(Range<usize>)>>,
    font_size: Option<Box<dyn Signal<Value = f64>>>,
    modifier: Modifier,
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

impl Component for GtkTextInputWidget {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let Self {
            rx,
            on_text_changed,
            on_selection_changed,
            font_size,
            modifier,
        } = *self;

        let updating = Rc::new(Cell::new(false));
        let on_text_changed = Rc::new(RefCell::new(on_text_changed));
        let on_selection_changed = Rc::new(RefCell::new(on_selection_changed));

        let text_view = NativeView::new(
            {
                let updating = updating.clone();
                let on_text_changed = on_text_changed.clone();
                let on_selection_changed = on_selection_changed.clone();

                move |_| {
                    let text_view = gtk4::TextView::new();
                    let buffer = text_view.buffer();

                    buffer.connect_changed({
                        let updating = updating.clone();
                        let on_text_changed = on_text_changed.clone();
                        move |buffer| {
                            if updating.get() {
                                return;
                            }

                            let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), true);
                            if let Some(on_text_changed) = &mut *on_text_changed.borrow_mut() {
                                on_text_changed(text.as_str());
                            }
                        }
                    });

                    buffer.connect_mark_set({
                        let updating = updating.clone();
                        let on_selection_changed = on_selection_changed.clone();
                        let last_selection = Rc::new(Cell::new(None::<(usize, usize)>));

                        move |buffer, _, mark| {
                            if updating.get() {
                                return;
                            }

                            let Some(name) = mark.name() else {
                                return;
                            };
                            if name != "insert" && name != "selection_bound" {
                                return;
                            }

                            let selection = buffer
                                .selection_bounds()
                                .map(|(start, end)| {
                                    (start.offset() as usize, end.offset() as usize)
                                })
                                .unwrap_or_else(|| {
                                    let cursor = buffer.cursor_position() as usize;
                                    (cursor, cursor)
                                });

                            if last_selection.replace(Some(selection)) == Some(selection) {
                                return;
                            }

                            if let Some(on_selection_changed) =
                                &mut *on_selection_changed.borrow_mut()
                            {
                                on_selection_changed(selection.0..selection.1);
                            }
                        }
                    });

                    text_view
                }
            },
            |view| view.upcast(),
            |_, _| {},
            modifier,
            &super::VIEW_REGISTRY_KEY,
        )
        .setup_in_component(ctx);

        if let Some(rx) = rx {
            let mut rx = Some(rx);
            let command = ctx.create_stream(
                None::<GtkTextType>,
                || (),
                move |_| {
                    rx.take()
                        .expect("text command stream should only be created once")
                        .map(|command| match command {
                            TextCommand::SetText(text) => text,
                        })
                        .map(Some)
                },
            );
            let text_view = text_view.clone();
            let updating = updating.clone();
            ctx.create_effect(move |_, _| {
                let Some(text) = command.read() else {
                    return;
                };

                let buffer = text_view.buffer();
                let current = buffer.text(&buffer.start_iter(), &buffer.end_iter(), true);
                if current.as_str() != text.0 {
                    updating.set(true);
                    buffer.set_text(&text.0);
                    updating.set(false);
                }
            });
        }

        if let Some(font_size) = font_size {
            let provider = gtk4::CssProvider::new();
            text_view
                .style_context()
                .add_provider(&provider, gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION);
            ctx.create_effect(move |_, _| {
                provider.load_from_data(&format!(
                    "textview {{ font-size: {}pt; }}",
                    font_size.read()
                ));
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::GtkTextType;
    use crate::widgets::PlatformTextType;

    #[test]
    fn text_type_uses_codepoint_offsets() {
        let text = GtkTextType::from("aé🦀z");
        assert_eq!(text.len(), 4);
        assert_eq!(text.replace(1..3, &"X").0, "aXz");
    }

    #[test]
    fn text_type_supports_insertion_at_end() {
        let text = GtkTextType::from("hé");
        assert_eq!(text.replace(2..2, &"llo").0, "héllo");
    }
}

impl WithModifier for GtkTextInputWidget {
    fn modifier(mut self, modifier: Modifier) -> Self {
        self.modifier = modifier;
        self
    }
}

impl TextInput for GtkTextInputWidget {
    type PlatformTextType = GtkTextType;

    fn new() -> Self {
        Self {
            rx: None,
            on_text_changed: None,
            on_selection_changed: None,
            font_size: None,
            modifier: Modifier::default(),
        }
    }

    fn with_commander(mut self, rx: Receiver<TextCommand<Self::PlatformTextType>>) -> Self {
        self.rx = Some(rx);
        self
    }

    fn with_on_text_changed(
        mut self,
        on_change: impl FnMut(<Self::PlatformTextType as PlatformTextType>::RefType<'_>) + 'static,
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
