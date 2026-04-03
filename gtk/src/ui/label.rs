use crate::view_component::{GtkViewBuilder, GtkViewComponent, NoChildWidget};
use gtk4::prelude::*;
use reactive_core::{Signal, SignalExt};
use ui_core::Prop;
use ui_core::layout::types::TextAlignment;
use ui_core::widgets;

pub type Label = GtkViewComponent<gtk4::Label, NoChildWidget>;

pub static PROP_TEXT: &Prop<Label, gtk4::Label, String> =
    &Prop::new(|label, text| label.set_text(&text));

pub static PROP_JUSTIFY: &Prop<Label, gtk4::Label, gtk4::Justification> =
    &Prop::new(|label, j| label.set_justify(j));

pub static PROP_FONT_SIZE: &Prop<Label, gtk4::Label, f64> = &Prop::new(|label, size| {
    use gtk4::pango;
    let attrs = pango::AttrList::new();
    let size_pango = (size * pango::SCALE as f64) as i32;
    attrs.insert(pango::AttrSize::new(size_pango));
    label.set_attributes(Some(&attrs));
});

impl widgets::Label for Label {
    fn new(text: impl Signal<Value = String> + 'static) -> Self {
        Self(
            GtkViewBuilder::create_no_child(
                |_| {
                    let label = gtk4::Label::new(None);
                    label.set_wrap(true);
                    label.set_xalign(0.0);
                    label
                },
                |l| l.upcast(),
            )
            .bind(PROP_TEXT, text),
        )
    }

    fn font_size(self, size: impl Signal<Value = f64> + 'static) -> Self {
        Self(self.0.bind(PROP_FONT_SIZE, size))
    }

    fn alignment(self, alignment: impl Signal<Value = TextAlignment> + 'static) -> Self {
        Self(self.0.bind(
            PROP_JUSTIFY,
            alignment.map_value(|a| match a {
                TextAlignment::Leading => gtk4::Justification::Left,
                TextAlignment::Center => gtk4::Justification::Center,
                TextAlignment::Trailing => gtk4::Justification::Right,
            }),
        ))
    }
}
