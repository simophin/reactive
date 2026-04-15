use super::{
    Button, Column, CustomLayoutOperation, Image, ImageCodec, Label, ProgressIndicator, Row,
    Slider, Stack, TextInput, Window,
};
use crate::widgets::list::List;
use crate::widgets::platform_view::{PlatformBaseView, PlatformContainerView};
use reactive_core::SetupContext;

pub trait Platform: 'static {
    type View: PlatformBaseView + Eq + Clone;
    type ContainerView: PlatformContainerView<BaseView = Self::View> + Eq + Clone;

    type ImageCodec: ImageCodec;
    type Button: Button;
    type Label: Label;
    type Image: Image<NativeHandle = <Self::ImageCodec as ImageCodec>::NativeHandle>;
    type ProgressIndicator: ProgressIndicator;
    type TextInput: TextInput;
    type Slider: Slider;
    type Row: Row;
    type Column: Column;
    type Stack: Stack;
    type Window: Window;
    type List: List;

    /// Start the platform's main loop, call `setup` to build the component
    /// tree, then block until the application exits.
    fn run_app(setup: impl FnOnce(&mut SetupContext) + 'static);

    fn register_back_handler(on_back: impl FnMut() -> bool + 'static);

    fn new_custom_layout(
        ops: impl CustomLayoutOperation<BaseView = Self::ContainerView> + 'static,
    ) -> Self::ContainerView;
}
