use std::cell::RefCell;
use std::rc::Rc;

use objc2::rc::Retained;
use objc2_app_kit::NSView;
use objc2_foundation::MainThreadMarker;
use reactive_core::{BoxedComponent, Component, ContextKey, SetupContext};
use ui_utils::layout::{Alignment, CrossAxisAlignment, LayoutHints, LAYOUT_HINTS};

use super::context::PARENT_VIEW;

// ── Child view registry ────────────────────────────────────────────────────

/// Holds each direct child's view and the layout hints active when it was
/// created. Row/Column read this after all children are set up to apply
/// inter-child constraints.
#[derive(Clone)]
pub(super) struct ChildEntry {
    pub view: Retained<NSView>,
    pub hints: LayoutHints,
}

pub(super) type ChildViewRegistry = Rc<RefCell<Vec<ChildEntry>>>;
pub(super) static CHILD_VIEW_REGISTRY: ContextKey<ChildViewRegistry> = ContextKey::new();

// ── Constraint helpers ─────────────────────────────────────────────────────

/// Pin `view` to the edges of `parent`, offset by `hints.padding`.
/// If `hints.alignment` is `Center`, center instead of fill.
/// Respects `hints.fixed_width` / `hints.fixed_height`.
pub(super) fn activate_fill(view: &NSView, parent: &NSView, hints: &LayoutHints) {
    match hints.alignment {
        Some(Alignment::Center) => {
            view.centerXAnchor()
                .constraintEqualToAnchor(&parent.centerXAnchor())
                .setActive(true);
            view.centerYAnchor()
                .constraintEqualToAnchor(&parent.centerYAnchor())
                .setActive(true);
        }
        _ => {
            let p = &hints.padding;
            view.topAnchor()
                .constraintEqualToAnchor_constant(&parent.topAnchor(), p.top)
                .setActive(true);
            view.leadingAnchor()
                .constraintEqualToAnchor_constant(&parent.leadingAnchor(), p.left)
                .setActive(true);
            view.trailingAnchor()
                .constraintEqualToAnchor_constant(&parent.trailingAnchor(), -p.right)
                .setActive(true);
            view.bottomAnchor()
                .constraintEqualToAnchor_constant(&parent.bottomAnchor(), -p.bottom)
                .setActive(true);
        }
    }
    if let Some(w) = hints.fixed_width {
        view.widthAnchor().constraintEqualToConstant(w).setActive(true);
    }
    if let Some(h) = hints.fixed_height {
        view.heightAnchor().constraintEqualToConstant(h).setActive(true);
    }
}

fn apply_size_hints(view: &NSView, hints: &LayoutHints) {
    if let Some(w) = hints.fixed_width {
        view.widthAnchor().constraintEqualToConstant(w).setActive(true);
    }
    if let Some(h) = hints.fixed_height {
        view.heightAnchor().constraintEqualToConstant(h).setActive(true);
    }
}

