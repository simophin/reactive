use crate::context::ChildViewEntry;
use objc2::rc::Retained;
use objc2_app_kit::{
    NSLayoutConstraint, NSLayoutDimension, NSLayoutGuide, NSLayoutXAxisAnchor, NSLayoutYAxisAnchor,
    NSView,
};
use objc2_foundation::{NSArray, NSString};
use ui_core::layout::{Alignment, BoxModifier, BoxModifierChain, CrossAxisAlignment};

pub(crate) struct MountedChild {
    view: Retained<NSView>,
    guides: Vec<Retained<NSLayoutGuide>>,
    constraints: Vec<Retained<NSLayoutConstraint>>,
}

impl MountedChild {
    pub(crate) fn activate(&self) {
        activate_constraints(&self.constraints);
    }

    pub(crate) fn set_root_identifier(&self, identifier: &str) {
        self.root().setIdentifier(&NSString::from_str(identifier));
    }

    pub(crate) fn unmount(self, parent: &NSView) {
        deactivate_constraints(&self.constraints);
        for guide in self.guides {
            parent.removeLayoutGuide(&guide);
        }
        self.view.removeFromSuperview();
    }

    pub(crate) fn root(&self) -> &NSLayoutGuide {
        &self.guides[0]
    }
}

pub(crate) trait LayoutItem {
    fn leading_anchor(&self) -> Retained<NSLayoutXAxisAnchor>;
    fn trailing_anchor(&self) -> Retained<NSLayoutXAxisAnchor>;
    fn top_anchor(&self) -> Retained<NSLayoutYAxisAnchor>;
    fn bottom_anchor(&self) -> Retained<NSLayoutYAxisAnchor>;
}

impl LayoutItem for NSView {
    fn leading_anchor(&self) -> Retained<NSLayoutXAxisAnchor> {
        self.leadingAnchor()
    }

    fn trailing_anchor(&self) -> Retained<NSLayoutXAxisAnchor> {
        self.trailingAnchor()
    }

    fn top_anchor(&self) -> Retained<NSLayoutYAxisAnchor> {
        self.topAnchor()
    }

    fn bottom_anchor(&self) -> Retained<NSLayoutYAxisAnchor> {
        self.bottomAnchor()
    }
}

impl LayoutItem for NSLayoutGuide {
    fn leading_anchor(&self) -> Retained<NSLayoutXAxisAnchor> {
        self.leadingAnchor()
    }

    fn trailing_anchor(&self) -> Retained<NSLayoutXAxisAnchor> {
        self.trailingAnchor()
    }

    fn top_anchor(&self) -> Retained<NSLayoutYAxisAnchor> {
        self.topAnchor()
    }

    fn bottom_anchor(&self) -> Retained<NSLayoutYAxisAnchor> {
        self.bottomAnchor()
    }
}

impl<T: LayoutItem> LayoutItem for Retained<T> {
    fn leading_anchor(&self) -> Retained<NSLayoutXAxisAnchor> {
        (**self).leading_anchor()
    }

    fn trailing_anchor(&self) -> Retained<NSLayoutXAxisAnchor> {
        (**self).trailing_anchor()
    }

    fn top_anchor(&self) -> Retained<NSLayoutYAxisAnchor> {
        (**self).top_anchor()
    }

    fn bottom_anchor(&self) -> Retained<NSLayoutYAxisAnchor> {
        (**self).bottom_anchor()
    }
}

pub(crate) fn pin_edges<Inner: LayoutItem, Outer: LayoutItem>(
    inner: &Inner,
    outer: &Outer,
) -> [Retained<NSLayoutConstraint>; 4] {
    [
        inner
            .leading_anchor()
            .constraintEqualToAnchor(&outer.leading_anchor()),
        inner
            .trailing_anchor()
            .constraintEqualToAnchor(&outer.trailing_anchor()),
        inner
            .top_anchor()
            .constraintEqualToAnchor(&outer.top_anchor()),
        inner
            .bottom_anchor()
            .constraintEqualToAnchor(&outer.bottom_anchor()),
    ]
}

pub(crate) fn mount_child_to_parent(parent: &NSView, entry: ChildViewEntry) -> MountedChild {
    parent.addSubview(&entry.native);

    let mut mounted =
        compile_child_layout(parent, entry.native.clone(), &entry.layout.box_modifiers);
    mounted
        .constraints
        .extend(pin_edges(mounted.root(), parent));
    activate_constraints(&mounted.constraints);
    mounted
}

pub(crate) fn compile_child_layout(
    parent: &NSView,
    view: Retained<NSView>,
    modifiers: &BoxModifierChain,
) -> MountedChild {
    let mut guides = Vec::new();
    let mut constraints = Vec::new();
    let root = compile_box(
        parent,
        &view,
        &modifiers.modifiers,
        &mut guides,
        &mut constraints,
    );

    if guides.is_empty() || guides[0] != root {
        guides.insert(0, root);
    }

    MountedChild {
        view,
        guides,
        constraints,
    }
}

