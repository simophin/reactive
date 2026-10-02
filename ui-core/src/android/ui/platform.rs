use super::button::Button;
use super::flex::Flex;
use super::image::{AndroidImageCodec, ImageView};
use super::label::Label;
use super::progress_indicator::ProgressIndicator;
use super::slider::Slider;
use super::stack::Stack;
use super::text_input::TextInput;
use super::window::Window;
use crate::android::Node;
use crate::widgets::{NativeViewRegistry, Platform};
use reactive_core::{ContextKey, SetupContext};
use std::rc::Rc;

pub struct Android;

impl Platform for Android {
    type NativeViewHandle = Node;

    type ImageCodec = AndroidImageCodec;
    type Button = Button;
    type Label = Label;
    type Image = ImageView;
    type ProgressIndicator = ProgressIndicator;
    type TextInput = TextInput;
    type Slider = Slider;
    type Stack = Stack;
    type Window = Window;
    type Flex = Flex;

    fn native_view_registry_key()
    -> &'static ContextKey<Rc<dyn NativeViewRegistry<Self::NativeViewHandle>>> {
        &super::VIEW_REGISTRY_KEY
    }

    /// The Android main loop belongs to the activity, so the app is started from Kotlin with
    /// `ReactiveHost.create()`, and its setup is declared with [`android_main!`](crate::android_main).
    fn run_app(_setup: impl FnOnce(&mut SetupContext) + 'static) {
        panic!("on Android, declare the app with ui_core::android_main! instead of run_app");
    }

    fn register_back_handler(_on_back: impl FnMut() -> bool + 'static) {}
}
