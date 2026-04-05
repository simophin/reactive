use objc2::rc::Retained;
use objc2::{DefinedClass, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::NSView;
use objc2_core_foundation::CGFloat;
use objc2_foundation::{MainThreadMarker, NSPoint, NSRect, NSSize};
use std::cell::RefCell;
use ui_core::ChildEntry;
use ui_core::layout::algorithm::{
    AxisConstraint, LayoutHost, Measurement, Rect, Size, SizeConstraint,
};
use ui_core::layout::{
    ChildLayoutInfo, CrossAxisAlignment, compute_flex_layout, measure_flex_container,
};

pub(crate) type ChildViewEntry = ChildEntry<Retained<NSView>>;

pub struct FlexData {
    pub children: Vec<ChildViewEntry>,
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

define_class!(
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[ivars = RefCell<FlexData>]
    #[name = "ReactiveLayoutView"]
    pub struct ReactiveLayoutView;

    impl ReactiveLayoutView {
        /// Top-left origin so our algorithm's coordinates map directly to frames.
        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> bool {
            true
        }

        /// Called by AppKit's layout engine whenever this view's layout is invalidated
        /// (e.g. on resize or after `setNeedsLayout`). Runs the algorithm and sets
        /// child frames.
        #[unsafe(method(layout))]
        fn layout_impl(&self) {
            // Let the superclass finish its own layout work first.
            let _: () = unsafe { msg_send![super(self), layout] };

            let data = self.ivars().borrow();
            if data.children.is_empty() {
                return;
            }

            let frame = unsafe { self.frame() };
            let available = Size {
                width: frame.size.width as f32,
                height: frame.size.height as f32,
            };

            let child_infos: Vec<ChildLayoutInfo> =
                data.children.iter().map(|e| e.layout.clone()).collect();

            let host = AppKitFlexHost {
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

        /// Reports the preferred size so Auto Layout can size this view correctly.
        #[unsafe(method(intrinsicContentSize))]
        fn intrinsic_content_size_impl(&self) -> NSSize {
            let data = self.ivars().borrow();
            if data.children.is_empty() {
                return NSSize {
                    width: 0.0,
                    height: 0.0,
                };
            }
            let child_infos: Vec<ChildLayoutInfo> =
                data.children.iter().map(|e| e.layout.clone()).collect();
            let host = AppKitFlexHost {
                children: &data.children,
            };
            let m = measure_flex_container(&host, &child_infos, data.vertical, data.spacing);
            NSSize {
                width: m.natural.width as CGFloat,
                height: m.natural.height as CGFloat,
            }
        }
    }
);

impl ReactiveLayoutView {
    pub fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(RefCell::new(FlexData::default()));
        unsafe { msg_send![super(this), init] }
    }

    pub fn set_flex_params(&self, vertical: bool, spacing: f32, cross_axis: CrossAxisAlignment) {
        let mut data = self.ivars().borrow_mut();
        data.vertical = vertical;
        data.spacing = spacing;
        data.cross_axis = cross_axis;
    }

    /// Update the child list and trigger a layout pass.
    ///
    /// Only removes/re-adds subviews when the set of native views actually
    /// changes (children added or removed).  When only modifier values changed
    /// (e.g. a reactive `SizedBox` width) the view hierarchy is left alone and
    /// only the stored layout metadata + a layout-invalidation call are issued.
    pub fn update_children(&self, entries: Vec<ChildViewEntry>) {
        let needs_remount = {
            let old = self.ivars().borrow();
            old.children.len() != entries.len()
                || old
                    .children
                    .iter()
                    .zip(entries.iter())
                    .any(|(o, n)| o.native != n.native)
        };

        if needs_remount {
            // Detach old subviews.
            let old_natives: Vec<_> = self
                .ivars()
                .borrow()
                .children
                .iter()
                .map(|e| e.native.clone())
                .collect();
            for view in &old_natives {
                unsafe { view.removeFromSuperview() };
            }

            // Attach new subviews.  They will be manually framed by layout(),
            // so turn off Auto Layout for the parent-child relationship.
            for entry in &entries {
                unsafe {
                    entry
                        .native
                        .setTranslatesAutoresizingMaskIntoConstraints(true);
                    self.addSubview(&entry.native);
                }
            }
        }

        // Always update layout metadata and request a new layout pass.
        self.ivars().borrow_mut().children = entries;
        self.invalidateIntrinsicContentSize();
        self.setNeedsLayout(true);
    }
}

// ── AppKit LayoutHost implementation ─────────────────────────────────────────

struct AppKitFlexHost<'a> {
    children: &'a [ChildViewEntry],
}

impl LayoutHost for AppKitFlexHost<'_> {
    fn child_count(&self) -> usize {
        self.children.len()
    }

    fn measure_child(&self, index: usize, constraint: SizeConstraint) -> Measurement {
        let view = &self.children[index].native;
        // `fittingSize` returns the smallest size that satisfies all Auto-Layout
        // constraints — it serves as both the minimum and the natural size.
        let fitting = unsafe { view.fittingSize() };
        let min = Size {
            width: fitting.width as f32,
            height: fitting.height as f32,
        };
        let natural = Size {
            width: match constraint.width {
                AxisConstraint::Exact(v) => v,
                AxisConstraint::AtMost(max) => (fitting.width as f32).min(max),
                AxisConstraint::Unconstrained => fitting.width as f32,
            },
            height: match constraint.height {
                AxisConstraint::Exact(v) => v,
                AxisConstraint::AtMost(max) => (fitting.height as f32).min(max),
                AxisConstraint::Unconstrained => fitting.height as f32,
            },
        };
        Measurement { min, natural }
    }

    fn place_child(&self, index: usize, frame: Rect) {
        let view = &self.children[index].native;
        view.setFrame(NSRect {
            origin: NSPoint {
                x: frame.x as CGFloat,
                y: frame.y as CGFloat,
            },
            size: NSSize {
                width: frame.width as CGFloat,
                height: frame.height as CGFloat,
            },
        });
    }
}
