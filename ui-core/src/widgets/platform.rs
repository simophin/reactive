use super::{
    Button, Column, Image, Label, ProgressIndicator, Row, Slider, Stack, TextInput, Window,
};
use crate::widgets::list::List;

pub trait Platform {
    type Button: Button;
    type Label: Label;
    type Image: Image;
    type ProgressIndicator: ProgressIndicator;
    type TextInput: TextInput;
    type Slider: Slider;
    type Row: Row;
    type Column: Column;
    type Stack: Stack;
    type Window: Window;
    type List: List;
}
