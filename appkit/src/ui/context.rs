use objc2::rc::Retained;
use objc2_app_kit::NSView;
use reactive_core::{ContextKey, Signal};
use std::rc::Rc;
use ui_core::layout::LayoutHints;

pub(crate) static CHILD_ADDER: ContextKey<
    Rc<dyn Fn(Retained<NSView>, Rc<dyn Signal<Value = LayoutHints>>)>,
> = ContextKey::new();