fn apply_column_constraints(
    container: &NSView,
    entries: &[ChildEntry],
    spacing: f64,
    cross: CrossAxisAlignment,
) {
    if entries.is_empty() {
        return;
    }

    for (i, entry) in entries.iter().enumerate() {
        let view = &entry.view;
        let p = &entry.hints.padding;

        // Cross-axis (horizontal for Column)
        match cross {
            CrossAxisAlignment::Stretch => {
                view.leadingAnchor()
                    .constraintEqualToAnchor_constant(&container.leadingAnchor(), p.left)
                    .setActive(true);
                view.trailingAnchor()
                    .constraintEqualToAnchor_constant(&container.trailingAnchor(), -p.right)
                    .setActive(true);
            }
            CrossAxisAlignment::Start => {
                view.leadingAnchor()
                    .constraintEqualToAnchor_constant(&container.leadingAnchor(), p.left)
                    .setActive(true);
            }
            CrossAxisAlignment::Center => {
                view.centerXAnchor()
                    .constraintEqualToAnchor(&container.centerXAnchor())
                    .setActive(true);
            }
            CrossAxisAlignment::End => {
                view.trailingAnchor()
                    .constraintEqualToAnchor_constant(&container.trailingAnchor(), -p.right)
                    .setActive(true);
            }
        }

        // Main axis (vertical for Column)
        if i == 0 {
            view.topAnchor()
                .constraintEqualToAnchor_constant(&container.topAnchor(), p.top)
                .setActive(true);
        } else {
            let prev = &entries[i - 1];
            let gap = spacing + prev.hints.padding.bottom + p.top;
            view.topAnchor()
                .constraintEqualToAnchor_constant(&prev.view.bottomAnchor(), gap)
                .setActive(true);
        }

        apply_size_hints(view, &entry.hints);
    }

    // Pin last child's bottom to container's bottom
    let last = entries.last().unwrap();
    last.view
        .bottomAnchor()
        .constraintEqualToAnchor_constant(&container.bottomAnchor(), -last.hints.padding.bottom)
        .setActive(true);

    // Make all flex children equal height
    let flex: Vec<_> = entries.iter().filter(|e| e.hints.flex.is_some()).collect();
    if flex.len() > 1 {
        let first = &flex[0].view;
        for e in &flex[1..] {
            e.view
                .heightAnchor()
                .constraintEqualToAnchor(&first.heightAnchor())
                .setActive(true);
        }
    }
}

fn apply_row_constraints(
    container: &NSView,
    entries: &[ChildEntry],
    spacing: f64,
    cross: CrossAxisAlignment,
) {
    if entries.is_empty() {
        return;
    }

    for (i, entry) in entries.iter().enumerate() {
        let view = &entry.view;
        let p = &entry.hints.padding;

        // Cross-axis (vertical for Row)
        match cross {
            CrossAxisAlignment::Stretch => {
                view.topAnchor()
                    .constraintEqualToAnchor_constant(&container.topAnchor(), p.top)
                    .setActive(true);
                view.bottomAnchor()
                    .constraintEqualToAnchor_constant(&container.bottomAnchor(), -p.bottom)
                    .setActive(true);
            }
            CrossAxisAlignment::Start => {
                view.topAnchor()
                    .constraintEqualToAnchor_constant(&container.topAnchor(), p.top)
                    .setActive(true);
            }
            CrossAxisAlignment::Center => {
                view.centerYAnchor()
                    .constraintEqualToAnchor(&container.centerYAnchor())
                    .setActive(true);
            }
            CrossAxisAlignment::End => {
                view.bottomAnchor()
                    .constraintEqualToAnchor_constant(&container.bottomAnchor(), -p.bottom)
                    .setActive(true);
            }
        }

        // Main axis (horizontal for Row)
        if i == 0 {
            view.leadingAnchor()
                .constraintEqualToAnchor_constant(&container.leadingAnchor(), p.left)
                .setActive(true);
        } else {
            let prev = &entries[i - 1];
            let gap = spacing + prev.hints.padding.right + p.left;
            view.leadingAnchor()
                .constraintEqualToAnchor_constant(&prev.view.trailingAnchor(), gap)
                .setActive(true);
        }

        apply_size_hints(view, &entry.hints);
    }

    // Pin last child's trailing to container's trailing
    let last = entries.last().unwrap();
    last.view
        .trailingAnchor()
        .constraintEqualToAnchor_constant(&container.trailingAnchor(), -last.hints.padding.right)
        .setActive(true);

    // Make all flex children equal width
    let flex: Vec<_> = entries.iter().filter(|e| e.hints.flex.is_some()).collect();
    if flex.len() > 1 {
        let first = &flex[0].view;
        for e in &flex[1..] {
            e.view
                .widthAnchor()
                .constraintEqualToAnchor(&first.widthAnchor())
                .setActive(true);
        }
    }
}

// ── Shared setup helpers ───────────────────────────────────────────────────

