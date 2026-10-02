use super::{ChildListRegistry, VIEW_REGISTRY_KEY, leaf};
use crate::android::Node;
use crate::widgets::{self, Alignment, NativeViewRegistry};
use reactive_core::{BoxedComponent, Component, SetupContext, Signal};
use std::rc::Rc;

pub struct Stack {
    children: Vec<BoxedComponent>,
    alignment: Option<Box<dyn Signal<Value = Alignment>>>,
}

impl widgets::Stack for Stack {
    fn new() -> Self {
        Self {
            children: Vec::new(),
            alignment: None,
        }
    }

    fn alignment(mut self, alignment: impl Signal<Value = Alignment> + 'static) -> Self {
        self.alignment = Some(Box::new(alignment));
        self
    }

    fn child(mut self, child: impl Component + 'static) -> Self {
        self.children.push(Box::new(child));
        self
    }
}

impl Component for Stack {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let Self {
            children,
            alignment,
        } = *self;

        let node = leaf(|_| Node::new("com/reactive/StackNode")).setup_in_component(ctx);

        if let Some(alignment) = alignment {
            let node = node.clone();
            ctx.create_effect(move |_, _| {
                let index = match alignment.read() {
                    Alignment::TopLeading => 0,
                    Alignment::Top => 1,
                    Alignment::TopTrailing => 2,
                    Alignment::Leading => 3,
                    Alignment::Center => 4,
                    Alignment::Trailing => 5,
                    Alignment::BottomLeading => 6,
                    Alignment::Bottom => 7,
                    Alignment::BottomTrailing => 8,
                };
                node.set("setAlignment", index);
            });
        }

        let registry: Rc<dyn NativeViewRegistry<Node>> = Rc::new(ChildListRegistry {
            scope: ctx.scope(),
            parent: node,
            children: Default::default(),
        });
        ctx.set_static_context(&VIEW_REGISTRY_KEY, registry);

        for child in children {
            ctx.boxed_child(child);
        }
    }
}