pub(crate) fn activate_constraints(constraints: &[Retained<NSLayoutConstraint>]) {
    if constraints.is_empty() {
        return;
    }

    NSLayoutConstraint::activateConstraints(&NSArray::from_retained_slice(constraints));
}

pub(crate) fn deactivate_constraints(constraints: &[Retained<NSLayoutConstraint>]) {
    if constraints.is_empty() {
        return;
    }

    NSLayoutConstraint::deactivateConstraints(&NSArray::from_retained_slice(constraints));
}

pub(crate) fn root_cross_axis_constraints(
    parent: &NSView,
    root: &NSLayoutGuide,
    vertical: bool,
    alignment: CrossAxisAlignment,
) -> Vec<Retained<NSLayoutConstraint>> {
    if vertical {
        match alignment {
            CrossAxisAlignment::Stretch => vec![
                root.leadingAnchor()
                    .constraintEqualToAnchor(&parent.leadingAnchor()),
                root.trailingAnchor()
                    .constraintEqualToAnchor(&parent.trailingAnchor()),
            ],
            CrossAxisAlignment::Start => vec![
                root.leadingAnchor()
                    .constraintEqualToAnchor(&parent.leadingAnchor()),
                root.trailingAnchor()
                    .constraintLessThanOrEqualToAnchor(&parent.trailingAnchor()),
            ],
            CrossAxisAlignment::Center => vec![
                root.centerXAnchor()
                    .constraintEqualToAnchor(&parent.centerXAnchor()),
                root.leadingAnchor()
                    .constraintGreaterThanOrEqualToAnchor(&parent.leadingAnchor()),
                root.trailingAnchor()
                    .constraintLessThanOrEqualToAnchor(&parent.trailingAnchor()),
            ],
            CrossAxisAlignment::End => vec![
                root.leadingAnchor()
                    .constraintGreaterThanOrEqualToAnchor(&parent.leadingAnchor()),
                root.trailingAnchor()
                    .constraintEqualToAnchor(&parent.trailingAnchor()),
            ],
        }
    } else {
        match alignment {
            CrossAxisAlignment::Stretch => vec![
                root.topAnchor()
                    .constraintEqualToAnchor(&parent.topAnchor()),
                root.bottomAnchor()
                    .constraintEqualToAnchor(&parent.bottomAnchor()),
            ],
            CrossAxisAlignment::Start => vec![
                root.topAnchor()
                    .constraintEqualToAnchor(&parent.topAnchor()),
                root.bottomAnchor()
                    .constraintLessThanOrEqualToAnchor(&parent.bottomAnchor()),
            ],
            CrossAxisAlignment::Center => vec![
                root.centerYAnchor()
                    .constraintEqualToAnchor(&parent.centerYAnchor()),
                root.topAnchor()
                    .constraintGreaterThanOrEqualToAnchor(&parent.topAnchor()),
                root.bottomAnchor()
                    .constraintLessThanOrEqualToAnchor(&parent.bottomAnchor()),
            ],
            CrossAxisAlignment::End => vec![
                root.topAnchor()
                    .constraintGreaterThanOrEqualToAnchor(&parent.topAnchor()),
                root.bottomAnchor()
                    .constraintEqualToAnchor(&parent.bottomAnchor()),
            ],
        }
    }
}

pub(crate) fn main_axis_dimension(
    guide: &NSLayoutGuide,
    vertical: bool,
) -> Retained<NSLayoutDimension> {
    if vertical {
        guide.heightAnchor()
    } else {
        guide.widthAnchor()
    }
}

fn compile_box(
    parent: &NSView,
    view: &NSView,
    modifiers: &[BoxModifier],
    guides: &mut Vec<Retained<NSLayoutGuide>>,
    constraints: &mut Vec<Retained<NSLayoutConstraint>>,
) -> Retained<NSLayoutGuide> {
    if let Some((modifier, rest)) = modifiers.split_first() {
        match modifier {
            BoxModifier::Padding(insets) => {
                let outer = new_guide(parent, guides, "padding");
                let inner = compile_box(parent, view, rest, guides, constraints);
                constraints.extend(vec![
                    inner
                        .leadingAnchor()
                        .constraintEqualToAnchor_constant(&outer.leadingAnchor(), insets.left as _),
                    inner.trailingAnchor().constraintEqualToAnchor_constant(
                        &outer.trailingAnchor(),
                        -(insets.right as f64),
                    ),
                    inner
                        .topAnchor()
                        .constraintEqualToAnchor_constant(&outer.topAnchor(), insets.top as _),
                    inner.bottomAnchor().constraintEqualToAnchor_constant(
                        &outer.bottomAnchor(),
                        -(insets.bottom as f64),
                    ),
                ]);
                outer
            }
            BoxModifier::Align(alignment) => {
                let outer = new_guide(parent, guides, "align");
                let inner = compile_box(parent, view, rest, guides, constraints);
                constraints.extend(alignment_constraints(&outer, &inner, *alignment));
                outer
            }
            BoxModifier::SizedBox { width, height } => {
                let outer = new_guide(parent, guides, "sized_box");
                let inner = compile_box(parent, view, rest, guides, constraints);
                constraints.extend(pin_edges(&inner, &outer));
                if let Some(width) = width {
                    constraints.push(outer.widthAnchor().constraintEqualToConstant(*width as _));
                }
                if let Some(height) = height {
                    constraints.push(outer.heightAnchor().constraintEqualToConstant(*height as _));
                }
                outer
            }
        }
    } else {
        let root = new_guide(parent, guides, "content");
        constraints.extend(pin_edges(view, &root));
        root
    }
}

