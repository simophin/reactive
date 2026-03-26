use std::cell::RefCell;
use std::rc::Rc;

use objc2::rc::Retained;
use objc2_app_kit::NSView;
use objc2_foundation::MainThreadMarker;
use reactive_core::{BoxedComponent, Component, ContextKey, IntoSignal, SetupContext, Signal};
use ui_core::layout::{Alignment, CrossAxisAlignment, LayoutHints, LAYOUT_HINTS};

use super::context::{PARENT_VIEW, ViewParent};

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

/// Pin `view` to `parent` according to `hints.alignment` and `hints.padding`.
/// - `None` alignment: fill the parent with padding offsets on all edges.
/// - Specific alignment: anchor the relevant edges/centers, leave size to intrinsic content.
/// Always applies `hints.fixed_width` / `hints.fixed_height` if set.
pub(super) fn activate_fill(view: &NSView, parent: &NSView, hints: &LayoutHints) {
    let p = &hints.padding;

    match hints.alignment {
        None => {
            view.topAnchor()
                .constraintEqualToAnchor_constant(&parent.topAnchor(), p.top as f64)
                .setActive(true);
            view.leadingAnchor()
                .constraintEqualToAnchor_constant(&parent.leadingAnchor(), p.left as f64)
                .setActive(true);
            view.trailingAnchor()
                .constraintEqualToAnchor_constant(&parent.trailingAnchor(), -(p.right as f64))
                .setActive(true);
            view.bottomAnchor()
                .constraintEqualToAnchor_constant(&parent.bottomAnchor(), -(p.bottom as f64))
                .setActive(true);
        }
        Some(alignment) => {
            // Horizontal axis
            match alignment {
                Alignment::TopLeading | Alignment::Leading | Alignment::BottomLeading => {
                    view.leadingAnchor()
                        .constraintEqualToAnchor_constant(
                            &parent.leadingAnchor(),
                            p.left as f64,
                        )
                        .setActive(true);
                }
                Alignment::TopTrailing | Alignment::Trailing | Alignment::BottomTrailing => {
                    view.trailingAnchor()
                        .constraintEqualToAnchor_constant(
                            &parent.trailingAnchor(),
                            -(p.right as f64),
                        )
                        .setActive(true);
                }
                Alignment::Top | Alignment::Center | Alignment::Bottom => {
                    view.centerXAnchor()
                        .constraintEqualToAnchor(&parent.centerXAnchor())
                        .setActive(true);
                }
            }
            // Vertical axis
            match alignment {
                Alignment::TopLeading | Alignment::Top | Alignment::TopTrailing => {
                    view.topAnchor()
                        .constraintEqualToAnchor_constant(&parent.topAnchor(), p.top as f64)
                        .setActive(true);
                }
                Alignment::BottomLeading | Alignment::Bottom | Alignment::BottomTrailing => {
                    view.bottomAnchor()
                        .constraintEqualToAnchor_constant(
                            &parent.bottomAnchor(),
                            -(p.bottom as f64),
                        )
                        .setActive(true);
                }
                Alignment::Leading | Alignment::Center | Alignment::Trailing => {
                    view.centerYAnchor()
                        .constraintEqualToAnchor(&parent.centerYAnchor())
                        .setActive(true);
                }
            }
        }
    }

    if let Some(w) = hints.fixed_width {
        view.widthAnchor()
            .constraintEqualToConstant(w as f64)
            .setActive(true);
    }
    if let Some(h) = hints.fixed_height {
        view.heightAnchor()
            .constraintEqualToConstant(h as f64)
            .setActive(true);
    }
}

fn apply_size_hints(view: &NSView, hints: &LayoutHints) {
    if let Some(w) = hints.fixed_width {
        view.widthAnchor()
            .constraintEqualToConstant(w as f64)
            .setActive(true);
    }
    if let Some(h) = hints.fixed_height {
        view.heightAnchor()
            .constraintEqualToConstant(h as f64)
            .setActive(true);
    }
}

/// Resolve per-child horizontal cross-axis alignment for a Column.
/// A child's `hints.alignment` overrides the container's `CrossAxisAlignment`
/// when the alignment maps unambiguously to a horizontal position.
fn column_cross(hints: &LayoutHints, container: CrossAxisAlignment) -> CrossAxisAlignment {
    match hints.alignment {
        Some(Alignment::Leading | Alignment::TopLeading | Alignment::BottomLeading) => {
            CrossAxisAlignment::Start
        }
        Some(Alignment::Trailing | Alignment::TopTrailing | Alignment::BottomTrailing) => {
            CrossAxisAlignment::End
        }
        Some(Alignment::Center | Alignment::Top | Alignment::Bottom) => CrossAxisAlignment::Center,
        None => container,
    }
}

