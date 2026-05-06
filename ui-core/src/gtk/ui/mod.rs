use crate::widgets::NativeViewRegistry;
use gtk4::Widget;
use reactive_core::ContextKey;
use std::rc::Rc;

pub mod button;
pub mod flex;
// mod gtk_view;
pub mod image_codec;
pub mod image_view;
pub mod label;
// pub mod list_view;
pub mod platform;
pub mod progress_indicator;
pub mod slider;
pub mod stack;
pub mod text_input;
pub mod window;

pub(crate) static VIEW_REGISTRY_KEY: ContextKey<Rc<dyn NativeViewRegistry<Widget>>> =
    ContextKey::new();
