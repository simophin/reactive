//! `Flex` as a `UIView` subclass whose `layoutSubviews` and `sizeThatFits:`
//! run the shared Taffy container. Taffy units are points.

use crate::widgets::taffy::FlexTaffyContainer;
use crate::widgets::{
    CommonFlex, CommonModifiers, Modifier, NativeView, NativeViewRegistry, SizeSpec,
};
use objc2::rc::{Retained, Weak};
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_core_foundation::{CGFloat, CGPoint, CGRect, CGSize};
use objc2_foundation::NSObjectProtocol;
use objc2_ui_kit::UIView;
use reactive_core::{Component, ComponentId, FunctionTracker, SetupContext, Signal};
use std::cell::RefCell;
use std::rc::Rc;
use taffy::prelude::TaffyMaxContent;
use taffy::{AvailableSpace, LayoutOutput, RequestedAxis, RunMode, Size};

type ViewTree = FlexTaffyContainer<Retained<UIView>>;

pub type Flex = CommonFlex<Retained<UIView>>;

/// Marks `view`'s parent for layout, and keeps going up through enclosing
/// Flex views, whose own size may depend on `view`'s. Call it when a prop
/// changes a view's natural size: unlike Auto Layout, the manual frames set
/// by Flex are not recomputed on `invalidateIntrinsicContentSize`.
pub(crate) fn invalidate_layout(view: &UIView) {
    let mut current = view.superview();
    while let Some(parent) = current {
        parent.invalidateIntrinsicContentSize();
        parent.setNeedsLayout();
        if parent.downcast_ref::<ReactiveFlexView>().is_none() {
            break;
        }
        current = parent.superview();
    }
}

struct FlexViewIvars {
    /// Weak: the tree holds this view as its root, and is itself kept alive
    /// by the component (registry and effects) until it is disposed.
    tree: std::rc::Weak<RefCell<ViewTree>>,
    tracker: RefCell<Option<FunctionTracker>>,
}

define_class!(
    // SAFETY: UIView has no subclassing requirements and this class does not
    // implement Drop.
    #[unsafe(super(UIView))]
    #[thread_kind = MainThreadOnly]
    #[name = "ReactiveFlexView"]
    #[ivars = FlexViewIvars]
    struct ReactiveFlexView;

    unsafe impl NSObjectProtocol for ReactiveFlexView {}

    impl ReactiveFlexView {
        #[unsafe(method(layoutSubviews))]
        fn layout_subviews(&self) {
            unsafe {
                let _: () = msg_send![super(self), layoutSubviews];
            }
            self.layout_flex_subviews(self.bounds().size);
        }

        #[unsafe(method(sizeThatFits:))]
        fn size_that_fits(&self, proposed: CGSize) -> CGSize {
            let output = self.measure(
                Size::NONE,
                Size {
                    width: proposed_space(proposed.width),
                    height: proposed_space(proposed.height),
                },
            );
            to_cg_size(output.size)
        }

        #[unsafe(method(intrinsicContentSize))]
        fn intrinsic_content_size(&self) -> CGSize {
            let output = self.measure(Size::NONE, Size::MAX_CONTENT);
            to_cg_size(output.size)
        }
    }
);

/// `sizeThatFits:` callers commonly pass zero or a huge value for "no limit".
fn proposed_space(value: CGFloat) -> AvailableSpace {
    if value > 0.0 && value < f32::MAX as CGFloat {
        AvailableSpace::Definite(value as f32)
    } else {
        AvailableSpace::MaxContent
    }
}

fn to_cg_size(size: Size<f32>) -> CGSize {
    CGSize::new(size.width as CGFloat, size.height as CGFloat)
}

fn fixed_size(modifier: &Modifier) -> Size<Option<f32>> {
    let fixed = |spec: SizeSpec| match spec {
        SizeSpec::Fixed(size) => Some(size as f32),
        SizeSpec::Unspecified => None,
    };
    let (width, height) = modifier.get_size().read();
    Size {
        width: fixed(width),
        height: fixed(height),
    }
}

impl ReactiveFlexView {
    fn new(tree: &Rc<RefCell<ViewTree>>, mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(FlexViewIvars {
            tree: Rc::downgrade(tree),
            tracker: Default::default(),
        });
        unsafe { msg_send![super(this), init] }
    }

    fn run_tracking<T>(&self, f: impl FnOnce() -> T) -> T {
        match self.ivars().tracker.borrow().as_ref() {
            Some(tracker) => tracker.run_tracking(f),
            None => f(),
        }
    }

    /// Computes this view's size, honouring its own fixed-size modifier.
    fn measure(
        &self,
        known_dimensions: Size<Option<f32>>,
        available_space: Size<AvailableSpace>,
    ) -> LayoutOutput {
        let Some(tree) = self.ivars().tree.upgrade() else {
            return LayoutOutput::HIDDEN;
        };
        self.run_tracking(|| {
            let mut tree = tree.borrow_mut();
            let fixed = fixed_size(tree.root_modifier().unwrap());
            tree.compute_layout(
                RunMode::ComputeSize,
                Size {
                    width: known_dimensions.width.or(fixed.width),
                    height: known_dimensions.height.or(fixed.height),
                },
                available_space,
                RequestedAxis::Both,
            )
        })
    }

