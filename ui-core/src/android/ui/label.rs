use super::leaf;
use crate::Prop;
use crate::android::Node;
use crate::widgets::{self, NativeView, TextAlignment};
use reactive_core::{Signal, SignalExt};

pub type Label = NativeView<Node, Node>;

pub static PROP_TEXT: Prop<Label, Node, String> = Prop::new(|n, v| n.set("setText", v.as_str()));
pub static PROP_FONT_SIZE: Prop<Label, Node, f64> =
    Prop::new(|n, v| n.set("setFontSize", v as f32));
pub static PROP_ALIGN: Prop<Label, Node, i32> = Prop::new(|n, v| n.set("setAlign", v));

impl widgets::Label for Label {
    fn new(text: impl Signal<Value = String> + 'static) -> Self {
        leaf(|_| Node::new("com/reactive/TextNode")).bind(PROP_TEXT, text)
    }

    fn font_size(self, size: impl Signal<Value = f64> + 'static) -> Self {
        self.bind(PROP_FONT_SIZE, size)
    }

    fn alignment(self, alignment: impl Signal<Value = TextAlignment> + 'static) -> Self {
        self.bind(
            PROP_ALIGN,
            alignment.map_value(|a| match a {
                TextAlignment::Leading => 0,
                TextAlignment::Center => 1,
                TextAlignment::Trailing => 2,
            }),
        )
    }
}
