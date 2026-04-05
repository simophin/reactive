use gtk4::prelude::*;
use ui_core::layout::{ChildLayoutInfo, CrossAxisAlignment};

/// Apply flex-parent alignment properties to a widget that is a direct child
/// of a flex container.  Called by containers that are not ConstraintHost-based
/// and still rely on GTK's own expand/align mechanism.
pub fn apply_parent_layout(
    widget: &gtk4::Widget,
    layout: &ChildLayoutInfo,
    vertical_parent: bool,
    cross_axis: CrossAxisAlignment,
) {
    if vertical_parent {
        widget.set_halign(cross_axis_to_halign(cross_axis));
        widget.set_hexpand(matches!(cross_axis, CrossAxisAlignment::Stretch));
        widget.set_vexpand(layout.flex.flex.is_some());
    } else {
        widget.set_valign(cross_axis_to_valign(cross_axis));
        widget.set_vexpand(matches!(cross_axis, CrossAxisAlignment::Stretch));
        widget.set_hexpand(layout.flex.flex.is_some());
    }
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