    fn layout_flex_subviews(&self, size: CGSize) {
        let scale = match self.contentScaleFactor() {
            scale if scale > 0.0 => scale,
            _ => 1.0,
        };
        let snap = |v: f32| (v as CGFloat * scale).round() / scale;

        let Some(tree) = self.ivars().tree.upgrade() else {
            return;
        };

        // Collect frames first so no borrow is held while UIKit runs.
        let frames: Vec<_> = self.run_tracking(|| {
            let mut tree = tree.borrow_mut();
            let size = Size {
                width: size.width as f32,
                height: size.height as f32,
            };
            tree.compute_layout(
                RunMode::PerformLayout,
                size.map(Some),
                size.map(AvailableSpace::Definite),
                RequestedAxis::Both,
            );

            tree.iter()
                .map(|(view, layout)| {
                    let frame = layout
                        .map(|layout| {
                            // Snap edges rather than sizes to the pixel grid so
                            // adjacent children never gap or overlap.
                            let left = snap(layout.location.x);
                            let top = snap(layout.location.y);
                            let right = snap(layout.location.x + layout.size.width);
                            let bottom = snap(layout.location.y + layout.size.height);
                            CGRect::new(
                                CGPoint::new(left, top),
                                CGSize::new(right - left, bottom - top),
                            )
                        })
                        .unwrap_or_default();
                    (view.clone(), frame)
                })
                .collect()
        });

        for (view, frame) in frames {
            view.setFrame(frame);
        }
    }
}

/// Converts Taffy's proposal into a `sizeThatFits:` argument.
fn propose(space: AvailableSpace) -> CGFloat {
    match space {
        AvailableSpace::Definite(size) => size as CGFloat,
        // Zero would mean "unconstrained" to many views; a tiny width asks for
        // the narrowest layout instead (e.g. a label wrapping per word).
        AvailableSpace::MinContent => 0.5,
        AvailableSpace::MaxContent => CGFloat::MAX,
    }
}

fn size_that_fits(view: &UIView, width: CGFloat, height: CGFloat) -> CGSize {
    let size = view.sizeThatFits(CGSize::new(width, height));
    // Round up so text is never truncated by a fractional frame.
    CGSize::new(size.width.ceil(), size.height.ceil())
}

/// Measures a child: nested Flex views run Taffy directly; any other view is
/// asked through `sizeThatFits:`.
fn measure_child(
    view: &Retained<UIView>,
    known: Size<Option<f32>>,
    available: Size<AvailableSpace>,
) -> Size<f32> {
    if let Some(flex) = view.downcast_ref::<ReactiveFlexView>() {
        return flex.measure(known, available).size;
    }

    match (known.width, known.height) {
        (Some(width), Some(height)) => Size { width, height },
        (Some(width), None) => Size {
            width,
            height: size_that_fits(view, width as CGFloat, CGFloat::MAX).height as f32,
        },
        (None, Some(height)) => Size {
            width: size_that_fits(view, CGFloat::MAX, height as CGFloat).width as f32,
            height,
        },
        (None, None) => {
            let size = size_that_fits(view, propose(available.width), propose(available.height));
            Size {
                width: size.width as f32,
                height: size.height as f32,
            }
        }
    }
}

struct ViewRegistry {
    tree: Rc<RefCell<ViewTree>>,
    view: Retained<ReactiveFlexView>,
}

impl ViewRegistry {
    fn mark_dirty(&self) {
        self.view.invalidateIntrinsicContentSize();
        self.view.setNeedsLayout();
        invalidate_layout(&self.view);
    }
}

impl NativeViewRegistry<Retained<UIView>> for ViewRegistry {
    fn update_view(&self, component_id: ComponentId, view: Retained<UIView>, modifier: Modifier) {
        // Re-adding an existing subview only moves it to the front.
        self.view.addSubview(&view);
        self.tree
            .borrow_mut()
            .insert_child(view, modifier, component_id);
        self.mark_dirty();
    }

    fn clear_view(&self, _component_id: ComponentId, view: Retained<UIView>) {
        self.tree.borrow_mut().remove_child(&view);
        view.removeFromSuperview();
        self.mark_dirty();
    }
}

impl Component for Flex {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let Self {
            props,
            children,
            modifier,
            ..
        } = *self;

        let mtm = MainThreadMarker::new().expect("must be on main thread");
        let tree = Rc::new(RefCell::new(ViewTree::new(
            ctx.scope(),
            props.read(),
            measure_child,
        )));

        let view = ReactiveFlexView::new(&tree, mtm);
        tree.borrow_mut().set_root(
            view.clone().into_super(),
            modifier.clone(),
            ctx.component_id(),
        );

        NativeView::from_parts(
            {
                let view = view.clone();
                move |_| view
            },
            |view: Retained<ReactiveFlexView>| view.into_super(),
            |_, _| {},
            modifier,
            &super::VIEW_REGISTRY_KEY,
        )
        .setup_in_component(ctx);

        // Re-layout when a signal read during layout (e.g. a modifier) changes.
        let weak = Weak::from_retained(&view);
        let tracker = ctx.create_fn_tracking(move || {
            if let Some(view) = weak.load() {
                view.invalidateIntrinsicContentSize();
                view.setNeedsLayout();
                invalidate_layout(&view);
            }
        });
        view.ivars().tracker.replace(Some(tracker));

        let registry = Rc::new(ViewRegistry {
            tree: tree.clone(),
            view: view.clone(),
        });

        ctx.create_effect({
            let registry = registry.clone();
            move |_, _| {
                registry.tree.borrow_mut().set_props(props.read());
                registry.mark_dirty();
            }
        });

        for child in children {
            let registry: Rc<dyn NativeViewRegistry<_>> = registry.clone();
            ctx.child(move |child_ctx: &mut SetupContext| {
                child_ctx.set_static_context(&super::VIEW_REGISTRY_KEY, registry);
                child_ctx.boxed_child(child);
            });
        }
    }
}
