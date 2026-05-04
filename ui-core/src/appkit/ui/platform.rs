use super::button::Button;
use super::image_codec::AppKitImageCodec;
use super::image_view::ImageView;
use super::label::Label;
use super::progress_indicator::ProgressIndicator;
use super::slider::Slider;
use super::stack::Stack as AppKitStack;
use super::window::Window;
use crate::appkit::collection_view::CollectionView;
use crate::appkit::flex::Flex;
use crate::appkit::text_view::TextView;
use crate::widgets::{NativeViewRegistry, Platform};
use objc2::rc::Retained;
use objc2_app_kit::NSView;
use reactive_core::{ContextKey, SetupContext};
use std::rc::Rc;

pub struct AppKit;

impl Platform for AppKit {
    type NativeViewHandle = Retained<NSView>;

    type ImageCodec = AppKitImageCodec;
    type Button = Button;
    type Label = Label;
    type Image = ImageView;
    type ProgressIndicator = ProgressIndicator;
    type TextInput = TextView;
    type Slider = Slider;
    type Stack = AppKitStack;
    type Window = Window;
    type List = CollectionView;
    type Flex = Flex;

    fn native_view_registry_key()
    -> &'static ContextKey<Rc<dyn NativeViewRegistry<Self::NativeViewHandle>>> {
        &super::VIEW_REGISTRY_KEY
    }

    fn run_app(setup: impl FnOnce(&mut SetupContext) + 'static) {
        crate::appkit::run_app(setup);
    }

    fn register_back_handler(_on_back: impl FnMut() -> bool + 'static) {
        todo!()
    }
}
