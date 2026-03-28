use super::{Button, Column, Label, Row, Slider, Stack, TextInput, Window};
use crate::widgets::list::List;

pub trait Platform {
    type Button: Button;
    type Label: Label;
    type TextInput: TextInput;
    type Slider: Slider;
    type Row: Row;
    type Column: Column;
    type Stack: Stack;
    type Window: Window;
    type List: List;
}
