use super::context::ChildViewEntry;
use super::layout::{
    MountedChild, activate_constraints, compile_child_layout, deactivate_constraints,
    main_axis_dimension, root_cross_axis_constraints,
};
use super::view_component::AppKitViewBuilder;
use crate::context::CHILDREN_VIEWS;
use objc2::rc::Retained;
use objc2_app_kit::{
    NSLayoutConstraint, NSLayoutConstraintOrientation, NSLayoutPriorityDefaultLow,
    NSLayoutPriorityRequired, NSView,
};
use objc2_foundation::MainThreadMarker;
use reactive_core::{BoxedComponent, Component, SetupContext, Signal, StoredSignal};
use ui_core::layout::CrossAxisAlignment;

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
            spacing: None,
            children: Vec::new(),
            cross_axis_alignment: CrossAxisAlignment::Start,
        }
    }
}

impl ui_core::widgets::Row for Flex {
    fn new() -> Self {
        Self::new(false)
    }
    fn spacing(mut self, spacing: impl Signal<Value = usize> + 'static) -> Self {
        self.spacing.replace(Box::new(spacing));
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

impl ui_core::widgets::Column for Flex {
    fn new() -> Self {
        Self::new(true)
    }

    fn spacing(mut self, spacing: impl Signal<Value = usize> + 'static) -> Self {
        self.spacing.replace(Box::new(spacing));
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

        let builder = children.into_iter().fold(
            AppKitViewBuilder::create_multiple_child(
                |_| {
                    let mtm = MainThreadMarker::new().expect("must be on main thread");
                    NSView::new(mtm)
                },
                |view| view,
            )
            .debug_identifier(if vertical { "Column" } else { "Row" }),
            |builder, child| builder.add_child(child),
        );

        let my_view = builder.setup(ctx);

        if let Some(children_views) = ctx.use_context(&CHILDREN_VIEWS) {
            ctx.create_effect(move |_, prev_mounted| {
                mount_flex_children(
                    &my_view,
                    prev_mounted,
                    children_views.read(),
                    vertical,
                    &spacing,
                    cross_axis_alignment,
                )
            });
        }
    }
}

struct MountedFlexLayout {
    children: Vec<MountedChild>,
    shared_constraints: Vec<Retained<NSLayoutConstraint>>,
}

impl MountedFlexLayout {
    fn unmount(self, parent: &NSView) {
        deactivate_constraints(&self.shared_constraints);
        for child in self.children {
            child.unmount(parent);
        }
    }
}

fn mount_flex_children(
    parent: &Retained<NSView>,
    previous: Option<MountedFlexLayout>,
    child_views: Vec<StoredSignal<Option<ChildViewEntry>>>,
    vertical: bool,
    spacing: &Option<Box<dyn Signal<Value = usize>>>,
    cross_axis_alignment: CrossAxisAlignment,
) -> MountedFlexLayout {
    if let Some(previous) = previous {
        previous.unmount(&parent);
    }

    let spacing = spacing
        .as_ref()
        .map_or(0.0, |spacing| spacing.read() as f64);
    let entries: Vec<ChildViewEntry> = child_views
        .iter()
        .filter_map(|child_view| child_view.read())
        .filter(|c| unsafe { c.native.superview() }.is_none())
        .collect();

    if entries.is_empty() {
        return MountedFlexLayout {
            children: Vec::new(),
            shared_constraints: Vec::new(),
        };
    }

    let has_flex = entries.iter().any(|entry| entry.layout.flex.flex.is_some());
    let flex_indices = entries
        .iter()
        .enumerate()
        .filter_map(|(index, entry)| entry.layout.flex.flex.map(|flex| (index, flex.get())))
        .collect::<Vec<_>>();
    let children = entries
        .into_iter()
        .map(|entry| {
            configure_main_axis_sizing(&entry.native, vertical, entry.layout.flex.flex.is_some());
            parent.addSubview(&entry.native);
            compile_child_layout(&parent, entry.native, &entry.layout.box_modifiers)
        })
        .collect::<Vec<_>>();

    for (index, child) in children.iter().enumerate() {
        let identifier = if vertical {
            format!("column_slot_{index}")
        } else {
            format!("row_slot_{index}")
        };
        child.set_root_identifier(&identifier);
        child.activate();
    }

    let mut constraints = Vec::new();

    for (index, child) in children.iter().enumerate() {
        constraints.extend(root_cross_axis_constraints(
            &parent,
            child.root(),
            vertical,
            cross_axis_alignment,
        ));

        if index == 0 {
            constraints.push(if vertical {
                child
                    .root()
                    .topAnchor()
                    .constraintEqualToAnchor(&parent.topAnchor())
            } else {
                child
                    .root()
                    .leadingAnchor()
                    .constraintEqualToAnchor(&parent.leadingAnchor())
            });
        } else {
            constraints.push(if vertical {
                child.root().topAnchor().constraintEqualToAnchor_constant(
                    &children[index - 1].root().bottomAnchor(),
                    spacing,
                )
            } else {
                child
                    .root()
                    .leadingAnchor()
                    .constraintEqualToAnchor_constant(
                        &children[index - 1].root().trailingAnchor(),
                        spacing,
                    )
            });
        }

        if index == children.len() - 1 {
            constraints.push(if vertical {
                if has_flex {
                    child
                        .root()
                        .bottomAnchor()
                        .constraintEqualToAnchor(&parent.bottomAnchor())
                } else {
                    child
                        .root()
                        .bottomAnchor()
                        .constraintLessThanOrEqualToAnchor(&parent.bottomAnchor())
                }
            } else if has_flex {
                child
                    .root()
                    .trailingAnchor()
                    .constraintEqualToAnchor(&parent.trailingAnchor())
            } else {
                child
                    .root()
                    .trailingAnchor()
                    .constraintLessThanOrEqualToAnchor(&parent.trailingAnchor())
            });
        }
    }

    if let Some((base_index, base_flex)) = flex_indices.first().copied() {
        for (index, flex) in flex_indices.into_iter().skip(1) {
            constraints.push(
                main_axis_dimension(children[index].root(), vertical)
                    .constraintEqualToAnchor_multiplier(
                        &main_axis_dimension(children[base_index].root(), vertical),
                        flex as f64 / base_flex as f64,
                    ),
            );
        }
    }

    activate_constraints(&constraints);

    MountedFlexLayout {
        children,
        shared_constraints: constraints,
    }
}

fn configure_main_axis_sizing(view: &NSView, vertical: bool, is_flex: bool) {
    let orientation = if vertical {
        NSLayoutConstraintOrientation::Vertical
    } else {
        NSLayoutConstraintOrientation::Horizontal
    };
    let priority = if is_flex {
        NSLayoutPriorityDefaultLow
    } else {
        NSLayoutPriorityRequired
    };

    view.setContentHuggingPriority_forOrientation(priority, orientation);
    view.setContentCompressionResistancePriority_forOrientation(priority, orientation);
}
