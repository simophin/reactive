use gtk4::prelude::*;
use ui_core::layout::ChildLayoutInfo;
use ui_core::layout::types::{Alignment, BoxModifier, CrossAxisAlignment};

/// Apply a child's `ChildLayoutInfo` (box modifiers + flex) to a GTK widget
/// before appending it to a container.
///
/// `vertical_parent` — true when the container is a Column (vertical gtk::Box).
/// `cross_axis`      — the container's cross-axis alignment default.
pub fn apply_child_layout(
    widget: &gtk4::Widget,
    layout: &ChildLayoutInfo,
    vertical_parent: bool,
    cross_axis: CrossAxisAlignment,
) {
    // Default cross-axis alignment from the container.
    if vertical_parent {
        widget.set_halign(cross_axis_to_halign(cross_axis));
        widget.set_hexpand(matches!(cross_axis, CrossAxisAlignment::Stretch));
        widget.set_vexpand(layout.flex.flex.is_some());
    } else {
        widget.set_valign(cross_axis_to_valign(cross_axis));
        widget.set_vexpand(matches!(cross_axis, CrossAxisAlignment::Stretch));
        widget.set_hexpand(layout.flex.flex.is_some());
    }

    // Box modifiers: accumulate padding, apply align / sized-box in order.
    let mut margin_top: i32 = 0;
    let mut margin_end: i32 = 0;
    let mut margin_bottom: i32 = 0;
    let mut margin_start: i32 = 0;

    for modifier in &layout.box_modifiers.modifiers {
        match modifier {
            BoxModifier::Padding(insets) => {
                margin_top += insets.top as i32;
                margin_end += insets.right as i32;
                margin_bottom += insets.bottom as i32;
                margin_start += insets.left as i32;
            }
            BoxModifier::Align(alignment) => {
                widget.set_halign(alignment_to_halign(alignment));
                widget.set_valign(alignment_to_valign(alignment));
            }
            BoxModifier::SizedBox { width, height } => {
                if let Some(w) = width {
                    widget.set_width_request(*w as i32);
                    widget.set_hexpand(false);
                    // Only coerce Fill on the cross-axis; on the main axis it
                    // changes where any extra Box allocation is shown.
                    if vertical_parent && widget.halign() == gtk4::Align::Fill {
                        widget.set_halign(gtk4::Align::Center);
                    }
                }
                if let Some(h) = height {
                    widget.set_height_request(*h as i32);
                    widget.set_vexpand(false);
                    if !vertical_parent && widget.valign() == gtk4::Align::Fill {
                        widget.set_valign(gtk4::Align::Center);
                    }
                }
            }
        }
    }

    widget.set_margin_top(margin_top);
    widget.set_margin_end(margin_end);
    widget.set_margin_bottom(margin_bottom);
    widget.set_margin_start(margin_start);
}

fn cross_axis_to_halign(alignment: CrossAxisAlignment) -> gtk4::Align {
    match alignment {
        CrossAxisAlignment::Stretch => gtk4::Align::Fill,
        CrossAxisAlignment::Start => gtk4::Align::Start,
        CrossAxisAlignment::Center => gtk4::Align::Center,
        CrossAxisAlignment::End => gtk4::Align::End,
    }
}

fn cross_axis_to_valign(alignment: CrossAxisAlignment) -> gtk4::Align {
    match alignment {
        CrossAxisAlignment::Stretch => gtk4::Align::Fill,
        CrossAxisAlignment::Start => gtk4::Align::Start,
        CrossAxisAlignment::Center => gtk4::Align::Center,
        CrossAxisAlignment::End => gtk4::Align::End,
    }
}

fn alignment_to_halign(alignment: &Alignment) -> gtk4::Align {
    match alignment {
        Alignment::TopLeading | Alignment::Leading | Alignment::BottomLeading => gtk4::Align::Start,
        Alignment::Top | Alignment::Center | Alignment::Bottom => gtk4::Align::Center,
        Alignment::TopTrailing | Alignment::Trailing | Alignment::BottomTrailing => {
            gtk4::Align::End
        }
    }
}

fn alignment_to_valign(alignment: &Alignment) -> gtk4::Align {
    match alignment {
        Alignment::TopLeading | Alignment::Top | Alignment::TopTrailing => gtk4::Align::Start,
        Alignment::Leading | Alignment::Center | Alignment::Trailing => gtk4::Align::Center,
        Alignment::BottomLeading | Alignment::Bottom | Alignment::BottomTrailing => {
            gtk4::Align::End
        }
    }
}
