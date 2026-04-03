use super::context::CHILDREN_WIDGETS;
use super::context::ChildWidgetEntry;
use super::layout::apply_child_layout;
use super::view_component::GtkViewBuilder;
use gtk4::prelude::*;
use reactive_core::{BoxedComponent, Component, SetupContext, Signal, StoredSignal};
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

impl Component for Flex {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let Flex {
            vertical,
            children,
            spacing,
            cross_axis_alignment,
        } = *self;

        let orientation = if vertical {
            gtk4::Orientation::Vertical
        } else {
            gtk4::Orientation::Horizontal
        };

        let builder = children.into_iter().fold(
            GtkViewBuilder::create_multiple_child(
                move |_| gtk4::Box::new(orientation, 0),
                |b| b.upcast(),
            ),
            |builder, child| builder.add_child(child),
        );

        let gtk_box = builder.setup(ctx);

        if let Some(children_widgets) = ctx.use_context(&CHILDREN_WIDGETS) {
            ctx.create_effect(move |_, prev: Option<Vec<gtk4::Widget>>| {
                mount_flex_children(
                    &gtk_box,
                    vertical,
                    &spacing,
                    cross_axis_alignment,
                    children_widgets.read(),
                    prev,
                )
            });
        }
    }
}

fn mount_flex_children(
    container: &gtk4::Box,
    vertical: bool,
    spacing: &Option<Box<dyn Signal<Value = usize>>>,
    cross_axis: CrossAxisAlignment,
    child_signals: Vec<StoredSignal<Option<ChildWidgetEntry>>>,
    previous: Option<Vec<gtk4::Widget>>,
) -> Vec<gtk4::Widget> {
    // Remove previously mounted widgets.
    if let Some(prev) = previous {
        for w in &prev {
            container.remove(w);
        }
    }

    let spacing_px = spacing.as_ref().map_or(0, |s| s.read()) as i32;
    container.set_spacing(spacing_px);

    let mut mounted = Vec::new();

    for child_signal in &child_signals {
        if let Some(entry) = child_signal.read() {
            apply_child_layout(&entry.native, &entry.layout, vertical, cross_axis);
            container.append(&entry.native);
            mounted.push(entry.native.clone());
        }
    }

    mounted
}
