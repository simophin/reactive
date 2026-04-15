use super::context::{CHILDREN_VIEWS, ChildViewEntry};
use super::layout_view::{AppKitContainerView, ReactiveLayoutView};
use super::view_component::AppKitViewBuilder;
use objc2::rc::Retained;
use objc2_app_kit::NSView;
use objc2_core_foundation::CGFloat;
use objc2_foundation::{MainThreadMarker, NSPoint, NSRect, NSSize};
use reactive_core::{BoxedComponent, Component, SetupContext, Signal};
use std::cell::RefCell;
use std::rc::Rc;
use ui_core::layout::algorithm::{LayoutHost, Measurement, Rect, Size, SizeConstraint};
use ui_core::layout::{
    ChildLayoutInfo, CrossAxisAlignment, compute_flex_layout, measure_flex_container,
};
use ui_core::widgets::{Column, CustomLayoutOperation, Row, SizeSpec};

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
            children: Vec::new(),
            spacing: None,
            cross_axis_alignment: CrossAxisAlignment::Start,
        }
    }
}

impl Row for Flex {
    fn new() -> Self {
        Self::new(false)
    }

    fn spacing(mut self, spacing: impl Signal<Value = usize> + 'static) -> Self {
        self.spacing = Some(Box::new(spacing));
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

impl Column for Flex {
    fn new() -> Self {
        Self::new(true)
    }

    fn spacing(mut self, spacing: impl Signal<Value = usize> + 'static) -> Self {
        self.spacing = Some(Box::new(spacing));
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

fn rlv_into_nsview(v: Retained<ReactiveLayoutView>) -> Retained<NSView> {
    v.into_super()
}

struct FlexOps {
    children: Rc<RefCell<Vec<ChildViewEntry>>>,
    vertical: bool,
    spacing: Rc<dyn Signal<Value = usize>>,
    cross_axis_alignment: CrossAxisAlignment,
}

impl CustomLayoutOperation for FlexOps {
    type BaseView = AppKitContainerView;

    fn on_measure(
        &self,
        _view: &Self::BaseView,
        width: SizeSpec,
        height: SizeSpec,
    ) -> (usize, usize) {
        let entries = self.children.borrow();
        let infos: Vec<ChildLayoutInfo> =
            entries.iter().map(|entry| entry.layout.clone()).collect();
        let host = AppKitFlexHost { children: &entries };
        let measured =
            measure_flex_container(&host, &infos, self.vertical, self.spacing.read() as f32);

        (
            resolve_size_spec(width, measured.natural.width as usize),
            resolve_size_spec(height, measured.natural.height as usize),
        )
    }

    fn on_measure_single(
        &self,
        view: &Self::BaseView,
        _measure: ui_core::widgets::SingleAxisMeasure,
    ) -> ui_core::widgets::SingleAxisMeasureResult {
        let measured = self.on_measure(view, SizeSpec::Unspecified, SizeSpec::Unspecified);
        ui_core::widgets::SingleAxisMeasureResult {
            min: measured.0.min(measured.1),
            natrual: measured.0.max(measured.1),
        }
    }

    fn on_layout(&self, _view: &Self::BaseView, size: (usize, usize)) {
        let entries = self.children.borrow();
        let infos: Vec<ChildLayoutInfo> =
            entries.iter().map(|entry| entry.layout.clone()).collect();
        let host = AppKitFlexHost { children: &entries };
        compute_flex_layout(
            &host,
            &infos,
            self.vertical,
            self.spacing.read() as f32,
            self.cross_axis_alignment,
            Size {
                width: size.0 as f32,
                height: size.1 as f32,
            },
        );
    }
}

struct AppKitFlexHost<'a> {
    children: &'a [ChildViewEntry],
}

impl LayoutHost for AppKitFlexHost<'_> {
    fn child_count(&self) -> usize {
        self.children.len()
    }

    fn measure_child(&self, index: usize, constraint: SizeConstraint) -> Measurement {
        let view = &self.children[index].native;
        let fitting = view.fittingSize();
        let min = Size {
            width: fitting.width as f32,
            height: fitting.height as f32,
        };
        let natural = Size {
            width: match constraint.width {
                ui_core::layout::AxisConstraint::Exact(v) => v,
                ui_core::layout::AxisConstraint::AtMost(max) => (fitting.width as f32).min(max),
                ui_core::layout::AxisConstraint::Unconstrained => fitting.width as f32,
            },
            height: match constraint.height {
                ui_core::layout::AxisConstraint::Exact(v) => v,
                ui_core::layout::AxisConstraint::AtMost(max) => (fitting.height as f32).min(max),
                ui_core::layout::AxisConstraint::Unconstrained => fitting.height as f32,
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

impl Component for Flex {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let Flex {
            vertical,
            children,
            spacing,
            cross_axis_alignment,
        } = *self;

        let spacing_signal: Rc<dyn Signal<Value = usize>> =
            Rc::new(spacing.unwrap_or_else(|| Box::new(0usize)));
        let children_state = Rc::new(RefCell::new(Vec::new()));

        let builder = children.into_iter().fold(
            {
                let children_state = Rc::clone(&children_state);
                let spacing_signal = Rc::clone(&spacing_signal);
                AppKitViewBuilder::create_multiple_child(
                    move |_| {
                        let mtm = MainThreadMarker::new().expect("must be on main thread");
                        ReactiveLayoutView::new(
                            mtm,
                            FlexOps {
                                children: Rc::clone(&children_state),
                                vertical,
                                spacing: Rc::clone(&spacing_signal),
                                cross_axis_alignment,
                            },
                        )
                    },
                    rlv_into_nsview,
                )
            }
            .debug_identifier(if vertical { "Column" } else { "Row" }),
            |builder, child| builder.add_child(child),
        );

        let layout_view = builder.setup(ctx);

        if let Some(children_views) = ctx.use_context(&CHILDREN_VIEWS) {
            let children_state = Rc::clone(&children_state);
            ctx.create_effect(move |_, _| {
                let _ = spacing_signal.read();

                let entries = children_views
                    .read()
                    .iter()
                    .filter_map(|slot| slot.read())
                    .collect::<Vec<_>>();

                *children_state.borrow_mut() = entries.clone();
                layout_view.replace_children(
                    entries
                        .into_iter()
                        .map(|entry| entry.native)
                        .collect::<Vec<_>>(),
                );
            });
        }
    }
}

fn resolve_size_spec(spec: SizeSpec, natural: usize) -> usize {
    match spec {
        SizeSpec::Exactly(size) => size,
        SizeSpec::AtMost(size) => natural.min(size),
        SizeSpec::Unspecified => natural,
    }
}
