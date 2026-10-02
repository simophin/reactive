use super::leaf;
use crate::Prop;
use crate::android::{Callback, Node};
use crate::widgets::{self, NativeView};
use reactive_core::Signal;
use std::ops::Range;

pub type Slider = NativeView<Node, Node>;

pub static PROP_VALUE: Prop<Slider, Node, usize> = Prop::new(|n, v| n.set("setValue", v as f32));
pub static PROP_RANGE: Prop<Slider, Node, Range<usize>> = Prop::new(|n, range| {
    n.set("setMin", range.start as f32);
    n.set("setMax", range.end as f32);
});

impl widgets::Slider for Slider {
    fn new(
        value: impl Signal<Value = usize> + 'static,
        range: impl Signal<Value = Range<usize>> + 'static,
        on_change: impl Fn(usize) + 'static,
    ) -> Self {
        leaf(move |ctx| {
            let node = Node::new("com/reactive/SliderNode");
            Callback::attach(ctx, &node, "setOnChange", move |_, value, _| {
                on_change(value.max(0) as usize);
                0
            });
            node
        })
        .bind(PROP_RANGE, range)
        .bind(PROP_VALUE, value)
    }
}
