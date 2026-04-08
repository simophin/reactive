use ui_core::widgets::Column;
use ui_core::widgets::Row;
use reactive_core::{BoxedComponent, Component, SetupContext, Signal};
use crate::ui::view_component::{AndroidViewBuilder, AndroidViewComponent};
use crate::ui::flex_layout::AndroidFlexLayout;

pub struct AndroidFlex {
    vertical: bool,
    children: Vec<BoxedComponent>,
    spacing: Option<Box<dyn Signal<Value = usize>>>,
    cross_axis_alignment: ui_core::layout::CrossAxisAlignment,
}

impl AndroidFlex {
    fn new(vertical: bool) -> Self {
        Self {
            vertical,
            children: Vec::new(),
            spacing: None,
            cross_axis_alignment: ui_core::layout::CrossAxisAlignment::Start,
        }
    }
}

impl Row for AndroidFlex {
    fn new() -> Self { Self::new(false) }
    fn spacing(mut self, spacing: impl Signal<Value = usize> + 'static) -> Self {
        self.spacing = Some(Box::new(spacing));
        self
    }
    fn cross_axis_alignment(mut self, alignment: ui_core::layout::CrossAxisAlignment) -> Self {
        self.cross_axis_alignment = alignment;
        self
    }
    fn child(mut self, child: impl Component + 'static) -> Self {
        self.children.push(Box::new(child));
        self
    }
}

impl Column for AndroidFlex {
    fn new() -> Self { Self::new(true) }
    fn spacing(mut self, spacing: impl Signal<Value = usize> + 'static) -> Self {
        self.spacing = Some(Box::new(spacing));
        self
    }
    fn cross_axis_alignment(mut self, alignment: ui_core::layout::CrossAxisAlignment) -> Self {
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
        let AndroidFlex { vertical, children, spacing, cross_axis_alignment } = *self;

        let builder = children.into_iter().fold(
            AndroidViewBuilder::create_multiple_child(
                |_| {
                    // This is a simplified placeholder. In real impl,
                    // the AndroidFlexLayout is instantiated via dexer.
                    AndroidView::new(
                        // VM and GlobalRef provided by a factory
                        todo!("Need VM and GlobalRef for AndroidView"),
                        &todo!("Native AndroidView object")
                    )
                },
                |v| v,
            ),
            |builder, child| builder.add_child(child),
        );

        let container = builder.setup(ctx);

        if let Some(children_views) = ctx.use_context(&crate::ui::context::CHILDREN_VIEWS) {
            ctx.create_effect(move |_, _| {
                let spacing_val = spacing.as_ref().map_or(0.0, |s| s.read() as f32);
                let entries = children_views
                    .read()
                    .iter()
                    .filter_map(|slot| slot.read())
                    .collect::<Vec<_>>();

                // Call lauter setters on AndroidFlexLayout via JNI
                // container.set_vertical(vertical);
                // container.set_spacing(spacing_val);
                // container.update_children(entries);
            });
        }
    }
}
