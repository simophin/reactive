use crate::widgets::NativeViewRegistry;
use objc2::rc::Retained;
use objc2_ui_kit::{UIView, UIWindowScene};
use reactive_core::{ContextKey, SetupContext, Signal};
use std::rc::Rc;

pub mod button;
pub mod flex;
pub mod image;
pub mod label;
pub mod platform;
pub mod progress_indicator;
pub mod slider;
pub mod unsupported;
pub mod window;

pub(crate) static VIEW_REGISTRY_KEY: ContextKey<Rc<dyn NativeViewRegistry<Retained<UIView>>>> =
    ContextKey::new();

/// The `UIWindowScene` the component tree was set up for, provided at the
/// root of the tree when a scene connects.
pub static WINDOW_SCENE: ContextKey<Retained<UIWindowScene>> = ContextKey::new();

pub(crate) fn window_scene(ctx: &SetupContext) -> Retained<UIWindowScene> {
    ctx.use_context(&WINDOW_SCENE)
        .expect("component must be set up under a connected UIWindowScene")
        .read()
}
