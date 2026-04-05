use super::context::ChildWidgetEntry;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::subclass::prelude::*;
use std::cell::RefCell;
use ui_core::layout::algorithm::{
    AxisConstraint, LayoutHost, Measurement, Rect, Size, SizeConstraint,
};
use ui_core::layout::{
    ChildLayoutInfo, CrossAxisAlignment, compute_flex_layout, measure_flex_container,
};

pub(crate) struct FlexData {
    pub children: Vec<ChildWidgetEntry>,
    pub vertical: bool,
    pub spacing: f32,
    pub cross_axis: CrossAxisAlignment,
}

impl Default for FlexData {
    fn default() -> Self {
        Self {
            children: Vec::new(),
            vertical: false,
            spacing: 0.0,
            cross_axis: CrossAxisAlignment::Start,
        }
    }
}

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct ConstraintHost {
        pub flex_data: RefCell<FlexData>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ConstraintHost {
        const NAME: &'static str = "ReactiveConstraintHost";
        type Type = super::ConstraintHost;
        type ParentType = gtk4::Widget;

        fn class_init(klass: &mut Self::Class) {
            // No layout manager — we handle everything in size_allocate.
            klass.set_css_name("widget");
        }
    }

    impl ObjectImpl for ConstraintHost {
        fn dispose(&self) {
            while let Some(child) = self.obj().first_child() {
                child.unparent();
            }
        }
    }

    impl WidgetImpl for ConstraintHost {
        fn measure(&self, orientation: gtk4::Orientation, _for_size: i32) -> (i32, i32, i32, i32) {
            let data = self.flex_data.borrow();
            if data.children.is_empty() {
                return (0, 0, -1, -1);
            }

            let child_infos: Vec<ChildLayoutInfo> =
                data.children.iter().map(|e| e.layout.clone()).collect();
            let host = GtkFlexHost {
                children: &data.children,
            };
            let m = measure_flex_container(&host, &child_infos, data.vertical, data.spacing);

            let (min, natural) = match orientation {
                gtk4::Orientation::Horizontal => (m.min.width as i32, m.natural.width as i32),
                gtk4::Orientation::Vertical => (m.min.height as i32, m.natural.height as i32),
                _ => (0, 0),
            };
            (min.max(0), natural.max(min.max(0)), -1, -1)
        }

        fn size_allocate(&self, width: i32, height: i32, baseline: i32) {
            self.parent_size_allocate(width, height, baseline);

            let data = self.flex_data.borrow();
            if data.children.is_empty() {
                return;
            }

            let available = Size {
                width: width as f32,
                height: height as f32,
            };
            let child_infos: Vec<ChildLayoutInfo> =
                data.children.iter().map(|e| e.layout.clone()).collect();
            let host = GtkFlexHost {
                children: &data.children,
            };
            compute_flex_layout(
                &host,
                &child_infos,
                data.vertical,
                data.spacing,
                data.cross_axis,
                available,
            );
        }
    }
}

glib::wrapper! {
    pub struct ConstraintHost(ObjectSubclass<imp::ConstraintHost>)
        @extends gtk4::Widget,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget;
}

impl ConstraintHost {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_flex_params(&self, vertical: bool, spacing: f32, cross_axis: CrossAxisAlignment) {
        let mut data = self.imp().flex_data.borrow_mut();
        data.vertical = vertical;
        data.spacing = spacing;
        data.cross_axis = cross_axis;
    }

    /// Update the child list and schedule a layout pass.
    ///
    /// Only unparents/reparents widgets when the set of native widgets actually
    /// changes.  Modifier-only updates (e.g. a reactive `SizedBox` width) skip
    /// the parenting work and only refresh the stored layout metadata before
    /// queueing an allocation.
    pub fn update_children(&self, entries: Vec<ChildWidgetEntry>) {
        let needs_remount = {
            let old = self.imp().flex_data.borrow();
            old.children.len() != entries.len()
                || old
                    .children
                    .iter()
                    .zip(entries.iter())
                    .any(|(o, n)| o.native != n.native)
        };

        if needs_remount {
            // Detach previous widgets.
            let old_natives: Vec<_> = self
                .imp()
                .flex_data
                .borrow()
                .children
                .iter()
                .map(|e| e.native.clone())
                .collect();
            for widget in &old_natives {
                widget.unparent();
            }

            // Attach new widgets.
            let mut prev: Option<gtk4::Widget> = None;
            for entry in &entries {
                entry.native.insert_after(self, prev.as_ref());
                prev = Some(entry.native.clone());
            }
        }

        // Always refresh layout metadata and queue an allocation pass.
        self.imp().flex_data.borrow_mut().children = entries;
        self.queue_allocate();
    }
}

// ── GTK LayoutHost implementation ─────────────────────────────────────────────

struct GtkFlexHost<'a> {
    children: &'a [ChildWidgetEntry],
}

impl LayoutHost for GtkFlexHost<'_> {
    fn child_count(&self) -> usize {
        self.children.len()
    }

    fn measure_child(&self, index: usize, constraint: SizeConstraint) -> Measurement {
        let widget = &self.children[index].native;

        // GTK height-for-width geometry negotiation.
        // First measure unconstrained to get min and natural widths.
        let (min_w, nat_w, _, _) = widget.measure(gtk4::Orientation::Horizontal, -1);

        // For constrained measurement, re-measure with the hinted width.
        let for_width = match constraint.width {
            AxisConstraint::Exact(v) | AxisConstraint::AtMost(v) => v as i32,
            AxisConstraint::Unconstrained => -1,
        };
        let constrained_nat_w = if for_width == -1 {
            nat_w
        } else {
            let (_, cnw, _, _) = widget.measure(gtk4::Orientation::Horizontal, for_width);
            cnw
        };

        let resolved_w = match constraint.width {
            AxisConstraint::Exact(v) => v,
            AxisConstraint::AtMost(max) => (constrained_nat_w as f32).min(max),
            AxisConstraint::Unconstrained => nat_w as f32,
        };

        // Measure height at min width (for min height) and at natural width (for natural height).
        let (min_h, _, _, _) = widget.measure(gtk4::Orientation::Vertical, min_w);
        let (_, nat_h, _, _) = widget.measure(gtk4::Orientation::Vertical, resolved_w as i32);

        let resolved_h = match constraint.height {
            AxisConstraint::Exact(v) => v,
            AxisConstraint::AtMost(max) => (nat_h as f32).min(max),
            AxisConstraint::Unconstrained => nat_h as f32,
        };

        Measurement {
            min: Size {
                width: min_w as f32,
                height: min_h as f32,
            },
            natural: Size {
                width: resolved_w,
                height: resolved_h,
            },
        }
    }

    fn place_child(&self, index: usize, frame: Rect) {
        let widget = &self.children[index].native;
        widget.size_allocate(
            &gtk4::Allocation::new(
                frame.x as i32,
                frame.y as i32,
                (frame.width as i32).max(1),
                (frame.height as i32).max(1),
            ),
            -1,
        );
    }
}
