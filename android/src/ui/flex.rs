use reactive_core::{BoxedComponent, Component, SetupContext, Signal};
use ui_core::layout::CrossAxisAlignment;
use ui_core::widgets::{Column, Row};

use crate::ui::flex_layout::AndroidFlexLayout;
use crate::ui::view_component::{AndroidViewBuilder, CHILDREN_VIEWS};

pub struct AndroidFlex {
    vertical: bool,
    children: Vec<BoxedComponent>,
    spacing: Option<Box<dyn Signal<Value = usize>>>,
    cross_axis_alignment: CrossAxisAlignment,
}

impl AndroidFlex {
    fn new(vertical: bool) -> Self {
        Self {
            vertical,
            children: Vec::new(),
            spacing: None,
            cross_axis_alignment: CrossAxisAlignment::Start,
        }
    }
}

impl Row for AndroidFlex {
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

impl Column for AndroidFlex {
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

impl Component for AndroidFlex {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let AndroidFlex {
            vertical,
            children,
            spacing,
            cross_axis_alignment,
        } = *self;

        let builder = children.into_iter().fold(
            AndroidViewBuilder::create_multiple_child(
                move |ctx| AndroidFlexLayout::new(ctx),
                |layout| layout.as_view(),
            ),
            |builder, child| builder.add_child(child),
        );

        let layout = builder.setup(ctx);

        if let Some(children_views) = ctx.use_context(&CHILDREN_VIEWS) {
            ctx.create_effect(move |_, _| {
                let spacing = spacing
                    .as_ref()
                    .map(|value| value.read())
                    .unwrap_or_default() as f32;
                let entries = children_views
                    .read()
                    .iter()
                    .filter_map(|slot| slot.read())
                    .collect::<Vec<_>>();

                layout.set_flex_params(vertical, spacing, cross_axis_alignment);
                layout.update_children(entries);
            });
        }
    }
}
