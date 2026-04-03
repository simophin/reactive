use super::context::CHILDREN_WIDGETS;
use super::layout::apply_child_layout;
use super::view_component::GtkViewBuilder;
use gtk4::prelude::*;
use reactive_core::{BoxedComponent, Component, SetupContext, Signal};
use ui_core::layout::CrossAxisAlignment;
use ui_core::layout::types::Alignment;
use ui_core::widgets::Stack;

pub struct GtkStack {
    children: Vec<BoxedComponent>,
    alignment: Option<Box<dyn Signal<Value = Alignment>>>,
}

impl Stack for GtkStack {
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

impl Component for GtkStack {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let GtkStack {
            children,
            alignment,
        } = *self;

        let builder = children.into_iter().fold(
            GtkViewBuilder::create_multiple_child(|_| gtk4::Overlay::new(), |o| o.upcast()),
            |builder, child| builder.add_child(child),
        );

        let overlay = builder.setup(ctx);

        if let Some(children_widgets) = ctx.use_context(&CHILDREN_WIDGETS) {
            ctx.create_effect(move |_, prev: Option<Vec<gtk4::Widget>>| {
                // Remove previously overlaid widgets.
                if let Some(prev) = prev {
                    for w in &prev {
                        overlay.remove_overlay(w);
                    }
                }

                let entries: Vec<_> = children_widgets
                    .read()
                    .into_iter()
                    .filter_map(|s| s.read())
                    .collect();

                let mut mounted = Vec::new();

                for (index, entry) in entries.iter().enumerate() {
                    // First child is the base; rest are overlays.
                    let align = alignment.as_ref().map(|a| a.read()).unwrap_or_default();
                    apply_child_layout(
                        &entry.native,
                        &entry.layout,
                        true,
                        CrossAxisAlignment::Stretch,
                    );
                    if index == 0 {
                        overlay.set_child(Some(&entry.native));
                    } else {
                        // Align the overlay child according to the alignment signal.
                        use super::layout::apply_child_layout as _apply;
                        let mut synthetic = entry.layout.clone();
                        use ui_core::layout::types::{BoxModifier, BoxModifierChain};
                        synthetic.box_modifiers = BoxModifierChain {
                            modifiers: vec![BoxModifier::Align(align)],
                        };
                        _apply(&entry.native, &synthetic, true, CrossAxisAlignment::Stretch);
                        overlay.add_overlay(&entry.native);
                    }
                    mounted.push(entry.native.clone());
                }

                mounted
            });
        }
    }
}
