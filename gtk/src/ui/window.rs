use super::context::CHILDREN_WIDGETS;
use super::layout::apply_child_layout;
use super::view_component::GtkViewBuilder;
use gtk4::prelude::*;
use reactive_core::{BoxedComponent, Component, SetupContext, Signal};
use ui_core::Prop;
use ui_core::layout::CrossAxisAlignment;
use ui_core::widgets;

pub struct Window {
    child: BoxedComponent,
    title: Box<dyn Signal<Value = String>>,
    initial_width: f64,
    initial_height: f64,
}

pub static PROP_TITLE: &Prop<Window, gtk4::ApplicationWindow, String> =
    &Prop::new(|w, title| w.set_title(Some(&title)));

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

impl Component for Window {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let Self {
            title,
            child,
            initial_width,
            initial_height,
        } = *self;

        let app_window = GtkViewBuilder::create_with_child(
            move |_| {
                let app = gtk4::gio::Application::default()
                    .and_then(|a| a.downcast::<gtk4::Application>().ok())
                    .expect("Window must be set up inside gtk::Application::activate");
                let window = gtk4::ApplicationWindow::new(&app);
                window.set_default_size(initial_width as i32, initial_height as i32);
                window.connect_close_request(|_| {
                    crate::stop_app();
                    gtk4::glib::Propagation::Proceed
                });
                window.present();
                window
            },
            |w| w.upcast(),
            child,
        )
        .bind(PROP_TITLE, title)
        .setup(ctx);

        if let Some(children_widgets) = ctx.use_context(&CHILDREN_WIDGETS) {
            ctx.create_effect(move |_, _| {
                if let Some(entry) = children_widgets.read().first().and_then(|s| s.read()) {
                    apply_child_layout(
                        &entry.native,
                        &entry.layout,
                        true,
                        CrossAxisAlignment::Stretch,
                    );
                    app_window.set_child(Some(&entry.native));
                }
            });
        }
    }
}
