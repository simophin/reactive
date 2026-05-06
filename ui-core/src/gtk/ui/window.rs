use crate::Prop;
use crate::widgets;
use crate::widgets::{Modifier, NativeView, NativeViewRegistry};
use glib::object::Cast;
use gtk4::prelude::GtkWindowExt;
use reactive_core::{BoxedComponent, Component, ComponentId, SetupContext, Signal};
use std::rc::Rc;

pub struct Window {
    child: BoxedComponent,
    title: Box<dyn Signal<Value = String>>,
    initial_width: f64,
    initial_height: f64,
}

pub static PROP_TITLE: Prop<Window, gtk4::ApplicationWindow, String> =
    Prop::new(|w, title| w.set_title(Some(&title)));

impl widgets::Window for Window {
    fn new(
        title: impl Signal<Value = String> + 'static,
        child: impl Component + 'static,
        width: f64,
        height: f64,
    ) -> Self {
        Self {
            child: Box::new(child),
            title: Box::new(title),
            initial_width: width,
            initial_height: height,
        }
    }
}

struct WindowViewRegistry(gtk4::ApplicationWindow);

impl NativeViewRegistry<gtk4::Widget> for WindowViewRegistry {
    fn update_view(&self, component_id: ComponentId, view: gtk4::Widget, modifier: Modifier) {
        self.0.set_child(Some(&view));
    }

    fn clear_view(&self, component_id: ComponentId, view: gtk4::Widget) {
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
            initial_width,
            initial_height,
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
