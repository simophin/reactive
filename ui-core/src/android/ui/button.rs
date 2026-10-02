use super::leaf;
use crate::Prop;
use crate::android::{Callback, Node};
use crate::widgets::{self, NativeView};
use reactive_core::Signal;

pub type Button = NativeView<Node, Node>;

pub static PROP_TITLE: Prop<Button, Node, String> = Prop::new(|n, v| n.set("setTitle", v.as_str()));
pub static PROP_ENABLED: Prop<Button, Node, bool> = Prop::new(|n, v| n.set("setEnabled", v));

impl widgets::Button for Button {
    fn new(title: impl Signal<Value = String> + 'static, on_click: impl Fn() + 'static) -> Self {
        leaf(move |ctx| {
            let node = Node::new("com/reactive/ButtonNode");
            Callback::attach(ctx, &node, "setOnClick", move |_, _, _| {
                on_click();
                0
            });
            node
        })
        .bind(PROP_TITLE, title)
    }

    fn enabled(self, value: impl Signal<Value = bool> + 'static) -> Self {
        self.bind(PROP_ENABLED, value)
    }
}
