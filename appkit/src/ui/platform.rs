use super::button::Button;
use super::slider::Slider;
use super::window::Window;
use crate::collection_view::CollectionView;
use crate::flex::Flex;
use crate::text_view::TextView;
use ui_core::widgets::Platform;

pub struct AppKit;

impl Platform for AppKit {
    type Button = Button;
    type Label = TextView;
    type TextInput = TextView;
    type Slider = Slider;
    type Row = Flex;
    type Column = Flex;
    type Window = Window;
    type List = CollectionView;
}
