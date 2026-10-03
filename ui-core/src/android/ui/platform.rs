use super::button::Button;
use super::flex::Flex;
use super::image::{BitmapCodec, Image};
use super::label::Label;
use super::unsupported::Unsupported;
use super::window::Window;
use crate::android::JavaObject;
use crate::widgets::{NativeViewRegistry, Platform};
use reactive_core::{ContextKey, SetupContext};
use std::rc::Rc;

pub struct Android;

impl Platform for Android {
    type NativeViewHandle = JavaObject;

    type ImageCodec = BitmapCodec;
    type Button = Button;
    type Label = Label;
    type Image = Image;
    type ProgressIndicator = Unsupported;
    type TextInput = Unsupported;
    type Slider = Unsupported;
    type Stack = Unsupported;
    type Window = Window;
    type Flex = Flex;

    fn native_view_registry_key()
    -> &'static ContextKey<Rc<dyn NativeViewRegistry<Self::NativeViewHandle>>> {
        &super::VIEW_REGISTRY_KEY
    }

    /// The Android main loop belongs to the activity: declare the entry point
    /// with [`android_main!`](crate::android_main) instead.
    fn run_app(_setup: impl FnOnce(&mut SetupContext) + 'static) {
        panic!("use ui_core::android_main! to start an Android app");
    }

    fn register_back_handler(_on_back: impl FnMut() -> bool + 'static) {}
}
