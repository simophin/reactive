use super::button::Button;
use super::flex::{Column, Row};
use super::slider::Slider;
use super::window::Window;
use crate::text_view::TextView;
use ui_core::widgets::Platform;

pub struct AppKit;

impl Platform for AppKit {
    type Button = Button;
    type Label = TextView;
    type TextInput = TextView;
    type Slider = Slider;
    type Row = Row;
    type Column = Column;
    type Window = Window;
}
