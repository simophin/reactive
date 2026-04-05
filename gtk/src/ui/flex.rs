use super::constraint_host::ConstraintHost;
use super::context::CHILDREN_WIDGETS;
use super::view_component::GtkViewBuilder;
use gtk4::prelude::Cast;
use reactive_core::{BoxedComponent, Component, SetupContext, Signal};
use ui_core::layout::CrossAxisAlignment;
use ui_core::widgets::{Column, Row};

pub struct Flex {
    vertical: bool,
    children: Vec<BoxedComponent>,
    spacing: Option<Box<dyn Signal<Value = usize>>>,
    cross_axis_alignment: CrossAxisAlignment,
}

impl Flex {
    fn new(vertical: bool) -> Self {
        Self {
            vertical,
            children: Vec::new(),
            spacing: None,
            cross_axis_alignment: CrossAxisAlignment::Start,
        }
    }
}

impl Row for Flex {
    fn new() -> Self {
        Self::new(false)
    }
    fn spacing(mut self, spacing: impl Signal<Value = usize> + 'static) -> Self {
        self.spacing = Some(Box::new(spacing));
        self
    }
    fn cross_axis_alignment(mut self, alignment: CrossAxisAlignment) -> Self {
        self.cross_axis_alignment = alignment;
        self
    }
    fn child(mut self, child: impl Component + 'static) -> Self {
        self.children.push(Box::new(child));
        self
    }
}

impl Column for Flex {
    fn new() -> Self {
        Self::new(true)
    }
    fn spacing(mut self, spacing: impl Signal<Value = usize> + 'static) -> Self {
        self.spacing = Some(Box::new(spacing));
        self
    }
    fn cross_axis_alignment(mut self, alignment: CrossAxisAlignment) -> Self {
        self.cross_axis_alignment = alignment;
        self
    }
    fn child(mut self, child: impl Component + 'static) -> Self {
        self.children.push(Box::new(child));
        self
    }
}

fn constraint_host_into_widget(host: ConstraintHost) -> gtk4::Widget {
    host.upcast()
}

impl Component for Flex {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let Flex {
            vertical,
            children,
            spacing,
            cross_axis_alignment,
        } = *self;

        let builder = children.into_iter().fold(
            GtkViewBuilder::create_multiple_child(
                |_| ConstraintHost::new(),
                constraint_host_into_widget,
            ),
            |builder, child| builder.add_child(child),
        );

        let container = builder.setup(ctx);

        if let Some(children_widgets) = ctx.use_context(&CHILDREN_WIDGETS) {
            ctx.create_effect(move |_, _| {
                let spacing_val = spacing.as_ref().map_or(0.0, |s| s.read() as f32);
                let entries = children_widgets
                    .read()
                    .iter()
                    .filter_map(|slot| slot.read())
                    .collect::<Vec<_>>();

                container.set_flex_params(vertical, spacing_val, cross_axis_alignment);
                container.update_children(entries);
            });
        }
    }
}
