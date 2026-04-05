use crate::view_component::{GtkViewBuilder, GtkViewComponent, NoChildWidget};
use gtk4::prelude::*;
use reactive_core::Signal;
use std::cell::{Cell, RefCell};
use std::fmt;
use std::ops::Range;
use std::rc::Rc;
use ui_core::widgets::{PlatformTextType, TextChange, TextInput, TextInputState};
use ui_core::Prop;

pub type GtkTextInputWidget = GtkViewComponent<gtk4::TextView, NoChildWidget>;

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

pub static PROP_FONT_SIZE: &Prop<GtkTextInputWidget, gtk4::TextView, f64> =
    &Prop::new(|view, size| {
        use gtk4::prelude::*;
        let css = gtk4::CssProvider::new();
        css.load_from_data(&format!("textview {{ font-size: {size}pt; }}"));
        view.style_context()
            .add_provider(&css, gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION);
    });

impl TextInput for GtkTextInputWidget {
    type PlatformTextType = GtkTextType;

    fn new(
        value: impl Signal<Value = TextInputState<Self::PlatformTextType>> + 'static,
        on_change: impl for<'a> FnMut(TextChange<<Self::PlatformTextType as PlatformTextType>::RefType<'a>>)
            + 'static,
    ) -> Self {
        let on_change = Rc::new(RefCell::new(on_change));
        let on_change2 = Rc::clone(&on_change);

        // Guard flag: prevents re-entrant signal→buffer→callback loops.
        let updating = Rc::new(Cell::new(false));
        let updating2 = Rc::clone(&updating);
        let updating3 = Rc::clone(&updating);

        // Track previous char length so we can emit a full-replace TextChange.
        let prev_len = Rc::new(Cell::new(0usize));
        let prev_len2 = Rc::clone(&prev_len);

        Self(GtkViewBuilder::create_no_child(
            move |ctx| {
                let text_view = gtk4::TextView::new();
                text_view.set_editable(true);
                text_view.set_wrap_mode(gtk4::WrapMode::Word);

                let buffer = text_view.buffer();

                // Buffer-changed: emit TextChange::Replacement covering the entire content.
                buffer.connect_changed({
                    let on_change2 = Rc::clone(&on_change2);
                    let updating2 = Rc::clone(&updating2);
                    let prev_len2 = Rc::clone(&prev_len2);
                    move |buf| {
                        if updating2.get() {
                            return;
                        }
                        let text = buf
                            .text(&buf.start_iter(), &buf.end_iter(), false)
                            .to_string();
                        let old_len = prev_len2.get();
                        let new_len = text.chars().count();
                        prev_len2.set(new_len);
                        (on_change2.borrow_mut())(TextChange::Replacement {
                            replace: 0..old_len,
                            with: &text,
                        });
                    }
                });

                // Selection (cursor) change: emit TextChange::SetSelection.
                buffer.connect_mark_set({
                    let on_change3 = Rc::clone(&on_change);
                    let updating3 = Rc::clone(&updating3);
                    move |buf, iter, mark| {
                        if updating3.get() {
                            return;
                        }
                        if mark.name().as_deref() != Some("insert") {
                            return;
                        }
                        let start = iter.offset() as usize;
                        let end = start;
                        (on_change3.borrow_mut())(TextChange::SetSelection {
                            selection: start..end,
                        });
                        let _ = buf;
                    }
                });

                // Effect: keep the buffer in sync with the signal value.
                let buffer2 = text_view.buffer();
                ctx.create_effect(move |_, _| {
                    let state = value.read();
                    let current = buffer2
                        .text(&buffer2.start_iter(), &buffer2.end_iter(), false)
                        .to_string();
                    if current != state.text.0 {
                        updating.set(true);
                        buffer2.set_text(&state.text.0);
                        prev_len.set(state.text.0.chars().count());
                        updating.set(false);
                    }
                    let offset = state.selection.start as i32;
                    let iter = buffer2.iter_at_offset(offset);
                    buffer2.place_cursor(&iter);
                });

                text_view
            },
            |tv| tv.upcast(),
        ))
    }

    fn font_size(self, size: impl Signal<Value = f64> + 'static) -> Self {
        Self(self.0.bind(PROP_FONT_SIZE, size))
    }
}
