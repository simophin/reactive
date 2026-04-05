use super::types::{Alignment, BoxModifier, ChildLayoutInfo, CrossAxisAlignment};

/// Platform-agnostic 2D size.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

/// A positioned, sized rectangle in container-local coordinates (top-left origin).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub fn size(self) -> Size {
        Size {
            width: self.width,
            height: self.height,
        }
    }
}

/// Constraint on one axis passed to [`LayoutHost::measure_child`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AxisConstraint {
    /// This axis is fixed; the child must use exactly this size.
    Exact(f32),
    /// Child may be at most this large; reports natural size within that bound.
    AtMost(f32),
    /// No constraint; child reports its natural/intrinsic size.
    Unconstrained,
}

/// Two-axis measurement input for [`LayoutHost::measure_child`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SizeConstraint {
    pub width: AxisConstraint,
    pub height: AxisConstraint,
}

impl SizeConstraint {
    pub fn unconstrained() -> Self {
        Self {
            width: AxisConstraint::Unconstrained,
            height: AxisConstraint::Unconstrained,
        }
    }
}

/// Both minimum and preferred sizes for a child, as returned by
/// [`LayoutHost::measure_child`].
///
/// `min` is the smallest size at which the child can render without clipping.
/// `natural` is the preferred size within the supplied constraint.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Measurement {
    pub min: Size,
    pub natural: Size,
}

/// Platform binding for the layout algorithm.
///
/// Implemented once per platform on the custom container view.  The algorithm
/// calls [`measure_child`] during the measure pass (read-only) and
/// [`place_child`] during the layout pass (write), using the same child index
/// space for both.
pub trait LayoutHost {
    fn child_count(&self) -> usize;

    /// Return the child's minimum and preferred sizes given the supplied constraint.
    ///
    /// `Measurement::min` is the smallest size the child can accept without
    /// clipping (GTK's minimum, AppKit's `fittingSize`).  `Measurement::natural`
    /// is the preferred size within the constraint.  For
    /// [`AxisConstraint::Exact`] the natural value on that axis should equal
    /// the exact value.
    fn measure_child(&self, index: usize, constraint: SizeConstraint) -> Measurement;

    /// Set the child's frame to `frame` in container-local coordinates.
    ///
    /// (0, 0) is the top-left corner of the container; y increases downward.
    fn place_child(&self, index: usize, frame: Rect);
}

// ── Public algorithm entry-points ────────────────────────────────────────────

/// Lay out `child_infos` in a flex container and place each child via `host`.
///
/// `available` is the container's current allocated size.  Returns the
/// container's natural preferred size (use it for intrinsic-size reporting).
pub fn compute_flex_layout(
    host: &dyn LayoutHost,
    child_infos: &[ChildLayoutInfo],
    vertical: bool,
    spacing: f32,
    cross_axis: CrossAxisAlignment,
    available: Size,
) -> Size {
    if child_infos.is_empty() {
        return Size::default();
    }

    let count = child_infos.len();
    let spacing_total = spacing * count.saturating_sub(1) as f32;
    let cross_avail = cross_component(available, vertical);

    // ── Pass 1: measure all children; distribute main-axis space ─────────────
    let mut main_sizes = vec![0.0f32; count];
    let mut measurements = vec![Measurement::default(); count];
    let mut non_flex_main = 0.0f32;
    let mut flex_total = 0usize;

    for (i, info) in child_infos.iter().enumerate() {
        let constraint = main_measure_constraint(cross_avail, cross_axis, vertical);
        let m = measure_with_modifiers(host, i, &info.box_modifiers.modifiers, constraint);
        measurements[i] = m;
        if let Some(flex) = info.flex.flex {
            flex_total += flex.get();
        } else {
            let main = main_component(m.natural, vertical);
            main_sizes[i] = main;
            non_flex_main += main;
        }
    }

    // Distribute remaining space to flex children, respecting their minimum sizes.
    let remaining =
        (main_component(available, vertical) - non_flex_main - spacing_total).max(0.0);
    if flex_total > 0 {
        for (i, info) in child_infos.iter().enumerate() {
            if let Some(flex) = info.flex.flex {
                let ratio = flex.get() as f32 / flex_total as f32;
                let min_main = main_component(measurements[i].min, vertical);
                main_sizes[i] = (remaining * ratio).max(min_main);
            }
        }
    }

    // ── Pass 2: place all children ────────────────────────────────────────────
    let mut cursor = 0.0f32;
    let mut max_cross = 0.0f32;

    for (i, info) in child_infos.iter().enumerate() {
        let main_size = main_sizes[i];

        let cross_size = match cross_axis {
            CrossAxisAlignment::Stretch => cross_avail,
            _ => {
                let constraint = cross_measure_constraint(main_size, cross_avail, vertical);
                cross_component(
                    measure_with_modifiers(host, i, &info.box_modifiers.modifiers, constraint)
                        .natural,
                    vertical,
                )
            }
        };

        let cross_offset = match cross_axis {
            CrossAxisAlignment::Stretch | CrossAxisAlignment::Start => 0.0,
            CrossAxisAlignment::Center => (cross_avail - cross_size) / 2.0,
            CrossAxisAlignment::End => cross_avail - cross_size,
        };

        let slot = if vertical {
            Rect {
                x: cross_offset,
                y: cursor,
                width: cross_size,
                height: main_size,
            }
        } else {
            Rect {
                x: cursor,
                y: cross_offset,
                width: main_size,
                height: cross_size,
            }
        };

        place_with_modifiers(host, i, &info.box_modifiers.modifiers, slot);

        cursor += main_size + if i + 1 < count { spacing } else { 0.0 };
        max_cross = max_cross.max(cross_size);
    }

    let natural_main = if flex_total > 0 {
        // When flex children are present, fill the available space
        main_component(available, vertical)
    } else {
        cursor
    };

    if vertical {
        Size {
            width: max_cross,
            height: natural_main,
        }
    } else {
        Size {
            width: natural_main,
            height: max_cross,
        }
    }
}

