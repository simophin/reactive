use super::{
    Button, Flex, Image, ImageCodec, Label, NativeViewRegistry, ProgressIndicator, Slider, Stack,
    TextInput, Window,
};
use crate::widgets::list::List;
use reactive_core::{ContextKey, SetupContext};
use std::rc::Rc;

pub trait Platform: 'static {
    type NativeViewHandle: Clone + 'static;

    type ImageCodec: ImageCodec;
    type Button: Button;
    type Label: Label;
    type Image: Image<NativeHandle = <Self::ImageCodec as ImageCodec>::NativeHandle>;
    type ProgressIndicator: ProgressIndicator;
    type TextInput: TextInput;
    type Slider: Slider;
    type Stack: Stack;
    type Window: Window;
    // type List: List;
    type Flex: Flex;

    fn native_view_registry_key()
    -> &'static ContextKey<Rc<dyn NativeViewRegistry<Self::NativeViewHandle>>>;

    /// Start the platform's main loop, call `setup` to build the component
    /// tree, then block until the application exits.
    fn run_app(setup: impl FnOnce(&mut SetupContext) + 'static);

    fn register_back_handler(on_back: impl FnMut() -> bool + 'static);
}
