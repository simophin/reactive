use super::context::CHILDREN_VIEWS;
use super::layout_view::ReactiveLayoutView;
use super::view_component::AppKitViewBuilder;
use objc2::rc::Retained;
use objc2_app_kit::NSView;
use objc2_foundation::MainThreadMarker;
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

fn rlv_into_nsview(v: Retained<ReactiveLayoutView>) -> Retained<NSView> {
    v.into_super()
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
            AppKitViewBuilder::create_multiple_child(
                move |_| {
                    let mtm = MainThreadMarker::new().expect("must be on main thread");
                    ReactiveLayoutView::new(mtm)
                },
                rlv_into_nsview,
            )
            .debug_identifier(if vertical { "Column" } else { "Row" }),
            |builder, child| builder.add_child(child),
        );

        let layout_view = builder.setup(ctx);

        if let Some(children_views) = ctx.use_context(&CHILDREN_VIEWS) {
            ctx.create_effect(move |_, _| {
                let spacing_val = spacing.as_ref().map_or(0.0, |s| s.read() as f32);
                let entries = children_views
                    .read()
                    .iter()
                    .filter_map(|slot| slot.read())
                    .collect::<Vec<_>>();

                layout_view.set_flex_params(vertical, spacing_val, cross_axis_alignment);
                layout_view.update_children(entries);
            });
        }
    }
}
