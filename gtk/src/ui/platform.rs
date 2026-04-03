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
use reactive_core::SetupContext;
use ui_core::widgets::Platform;

pub struct Gtk;

impl Platform for Gtk {
    type ImageCodec = GtkImageCodec;
    type Button = Button;
    type Label = Label;
    type Image = ImageView;
    type ProgressIndicator = ProgressIndicator;
    type TextInput = GtkTextInputWidget;
    type Slider = Slider;
    type Row = Flex;
    type Column = Flex;
    type Stack = GtkStack;
    type Window = Window;
    type List = ListView;

    fn run_app(setup: impl FnOnce(&mut SetupContext) + 'static) {
        crate::run_app(setup);
    }
}
