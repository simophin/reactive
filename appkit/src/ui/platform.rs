use super::button::Button;
use super::image_codec::AppKitImageCodec;
use super::image_view::ImageView;
use super::label::Label;
use super::progress_indicator::ProgressIndicator;
use super::slider::Slider;
use super::stack::Stack as AppKitStack;
use super::window::Window;
use crate::collection_view::CollectionView;
use crate::flex::Flex;
use crate::text_view::TextView;
use reactive_core::SetupContext;
use ui_core::widgets::Platform;

pub struct AppKit;

impl Platform for AppKit {
    type ImageCodec = AppKitImageCodec;
    type Button = Button;
    type Label = Label;
    type Image = ImageView;
    type ProgressIndicator = ProgressIndicator;
    type TextInput = TextView;
    type Slider = Slider;
    type Row = Flex;
    type Column = Flex;
    type Stack = AppKitStack;
    type Window = Window;
    type List = CollectionView;

    fn run_app(setup: impl FnOnce(&mut SetupContext) + 'static) {
        crate::run_app(setup);
    }

    fn register_back_handler(on_back: impl FnMut() -> bool + 'static) {
        todo!()
    }
}
