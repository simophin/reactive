use super::button::Button;
use super::flex::Flex;
use super::image::{Image, UIImageCodec};
use super::label::Label;
use super::progress_indicator::ProgressIndicator;
use super::slider::Slider;
use super::unsupported::Unsupported;
use super::window::Window;
use crate::widgets::{NativeViewRegistry, Platform};
use objc2::rc::Retained;
use objc2_ui_kit::UIView;
use reactive_core::{ContextKey, SetupContext};
use std::rc::Rc;

pub struct UIKit;

impl Platform for UIKit {
    type NativeViewHandle = Retained<UIView>;

    type ImageCodec = UIImageCodec;
    type Button = Button;
    type Label = Label;
    type Image = Image;
    type ProgressIndicator = ProgressIndicator;
    type TextInput = Unsupported;
    type Slider = Slider;
    type Stack = Unsupported;
    type Window = Window;
    type Flex = Flex;

    fn native_view_registry_key()
    -> &'static ContextKey<Rc<dyn NativeViewRegistry<Self::NativeViewHandle>>> {
        &super::VIEW_REGISTRY_KEY
    }

    /// Calls `UIApplicationMain` and never returns; see
    /// [`crate::uikit::run_app`] for the lifecycle and its requirements.
    fn run_app(setup: impl FnOnce(&mut SetupContext) + 'static) {
        crate::uikit::run_app(setup)
    }

    /// iOS has no system back button; swipe-back belongs to navigation
    /// controllers, which this backend does not use yet.
    fn register_back_handler(_on_back: impl FnMut() -> bool + 'static) {}
}
