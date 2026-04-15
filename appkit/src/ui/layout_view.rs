use objc2::rc::Retained;
use objc2::{DefinedClass, MainThreadOnly, Message, define_class, msg_send};
use objc2_app_kit::NSView;
use objc2_core_foundation::CGFloat;
use objc2_foundation::{MainThreadMarker, NSPoint, NSRect, NSSize};
use std::cell::UnsafeCell;
use std::ops::Deref;
use ui_core::widgets::{
    CustomLayoutOperation, PlatformBaseView, PlatformContainerView, SingleAxisMeasure,
    SingleAxisMeasureResult, SizeSpec,
};

#[derive(Clone)]
pub struct AppKitView(pub Retained<NSView>);

impl Deref for AppKitView {
    type Target = NSView;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Retained<NSView>> for AppKitView {
    fn from(value: Retained<NSView>) -> Self {
        Self(value)
    }
}

impl PartialEq for AppKitView {
    fn eq(&self, other: &Self) -> bool {
        Retained::as_ptr(&self.0) == Retained::as_ptr(&other.0)
    }
}

impl Eq for AppKitView {}

#[derive(Clone)]
pub struct AppKitContainerView(pub Retained<ReactiveLayoutView>);

impl Deref for AppKitContainerView {
    type Target = ReactiveLayoutView;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PartialEq for AppKitContainerView {
    fn eq(&self, other: &Self) -> bool {
        Retained::as_ptr(&self.0) == Retained::as_ptr(&other.0)
    }
}

impl Eq for AppKitContainerView {}

pub struct LayoutViewData {
    children: Vec<AppKitView>,
    ops: Box<dyn CustomLayoutOperation<BaseView = AppKitContainerView>>,
}

define_class!(
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[ivars = UnsafeCell<LayoutViewData>]
    #[name = "ReactiveLayoutView"]
    pub struct ReactiveLayoutView;

    impl ReactiveLayoutView {
        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> bool {
            true
        }

        #[unsafe(method(layout))]
        fn layout_impl(&self) {
            let _: () = unsafe { msg_send![super(self), layout] };
            let size = nsview_size(self);
            self.with_data(|data| data.ops.on_layout(&self.handle(), size));
        }

        #[unsafe(method(intrinsicContentSize))]
        fn intrinsic_content_size_impl(&self) -> NSSize {
            let measured = self.with_data(|data| {
                data.ops.on_measure(&self.handle(), SizeSpec::Unspecified, SizeSpec::Unspecified)
            });

            NSSize {
                width: measured.0 as CGFloat,
                height: measured.1 as CGFloat,
            }
        }
    }
);

impl ReactiveLayoutView {
    pub fn new(
        mtm: MainThreadMarker,
        ops: impl CustomLayoutOperation<BaseView = AppKitContainerView> + 'static,
    ) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(UnsafeCell::new(LayoutViewData {
            children: Vec::new(),
            ops: Box::new(ops),
        }));
        unsafe { msg_send![super(this), init] }
    }

    pub fn new_empty(mtm: MainThreadMarker) -> Retained<Self> {
        Self::new(mtm, NoopLayout)
    }

    pub fn handle(&self) -> AppKitContainerView {
        AppKitContainerView(self.retain())
    }

    fn with_data<R>(&self, f: impl FnOnce(&LayoutViewData) -> R) -> R {
        let data = unsafe { &*self.ivars().get() };
        f(data)
    }

    fn with_data_mut<R>(&self, f: impl FnOnce(&mut LayoutViewData) -> R) -> R {
        let data = unsafe { &mut *self.ivars().get() };
        f(data)
    }

    pub fn update_window_min_size(&self) {
        let Some(window) = self.window() else { return };
        let measured = self.with_data(|data| {
            data.ops
                .on_measure(&self.handle(), SizeSpec::Unspecified, SizeSpec::Unspecified)
        });

        let extra = unsafe { self.superview() }
            .map(|sv| {
                let sv = sv.frame();
                let us = self.frame();
                NSSize {
                    width: (sv.size.width - us.size.width).max(0.0),
                    height: (sv.size.height - us.size.height).max(0.0),
                }
            })
            .unwrap_or(NSSize {
                width: 0.0,
                height: 0.0,
            });

        window.setContentMinSize(NSSize {
            width: measured.0 as CGFloat + extra.width,
            height: measured.1 as CGFloat + extra.height,
        });
    }

    pub fn clear_children(&self) {
        self.with_data_mut(|data| {
            for child in data.children.drain(..) {
                child.removeFromSuperview();
            }
        });
        self.update_window_min_size();
        nsview_request_layout(self);
    }

    pub fn replace_children(&self, children: Vec<Retained<NSView>>) {
        self.clear_children();
        self.with_data_mut(|data| {
            data.children = children.into_iter().map(AppKitView::from).collect();
            for child in &data.children {
                child.removeFromSuperview();
                self.addSubview(child);
            }
        });
        self.update_window_min_size();
        nsview_request_layout(self);
    }
}

struct NoopLayout;

impl CustomLayoutOperation for NoopLayout {
    type BaseView = AppKitContainerView;

    fn on_measure(
        &self,
        _view: &Self::BaseView,
        width: SizeSpec,
        height: SizeSpec,
    ) -> (usize, usize) {
        (
            resolve_size_spec(width, 0.0),
            resolve_size_spec(height, 0.0),
        )
    }