/// Compute the container's minimum and preferred sizes without a concrete allocation.
///
/// Used for intrinsic-size / natural-size reporting (e.g. `intrinsicContentSize`
/// on AppKit, or `measure()` on GTK).
pub fn measure_flex_container(
    host: &dyn LayoutHost,
    child_infos: &[ChildLayoutInfo],
    vertical: bool,
    spacing: f32,
) -> Measurement {
    let mut min_main = 0.0f32;
    let mut nat_main = 0.0f32;
    let mut min_cross = 0.0f32;
    let mut nat_cross = 0.0f32;

    for (i, info) in child_infos.iter().enumerate() {
        let m = measure_with_modifiers(
            host,
            i,
            &info.box_modifiers.modifiers,
            SizeConstraint::unconstrained(),
        );
        min_main += main_component(m.min, vertical);
        nat_main += main_component(m.natural, vertical);
        min_cross = min_cross.max(cross_component(m.min, vertical));
        nat_cross = nat_cross.max(cross_component(m.natural, vertical));
    }

    let spacing_total = spacing * child_infos.len().saturating_sub(1) as f32;

    if vertical {
        Measurement {
            min: Size {
                width: min_cross,
                height: min_main + spacing_total,
            },
            natural: Size {
                width: nat_cross,
                height: nat_main + spacing_total,
            },
        }
    } else {
        Measurement {
            min: Size {
                width: min_main + spacing_total,
                height: min_cross,
            },
            natural: Size {
                width: nat_main + spacing_total,
                height: nat_cross,
            },
        }
    }
}

// ── Modifier chain helpers ────────────────────────────────────────────────────

/// Compute the minimum and preferred sizes of a child as seen through its modifier chain.
fn measure_with_modifiers(
    host: &dyn LayoutHost,
    index: usize,
    modifiers: &[BoxModifier],
    constraint: SizeConstraint,
) -> Measurement {
    match modifiers.split_first() {
        None => {
            let m = host.measure_child(index, constraint);
            Measurement {
                min: m.min,
                natural: Size {
                    width: clamp(m.natural.width, constraint.width),
                    height: clamp(m.natural.height, constraint.height),
                },
            }
        }
        Some((BoxModifier::Padding(insets), rest)) => {
            let h_pad = (insets.left + insets.right) as f32;
            let v_pad = (insets.top + insets.bottom) as f32;
            let inner = SizeConstraint {
                width: shrink(constraint.width, h_pad),
                height: shrink(constraint.height, v_pad),
            };
            let inner_m = measure_with_modifiers(host, index, rest, inner);
            Measurement {
                min: Size {
                    width: inner_m.min.width + h_pad,
                    height: inner_m.min.height + v_pad,
                },
                natural: Size {
                    width: inner_m.natural.width + h_pad,
                    height: inner_m.natural.height + v_pad,
                },
            }
        }
        Some((BoxModifier::Align(_), rest)) => {
            // Align measures child unconstrained; preferred size = child's natural size.
            // (When placed, it positions the child within the given slot.)
            measure_with_modifiers(host, index, rest, SizeConstraint::unconstrained())
        }
        Some((BoxModifier::SizedBox { width, height }, rest)) => {
            let w = width.map(|w| w as f32);
            let h = height.map(|h| h as f32);
            let inner = SizeConstraint {
                width: w.map(AxisConstraint::Exact).unwrap_or(constraint.width),
                height: h.map(AxisConstraint::Exact).unwrap_or(constraint.height),
            };
            let inner_m = measure_with_modifiers(host, index, rest, inner);
            Measurement {
                min: Size {
                    width: w.unwrap_or(inner_m.min.width),
                    height: h.unwrap_or(inner_m.min.height),
                },
                natural: Size {
                    width: w.unwrap_or(inner_m.natural.width),
                    height: h.unwrap_or(inner_m.natural.height),
                },
            }
        }
    }
}

