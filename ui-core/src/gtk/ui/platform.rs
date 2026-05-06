use super::button::Button;
use super::flex::Flex;
use super::image_codec::GtkImageCodec;
use super::image_view::ImageView;
use super::label::Label;
use super::list_view::ListView;
use super::progress_indicator::ProgressIndicator;
use super::slider::Slider;
use super::stack::GtkStack;
use super::text_input::GtkTextInputWidget;
use super::window::Window;
use crate::widgets::{NativeViewRegistry, Platform};
use gtk4::Widget;
use gtk4::ffi::{GtkBox, GtkWidget};
use reactive_core::{ContextKey, SetupContext};
use std::rc::Rc;

pub struct Gtk;

impl Platform for Gtk {
    type NativeViewHandle = Widget;

    type ImageCodec = GtkImageCodec;
    type Button = Button;
    type Label = Label;
    type Image = ImageView;
    type ProgressIndicator = ProgressIndicator;
    type TextInput = GtkTextInputWidget;
    type Slider = Slider;
    type Stack = GtkStack;
    type Window = Window;
    type List = ListView;
    type Flex = Flex;

    fn native_view_registry_key()
    -> &'static ContextKey<Rc<dyn NativeViewRegistry<Self::NativeViewHandle>>> {
        &super::VIEW_REGISTRY_KEY
    }

    fn run_app(setup: impl FnOnce(&mut SetupContext) + 'static) {
        crate::gtk::run_app(setup);
    }

    fn register_back_handler(on_back: impl FnMut() -> bool + 'static) {}
}