    fn on_measure_single(
        &self,
        _view: &Self::BaseView,
        _measure: SingleAxisMeasure,
    ) -> SingleAxisMeasureResult {
        SingleAxisMeasureResult { min: 0, natrual: 0 }
    }

    fn on_layout(&self, _view: &Self::BaseView, _size: (usize, usize)) {}
}

pub(crate) fn nsview_measure(
    view: &NSView,
    width_spec: SizeSpec,
    height_spec: SizeSpec,
) -> (usize, usize) {
    let fitting = view.fittingSize();
    (
        resolve_size_spec(width_spec, fitting.width),
        resolve_size_spec(height_spec, fitting.height),
    )
}

pub(crate) fn nsview_measure_single(
    view: &NSView,
    measure: SingleAxisMeasure,
) -> SingleAxisMeasureResult {
    let result = match measure {
        // AppKit does not expose an axis-agnostic single-axis query for arbitrary
        // views, so we use the larger intrinsic dimension as a conservative
        // fallback here and the constraint-aware paths below when the caller can
        // provide the dependent axis.
        SingleAxisMeasure::Independent => {
            let fitting = view.fittingSize();
            fitting.width.max(fitting.height)
        }
        SingleAxisMeasure::WidthForHeight(height) => {
            nsview_measure(view, SizeSpec::Unspecified, SizeSpec::Exactly(height)).0 as CGFloat
        }
        SingleAxisMeasure::HeightForWidth(width) => {
            nsview_measure(view, SizeSpec::Exactly(width), SizeSpec::Unspecified).1 as CGFloat
        }
    };

    let result = result.max(0.0) as usize;
    SingleAxisMeasureResult {
        min: result,
        natrual: result,
    }
}

pub(crate) fn nsview_size(view: &NSView) -> (usize, usize) {
    let frame = view.frame();
    (
        frame.size.width.max(0.0) as usize,
        frame.size.height.max(0.0) as usize,
    )
}

pub(crate) fn nsview_request_layout(view: &NSView) {
    view.invalidateIntrinsicContentSize();
    view.setNeedsLayout(true);
    if let Some(superview) = unsafe { view.superview() } {
        superview.setNeedsLayout(true);
    }
}

fn resolve_size_spec(spec: SizeSpec, fitting: CGFloat) -> usize {
    let fitting = fitting.max(0.0) as usize;
    match spec {
        SizeSpec::Exactly(size) => size,
        SizeSpec::AtMost(size) => fitting.min(size),
        SizeSpec::Unspecified => fitting,
    }
}

impl PlatformBaseView for AppKitView {
    fn measure(&self, width_spec: SizeSpec, height_spec: SizeSpec) -> (usize, usize) {
        nsview_measure(self, width_spec, height_spec)
    }

    fn measure_single_axis(&self, measure: SingleAxisMeasure) -> SingleAxisMeasureResult {
        nsview_measure_single(self, measure)
    }

    fn size(&self) -> (usize, usize) {
        nsview_size(self)
    }

    fn request_layout(&self) {
        nsview_request_layout(self);
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl PlatformBaseView for AppKitContainerView {
    fn measure(&self, width_spec: SizeSpec, height_spec: SizeSpec) -> (usize, usize) {
        self.0
            .with_data(|data| data.ops.on_measure(self, width_spec, height_spec))
    }

    fn measure_single_axis(&self, measure: SingleAxisMeasure) -> SingleAxisMeasureResult {
        self.0
            .with_data(|data| data.ops.on_measure_single(self, measure))
    }

    fn size(&self) -> (usize, usize) {
        nsview_size(&self.0)
    }

    fn request_layout(&self) {
        nsview_request_layout(&self.0);
        self.0.update_window_min_size();
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl PlatformContainerView for AppKitContainerView {
    type BaseView = AppKitView;

    fn add_child(&self, child: &Self::BaseView) {
        child.removeFromSuperview();
        self.0.addSubview(child);
        self.0
            .with_data_mut(|data| data.children.push(child.clone()));
        self.0.update_window_min_size();
    }

    fn update_child_at(&self, index: usize, child: &Self::BaseView) {
        child.removeFromSuperview();
        self.0.addSubview(child);
        self.0.with_data_mut(|data| {
            if index < data.children.len() {
                data.children[index] = child.clone();
            } else {
                data.children.push(child.clone());
            }
        });
        self.0.update_window_min_size();
    }

    fn remove_child(&self, child: &Self::BaseView) {
        self.0
            .with_data_mut(|data| data.children.retain(|existing| existing != child));
        child.removeFromSuperview();
        self.0.update_window_min_size();
    }

    fn remove_all_children(&self) {
        self.0.clear_children();
    }

    fn child_at(&self, index: usize) -> Option<&Self::BaseView> {
        let data = unsafe { &*self.0.ivars().get() };
        data.children.get(index)
    }

    fn child_count(&self) -> usize {
        self.0.with_data(|data| data.children.len())
    }

    fn place_child(&self, child_index: usize, pos: (usize, usize), size: (usize, usize)) {
        let Some(child) = self.child_at(child_index) else {
            return;
        };

        child.setFrame(NSRect {
            origin: NSPoint {
                x: pos.0 as CGFloat,
                y: pos.1 as CGFloat,
            },
            size: NSSize {
                width: size.0 as CGFloat,
                height: size.1 as CGFloat,
            },
        });
    }
}