fn attach_to_parent(ctx: &mut SetupContext, container: &Retained<NSView>, hints: &LayoutHints) {
    if let Some(parent_signal) = ctx.use_context(&PARENT_VIEW) {
        let parent = parent_signal.read();
        parent.addSubview(container);

        ctx.on_cleanup({
            let c = container.clone();
            move || c.removeFromSuperview()
        });

        if let Some(registry) = ctx.use_context(&CHILD_VIEW_REGISTRY) {
            registry
                .read()
                .borrow_mut()
                .push(ChildEntry { view: container.clone(), hints: *hints });
        } else {
            activate_fill(container, &parent, hints);
        }
    }
}

// ── Column ─────────────────────────────────────────────────────────────────

pub struct Column {
    pub spacing: f64,
    pub cross_axis_alignment: CrossAxisAlignment,
    children: Vec<BoxedComponent>,
}

impl Column {
    pub fn new() -> Self {
        Self {
            spacing: 0.0,
            cross_axis_alignment: CrossAxisAlignment::Stretch,
            children: Vec::new(),
        }
    }

    pub fn spacing(mut self, s: f64) -> Self {
        self.spacing = s;
        self
    }

    pub fn cross_axis_alignment(mut self, a: CrossAxisAlignment) -> Self {
        self.cross_axis_alignment = a;
        self
    }

    pub fn child(mut self, c: impl Component + 'static) -> Self {
        self.children.push(Box::new(c));
        self
    }
}

impl Default for Column {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for Column {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let Column { spacing, cross_axis_alignment, children } = *self;

        let mtm = MainThreadMarker::new().expect("must be on main thread");
        let container = NSView::new(mtm);
        container.setTranslatesAutoresizingMaskIntoConstraints(false);

        let hints = ctx.use_context(&LAYOUT_HINTS).map(|s| s.read()).unwrap_or_default();
        attach_to_parent(ctx, &container, &hints);

        ctx.provide_context(&PARENT_VIEW, container.clone());
        ctx.provide_context(&LAYOUT_HINTS, LayoutHints::default());
        let registry: ChildViewRegistry = Default::default();
        ctx.provide_context(&CHILD_VIEW_REGISTRY, registry.clone());

        for child in children {
            child.setup(&mut ctx.new_child());
        }

        apply_column_constraints(&container, &registry.borrow(), spacing, cross_axis_alignment);
    }
}

// ── Row ────────────────────────────────────────────────────────────────────

pub struct Row {
    pub spacing: f64,
    pub cross_axis_alignment: CrossAxisAlignment,
    children: Vec<BoxedComponent>,
}

impl Row {
    pub fn new() -> Self {
        Self {
            spacing: 0.0,
            cross_axis_alignment: CrossAxisAlignment::Stretch,
            children: Vec::new(),
        }
    }

    pub fn spacing(mut self, s: f64) -> Self {
        self.spacing = s;
        self
    }

    pub fn cross_axis_alignment(mut self, a: CrossAxisAlignment) -> Self {
        self.cross_axis_alignment = a;
        self
    }

    pub fn child(mut self, c: impl Component + 'static) -> Self {
        self.children.push(Box::new(c));
        self
    }
}

impl Default for Row {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for Row {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let Row { spacing, cross_axis_alignment, children } = *self;

        let mtm = MainThreadMarker::new().expect("must be on main thread");
        let container = NSView::new(mtm);
        container.setTranslatesAutoresizingMaskIntoConstraints(false);

        let hints = ctx.use_context(&LAYOUT_HINTS).map(|s| s.read()).unwrap_or_default();
        attach_to_parent(ctx, &container, &hints);

        ctx.provide_context(&PARENT_VIEW, container.clone());
        ctx.provide_context(&LAYOUT_HINTS, LayoutHints::default());
        let registry: ChildViewRegistry = Default::default();
        ctx.provide_context(&CHILD_VIEW_REGISTRY, registry.clone());

        for child in children {
            child.setup(&mut ctx.new_child());
        }

        apply_row_constraints(&container, &registry.borrow(), spacing, cross_axis_alignment);
    }
}