fn new_guide(
    parent: &NSView,
    guides: &mut Vec<Retained<NSLayoutGuide>>,
    identifier: &str,
) -> Retained<NSLayoutGuide> {
    let guide = NSLayoutGuide::new();
    guide.setIdentifier(&NSString::from_str(identifier));
    parent.addLayoutGuide(&guide);
    guides.push(guide.clone());
    guide
}

fn alignment_constraints(
    outer: &NSLayoutGuide,
    inner: &NSLayoutGuide,
    alignment: Alignment,
) -> Vec<Retained<NSLayoutConstraint>> {
    let (horizontal, vertical) = match alignment {
        Alignment::TopLeading => (AxisAlignment::Start, AxisAlignment::Start),
        Alignment::Top => (AxisAlignment::Center, AxisAlignment::Start),
        Alignment::TopTrailing => (AxisAlignment::End, AxisAlignment::Start),
        Alignment::Leading => (AxisAlignment::Start, AxisAlignment::Center),
        Alignment::Center => (AxisAlignment::Center, AxisAlignment::Center),
        Alignment::Trailing => (AxisAlignment::End, AxisAlignment::Center),
        Alignment::BottomLeading => (AxisAlignment::Start, AxisAlignment::End),
        Alignment::Bottom => (AxisAlignment::Center, AxisAlignment::End),
        Alignment::BottomTrailing => (AxisAlignment::End, AxisAlignment::End),
    };

    let mut constraints = horizontal_alignment_constraints(outer, inner, horizontal);
    constraints.extend(vertical_alignment_constraints(outer, inner, vertical));
    constraints
}

#[derive(Clone, Copy)]
enum AxisAlignment {
    Start,
    Center,
    End,
}

fn horizontal_alignment_constraints(
    outer: &NSLayoutGuide,
    inner: &NSLayoutGuide,
    alignment: AxisAlignment,
) -> Vec<Retained<NSLayoutConstraint>> {
    match alignment {
        AxisAlignment::Start => vec![
            inner
                .leadingAnchor()
                .constraintEqualToAnchor(&outer.leadingAnchor()),
            inner
                .trailingAnchor()
                .constraintLessThanOrEqualToAnchor(&outer.trailingAnchor()),
        ],
        AxisAlignment::Center => vec![
            inner
                .centerXAnchor()
                .constraintEqualToAnchor(&outer.centerXAnchor()),
            inner
                .leadingAnchor()
                .constraintGreaterThanOrEqualToAnchor(&outer.leadingAnchor()),
            inner
                .trailingAnchor()
                .constraintLessThanOrEqualToAnchor(&outer.trailingAnchor()),
        ],
        AxisAlignment::End => vec![
            inner
                .leadingAnchor()
                .constraintGreaterThanOrEqualToAnchor(&outer.leadingAnchor()),
            inner
                .trailingAnchor()
                .constraintEqualToAnchor(&outer.trailingAnchor()),
        ],
    }
}

fn vertical_alignment_constraints(
    outer: &NSLayoutGuide,
    inner: &NSLayoutGuide,
    alignment: AxisAlignment,
) -> Vec<Retained<NSLayoutConstraint>> {
    match alignment {
        AxisAlignment::Start => vec![
            inner
                .topAnchor()
                .constraintEqualToAnchor(&outer.topAnchor()),
            inner
                .bottomAnchor()
                .constraintLessThanOrEqualToAnchor(&outer.bottomAnchor()),
        ],
        AxisAlignment::Center => vec![
            inner
                .centerYAnchor()
                .constraintEqualToAnchor(&outer.centerYAnchor()),
            inner
                .topAnchor()
                .constraintGreaterThanOrEqualToAnchor(&outer.topAnchor()),
            inner
                .bottomAnchor()
                .constraintLessThanOrEqualToAnchor(&outer.bottomAnchor()),
        ],
        AxisAlignment::End => vec![
            inner
                .topAnchor()
                .constraintGreaterThanOrEqualToAnchor(&outer.topAnchor()),
            inner
                .bottomAnchor()
                .constraintEqualToAnchor(&outer.bottomAnchor()),
        ],
    }
}
