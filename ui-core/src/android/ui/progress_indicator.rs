use super::leaf;
use crate::Prop;
use crate::android::Node;
use crate::widgets::{self, NativeView};
use jni::objects::JValue;
use reactive_core::{Signal, SignalExt};

pub type ProgressIndicator = NativeView<Node, Node>;

pub static PROP_PROGRESS: Prop<ProgressIndicator, Node, f32> =
    Prop::new(|n, v| n.set("setProgress", v));

fn progress_node(spinner: bool) -> Node {
    Node::new_with(
        "com/reactive/ProgressNode",
        "(Z)V",
        &[JValue::Bool(spinner as u8)],
    )
}

impl widgets::ProgressIndicator for ProgressIndicator {
    /// `value` is a percentage, 0 to 100.
    fn new_bar(value: impl Signal<Value = usize> + 'static) -> Self {
        leaf(|_| progress_node(false)).bind(PROP_PROGRESS, value.map_value(|v| v as f32 / 100.0))
    }

    fn new_spinner() -> Self {
        leaf(|_| progress_node(true))
    }
}
