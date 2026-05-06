use crate::Prop;
use crate::widgets;
use crate::widgets::{NativeView, TextAlignment};
use glib::object::Cast;
use gtk4::pango;
use gtk4::prelude::WidgetExt;
use reactive_core::{Signal, SignalExt};

pub type Label = NativeView<gtk4::Widget, gtk4::Label>;

pub static PROP_TEXT: Prop<Label, gtk4::Label, String> =
    Prop::new(|label, text| label.set_text(&text));

pub static PROP_HALIGN: Prop<Label, gtk4::Label, gtk4::Align> =
    Prop::new(|label, value| label.set_halign(value));

pub static PROP_XALIGN: Prop<Label, gtk4::Label, f32> =
    Prop::new(|label, value| label.set_xalign(value));

pub static PROP_FONT_SIZE: Prop<Label, gtk4::Label, f64> = Prop::new(|label, size| {
    let attrs = pango::AttrList::new();
    let size_pango = (size * pango::SCALE as f64) as i32;
    attrs.insert(pango::AttrSize::new(size_pango));
    label.set_attributes(Some(&attrs));
});

impl widgets::Label for Label {
    fn new(text: impl Signal<Value = String> + 'static) -> Self {
        NativeView::new(
            |_| {
                let label = gtk4::Label::new(None);
                label.set_wrap(true);
                label.set_xalign(0.0);
                label.set_yalign(0.0);
                label
            },
            |l| l.upcast(),
            |_, _| {},
            Default::default(),
            &super::VIEW_REGISTRY_KEY,
        )
        .bind(PROP_TEXT, text)
    }

    fn font_size(self, size: impl Signal<Value = f64> + 'static) -> Self {
        self.bind(PROP_FONT_SIZE, size)
    }

    fn alignment(self, alignment: impl Signal<Value = TextAlignment> + 'static) -> Self {
        self.bind(
            PROP_XALIGN,
            alignment.map_value(|a| match a {
                TextAlignment::Leading => 0.0,
                TextAlignment::Center => 0.5,
                TextAlignment::Trailing => 1.0,
            }),
        )
    }
}
