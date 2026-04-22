use objc2::rc::Retained;
use objc2_app_kit::NSView;
use reactive_core::ContextKey;
use std::rc::Rc;
use ui_core::widgets::NativeViewRegistry;

pub mod button;
pub mod checkbox;
pub mod collection_view;
pub mod flex;
pub mod image_codec;
pub mod image_view;
pub mod label;
pub mod native;
pub mod platform;
pub mod progress_indicator;
pub mod slider;
pub mod stack;
pub mod text_view;
pub mod window;

static VIEW_REGISTRY_KEY: ContextKey<Rc<dyn NativeViewRegistry<Retained<NSView>>>> =
    ContextKey::new();