/// Resolve per-child vertical cross-axis alignment for a Row.
fn row_cross(hints: &LayoutHints, container: CrossAxisAlignment) -> CrossAxisAlignment {
    match hints.alignment {
        Some(Alignment::Top | Alignment::TopLeading | Alignment::TopTrailing) => {
            CrossAxisAlignment::Start
        }
        Some(Alignment::Bottom | Alignment::BottomLeading | Alignment::BottomTrailing) => {
            CrossAxisAlignment::End
        }
        Some(Alignment::Center | Alignment::Leading | Alignment::Trailing) => {
            CrossAxisAlignment::Center
        }
        None => container,
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
        match column_cross(&entry.hints, cross) {
            CrossAxisAlignment::Stretch => {
                view.leadingAnchor()
                    .constraintEqualToAnchor_constant(
                        &container.leadingAnchor(),
                        p.left as f64,
                    )
                    .setActive(true);
                view.trailingAnchor()
                    .constraintEqualToAnchor_constant(
                        &container.trailingAnchor(),
                        -(p.right as f64),
                    )
                    .setActive(true);
            }
            CrossAxisAlignment::Start => {
                view.leadingAnchor()
                    .constraintEqualToAnchor_constant(
                        &container.leadingAnchor(),
                        p.left as f64,
                    )
                    .setActive(true);
            }
            CrossAxisAlignment::Center => {
                view.centerXAnchor()
                    .constraintEqualToAnchor(&container.centerXAnchor())
                    .setActive(true);
            }
            CrossAxisAlignment::End => {
                view.trailingAnchor()
                    .constraintEqualToAnchor_constant(
                        &container.trailingAnchor(),
                        -(p.right as f64),
                    )
                    .setActive(true);
            }
        }

        // Main axis (vertical for Column)
        if i == 0 {
            view.topAnchor()
                .constraintEqualToAnchor_constant(&container.topAnchor(), p.top as f64)
                .setActive(true);
        } else {
            let prev = &entries[i - 1];
            let gap = spacing + prev.hints.padding.bottom as f64 + p.top as f64;
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
        .constraintEqualToAnchor_constant(
            &container.bottomAnchor(),
            -(last.hints.padding.bottom as f64),
        )
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
        match row_cross(&entry.hints, cross) {
            CrossAxisAlignment::Stretch => {
                view.topAnchor()
                    .constraintEqualToAnchor_constant(&container.topAnchor(), p.top as f64)
                    .setActive(true);
                view.bottomAnchor()
                    .constraintEqualToAnchor_constant(
                        &container.bottomAnchor(),
                        -(p.bottom as f64),
                    )
                    .setActive(true);
            }
            CrossAxisAlignment::Start => {
                view.topAnchor()
                    .constraintEqualToAnchor_constant(&container.topAnchor(), p.top as f64)
                    .setActive(true);
            }
            CrossAxisAlignment::Center => {
                view.centerYAnchor()
                    .constraintEqualToAnchor(&container.centerYAnchor())
                    .setActive(true);
            }
            CrossAxisAlignment::End => {
                view.bottomAnchor()
                    .constraintEqualToAnchor_constant(
                        &container.bottomAnchor(),
                        -(p.bottom as f64),
                    )
                    .setActive(true);
            }
        }

        // Main axis (horizontal for Row)
        if i == 0 {
            view.leadingAnchor()
                .constraintEqualToAnchor_constant(&container.leadingAnchor(), p.left as f64)
                .setActive(true);
        } else {
            let prev = &entries[i - 1];
            let gap = spacing + prev.hints.padding.right as f64 + p.left as f64;
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
        .constraintEqualToAnchor_constant(
            &container.trailingAnchor(),
            -(last.hints.padding.right as f64),
        )
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

pub(super) fn attach_to_parent(ctx: &mut SetupContext, container: &Retained<NSView>, hints: &LayoutHints) {
    if let Some(parent_signal) = ctx.use_context(&PARENT_VIEW) {
        let parent = parent_signal.read();
        parent.add_child(container.clone());

        ctx.on_cleanup({
            let c = container.clone();
            move || c.removeFromSuperview()
        });

        if let Some(registry) = ctx.use_context(&CHILD_VIEW_REGISTRY) {
            registry
                .read()
                .borrow_mut()
                .push(ChildEntry { view: container.clone(), hints: *hints });
        } else if let ViewParent::View(parent_nsview) = &parent {
            activate_fill(container, &parent_nsview, hints);
        }
        // ViewParent::Stack: NSStackView arranges its children; no extra constraints needed.
    }
}

/// Attach a leaf (non-container) view to the parent from context, applying
/// layout hints. Disables autoresizing mask translation and registers cleanup.
pub(super) fn attach_leaf_view(ctx: &mut SetupContext, nsview: Retained<NSView>) {
    nsview.setTranslatesAutoresizingMaskIntoConstraints(false);
    let hints = ctx.use_context(&LAYOUT_HINTS).map(|s| s.read()).unwrap_or_default();
    attach_to_parent(ctx, &nsview, &hints);
    ctx.on_cleanup(move || drop(nsview));
}

// ── Column ─────────────────────────────────────────────────────────────────

pub struct Column {
    spacing: Box<dyn Signal<Value = f64> + 'static>,
    cross_axis_alignment: CrossAxisAlignment,
    children: Vec<BoxedComponent>,
}

impl Column {
    pub fn new() -> Self {
        Self {
            spacing: Box::new(0.0_f64.into_signal()),
            cross_axis_alignment: CrossAxisAlignment::Stretch,
            children: Vec::new(),
        }
    }

    pub fn spacing(mut self, s: impl Signal<Value = f64> + 'static) -> Self {
        self.spacing = Box::new(s);
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
        let spacing = spacing.read();

        let mtm = MainThreadMarker::new().expect("must be on main thread");
        let container = NSView::new(mtm);
        container.setTranslatesAutoresizingMaskIntoConstraints(false);

        let hints = ctx.use_context(&LAYOUT_HINTS).map(|s| s.read()).unwrap_or_default();
        attach_to_parent(ctx, &container, &hints);

        ctx.provide_context(&PARENT_VIEW, ViewParent::View(container.clone()));
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
    spacing: Box<dyn Signal<Value = f64> + 'static>,
    cross_axis_alignment: CrossAxisAlignment,
    children: Vec<BoxedComponent>,
}

impl Row {
    pub fn new() -> Self {
        Self {
            spacing: Box::new(0.0_f64.into_signal()),
            cross_axis_alignment: CrossAxisAlignment::Stretch,
            children: Vec::new(),
        }
    }

    pub fn spacing(mut self, s: impl Signal<Value = f64> + 'static) -> Self {
        self.spacing = Box::new(s);
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
        let spacing = spacing.read();

        let mtm = MainThreadMarker::new().expect("must be on main thread");
        let container = NSView::new(mtm);
        container.setTranslatesAutoresizingMaskIntoConstraints(false);

        let hints = ctx.use_context(&LAYOUT_HINTS).map(|s| s.read()).unwrap_or_default();
        attach_to_parent(ctx, &container, &hints);

        ctx.provide_context(&PARENT_VIEW, ViewParent::View(container.clone()));
        ctx.provide_context(&LAYOUT_HINTS, LayoutHints::default());
        let registry: ChildViewRegistry = Default::default();
        ctx.provide_context(&CHILD_VIEW_REGISTRY, registry.clone());

        for child in children {
            child.setup(&mut ctx.new_child());
        }

        apply_row_constraints(&container, &registry.borrow(), spacing, cross_axis_alignment);
    }
}

// ── Platform trait impls ───────────────────────────────────────────────────

impl ui_core::widgets::Row for Row {
    fn new() -> Self {
        Row::new()
    }
    fn spacing(self, spacing: impl Signal<Value = f64> + 'static) -> Self {
        self.spacing(spacing)
    }
    fn cross_axis_alignment(self, alignment: CrossAxisAlignment) -> Self {
        self.cross_axis_alignment(alignment)
    }
    fn child(self, child: impl Component + 'static) -> Self {
        self.child(child)
    }
}

impl ui_core::widgets::Column for Column {
    fn new() -> Self {
        Column::new()
    }
    fn spacing(self, spacing: impl Signal<Value = f64> + 'static) -> Self {
        self.spacing(spacing)
    }
    fn cross_axis_alignment(self, alignment: CrossAxisAlignment) -> Self {
        self.cross_axis_alignment(alignment)
    }
    fn child(self, child: impl Component + 'static) -> Self {
        self.child(child)
    }
}
