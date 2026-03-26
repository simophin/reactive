use super::{Button, Column, Label, Row, Slider, TextInput, Window};

pub trait Platform {
    type Button: Button;
    type Label: Label;
    type TextInput: TextInput;
    type Slider: Slider;
    type Row: Row;
    type Column: Column;
    type Window: Window;
}