/// Place a child through its modifier chain, given the outer slot rect.
fn place_with_modifiers(
    host: &dyn LayoutHost,
    index: usize,
    modifiers: &[BoxModifier],
    slot: Rect,
) {
    match modifiers.split_first() {
        None => host.place_child(index, slot),
        Some((BoxModifier::Padding(insets), rest)) => {
            let inner = Rect {
                x: slot.x + insets.left as f32,
                y: slot.y + insets.top as f32,
                width: (slot.width - (insets.left + insets.right) as f32).max(0.0),
                height: (slot.height - (insets.top + insets.bottom) as f32).max(0.0),
            };
            place_with_modifiers(host, index, rest, inner);
        }
        Some((BoxModifier::Align(alignment), rest)) => {
            let inner_size =
                measure_with_modifiers(host, index, rest, SizeConstraint::unconstrained())
                    .natural;
            let inner = align_in(inner_size, slot, *alignment);
            place_with_modifiers(host, index, rest, inner);
        }
        Some((BoxModifier::SizedBox { width, height }, rest)) => {
            let inner = Rect {
                x: slot.x,
                y: slot.y,
                width: width.map(|w| w as f32).unwrap_or(slot.width),
                height: height.map(|h| h as f32).unwrap_or(slot.height),
            };
            place_with_modifiers(host, index, rest, inner);
        }
    }
}

fn align_in(child: Size, slot: Rect, alignment: Alignment) -> Rect {
    let (hf, vf) = match alignment {
        Alignment::TopLeading => (0.0_f32, 0.0_f32),
        Alignment::Top => (0.5, 0.0),
        Alignment::TopTrailing => (1.0, 0.0),
        Alignment::Leading => (0.0, 0.5),
        Alignment::Center => (0.5, 0.5),
        Alignment::Trailing => (1.0, 0.5),
        Alignment::BottomLeading => (0.0, 1.0),
        Alignment::Bottom => (0.5, 1.0),
        Alignment::BottomTrailing => (1.0, 1.0),
    };
    Rect {
        x: slot.x + (slot.width - child.width) * hf,
        y: slot.y + (slot.height - child.height) * vf,
        width: child.width,
        height: child.height,
    }
}

// ── Constraint / axis helpers ─────────────────────────────────────────────────

fn clamp(natural: f32, constraint: AxisConstraint) -> f32 {
    match constraint {
        AxisConstraint::Exact(v) => v,
        AxisConstraint::AtMost(max) => natural.min(max),
        AxisConstraint::Unconstrained => natural,
    }
}

fn shrink(constraint: AxisConstraint, by: f32) -> AxisConstraint {
    match constraint {
        AxisConstraint::Exact(v) => AxisConstraint::Exact((v - by).max(0.0)),
        AxisConstraint::AtMost(max) => AxisConstraint::AtMost((max - by).max(0.0)),
        AxisConstraint::Unconstrained => AxisConstraint::Unconstrained,
    }
}

/// Constraint used when measuring a child for main-axis size in pass 1.
fn main_measure_constraint(
    cross_avail: f32,
    cross_axis: CrossAxisAlignment,
    vertical: bool,
) -> SizeConstraint {
    let cross = match cross_axis {
        CrossAxisAlignment::Stretch => AxisConstraint::Exact(cross_avail),
        _ => AxisConstraint::AtMost(cross_avail),
    };
    if vertical {
        SizeConstraint {
            width: cross,
            height: AxisConstraint::Unconstrained,
        }
    } else {
        SizeConstraint {
            width: AxisConstraint::Unconstrained,
            height: cross,
        }
    }
}

/// Constraint used when measuring a child for cross-axis size in pass 2.
fn cross_measure_constraint(main_size: f32, cross_avail: f32, vertical: bool) -> SizeConstraint {
    if vertical {
        SizeConstraint {
            width: AxisConstraint::AtMost(cross_avail),
            height: AxisConstraint::Exact(main_size),
        }
    } else {
        SizeConstraint {
            width: AxisConstraint::Exact(main_size),
            height: AxisConstraint::AtMost(cross_avail),
        }
    }
}

fn main_component(size: Size, vertical: bool) -> f32 {
    if vertical { size.height } else { size.width }
}

fn cross_component(size: Size, vertical: bool) -> f32 {
    if vertical { size.width } else { size.height }
}
