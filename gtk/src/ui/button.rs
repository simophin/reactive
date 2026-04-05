use crate::view_component::{GtkViewBuilder, GtkViewComponent, NoChildWidget};
use gtk4::prelude::*;
use reactive_core::Signal;
use ui_core::widgets;
use ui_core::Prop;

pub type Button = GtkViewComponent<gtk4::Button, NoChildWidget>;

pub static PROP_LABEL: &Prop<Button, gtk4::Button, String> =
    &Prop::new(|btn, text| btn.set_label(&text));

pub static PROP_SENSITIVE: &Prop<Button, gtk4::Button, bool> =
    &Prop::new(|btn, enabled| btn.set_sensitive(enabled));

impl widgets::Button for Button {
    fn new(title: impl Signal<Value = String> + 'static, on_click: impl Fn() + 'static) -> Self {
        Self(
            GtkViewBuilder::create_no_child(
                move |_| {
                    let button = gtk4::Button::new();
                    button.connect_clicked(move |_| on_click());
                    button
                },
                |b| b.upcast(),
            )
            .bind(PROP_LABEL, title),
        )
    }

    fn enabled(self, value: impl Signal<Value = bool> + 'static) -> Self {
        Self(self.0.bind(PROP_SENSITIVE, value))
    }
}
