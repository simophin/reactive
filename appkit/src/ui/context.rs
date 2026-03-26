use objc2::rc::Retained;
use objc2_app_kit::NSView;
use reactive_core::ContextKey;

pub(super) static PARENT_VIEW: ContextKey<Retained<NSView>> = ContextKey::new();
