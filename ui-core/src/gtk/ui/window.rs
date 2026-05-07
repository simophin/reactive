use crate::Prop;
use crate::widgets::{CommonWindow, Modifier, NativeView, NativeViewRegistry};
use glib::object::Cast;
use gtk4::prelude::GtkWindowExt;
use reactive_core::{Component, ComponentId, SetupContext};
use std::rc::Rc;

pub type Window = CommonWindow<gtk4::Widget>;

pub static PROP_TITLE: Prop<Window, gtk4::ApplicationWindow, String> =
    Prop::new(|w, title| w.set_title(Some(&title)));

struct WindowViewRegistry(gtk4::ApplicationWindow);

impl NativeViewRegistry<gtk4::Widget> for WindowViewRegistry {
    fn update_view(&self, _component_id: ComponentId, view: gtk4::Widget, modifier: Modifier) {
        self.0.set_child(Some(&view));
    }

    fn clear_view(&self, _component_id: ComponentId, view: gtk4::Widget) {
        if self.0.child().as_ref() == Some(&view) {
            self.0.set_child(None::<&gtk4::Widget>);
        }
    }
}

impl Component for Window {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let Self {
            title,
            child,
            initial_size: (initial_width, initial_height),
            ..
        } = *self;

        let app_window = NativeView::new(
            move |_| {
                let app = gtk4::gio::Application::default()
                    .and_then(|a| a.downcast::<gtk4::Application>().ok())
                    .expect("Window must be set up inside gtk::Application::activate");
                let window = gtk4::ApplicationWindow::new(&app);
                window.set_default_size(initial_width as i32, initial_height as i32);
                window.connect_close_request(|_| {
                    crate::gtk::stop_app();
                    glib::Propagation::Proceed
                });
                window.present();
                window
            },
            |w| w.upcast(),
            |_, _| {},
            Default::default(),
            &super::VIEW_REGISTRY_KEY,
        )
        .bind(PROP_TITLE, title)
        .setup_in_component(ctx);

        ctx.set_static_context(
            &super::VIEW_REGISTRY_KEY,
            Rc::new(WindowViewRegistry(app_window)),
        );

        ctx.boxed_child(child);
    }
}
