use super::GtkViewComponent;
use glib::object::Cast;
use gtk4::prelude::{ButtonExt, WidgetExt};
use reactive_core::Signal;
use ui_core::Prop;
use ui_core::widgets;
use ui_core::widgets::NativeView;

pub type Button = GtkViewComponent<gtk4::Button>;

pub static PROP_LABEL: Prop<Button, gtk4::Button, String> =
    Prop::new(|btn, text| btn.set_label(&text));

pub static PROP_ENABLED: Prop<Button, gtk4::Button, bool> =
    Prop::new(|btn, enabled| btn.set_sensitive(enabled));

impl widgets::Button for Button {
    fn new(title: impl Signal<Value = String> + 'static, on_click: impl Fn() + 'static) -> Self {
        Self(
            NativeView::new(
                move |_| {
                    let button = gtk4::Button::new();
                    button.connect_clicked(move |_| on_click());
                    button
                },
                |b| b.upcast(),
                |_, _| {},
                Default::default(),
                &super::VIEW_REGISTRY_KEY,
            )
            .bind(PROP_LABEL, title),
        )
    }

    fn enabled(self, value: impl Signal<Value = bool> + 'static) -> Self {
        Self(self.0.bind(PROP_ENABLED, value))
    }
}
