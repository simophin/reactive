use crate::widgets::taffy::FlexTaffyContainer;
use crate::widgets::{
    CommonFlex, CommonModifiers, FlexScope, Modifier, NativeView, NativeViewRegistry, SizeSpec,
    WithModifier,
};
use objc2::rc::{Retained, Weak};
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{NSControl, NSTextField, NSView};
use objc2_core_foundation::{CGFloat, CGPoint, CGSize};
use objc2_foundation::{NSObjectNSScriptClassDescription, NSObjectProtocol, NSRect, NSSize};
use reactive_core::{Component, ComponentId, FunctionTracker, SetupContext, Signal};
use std::cell::RefCell;
use std::rc::Rc;
use taffy::{AvailableSpace, LayoutOutput, RequestedAxis, RunMode, Size};
use tracing::instrument;

type ViewTree = FlexTaffyContainer<Retained<NSView>>;

struct ViewRegistry {
    tree: Rc<RefCell<ViewTree>>,
}

impl NativeViewRegistry<Retained<NSView>> for ViewRegistry {
    fn update_view(&self, component_id: ComponentId, view: Retained<NSView>, modifier: Modifier) {
        let mut tree = self.tree.borrow_mut();
        let root_view = tree.root_view().unwrap();
        root_view.addSubview(&view);
        root_view.setNeedsLayout(true);
        root_view.setNeedsUpdateConstraints(true);
        root_view.invalidateIntrinsicContentSize();

        tree.insert_child(view, modifier, component_id);
    }

    fn clear_view(&self, _component_id: ComponentId, view: Retained<NSView>) {
        let mut tree = self.tree.borrow_mut();
        tree.remove_child(&view);

        let root_view = tree.root_view().unwrap();
        view.removeFromSuperview();
        root_view.setNeedsLayout(true);
        root_view.setNeedsUpdateConstraints(true);
        root_view.invalidateIntrinsicContentSize();
    }
}

struct FlexViewIvars {
    tree: Rc<RefCell<ViewTree>>,
    tracker: RefCell<Option<FunctionTracker>>,
}

define_class!(
    #[unsafe(super(NSControl))]
    #[thread_kind = MainThreadOnly]
    #[name = "ReactiveFlexView"]
    #[ivars = FlexViewIvars]
    struct ReactiveFlexView;

    unsafe impl NSObjectProtocol for ReactiveFlexView {}

    impl ReactiveFlexView {
        #[unsafe(method(layout))]
        fn layout(&self) {
            unsafe {
                let _: () = msg_send![super(self), layout];
            }
            self.layout_flex_subviews(self.frame().size);
        }

        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> bool {
            true
        }

        #[unsafe(method(intrinsicContentSize))]
        fn intrinsic_content_size(&self) -> NSSize {
            self.measure_intrinsic_size()
        }

        #[unsafe(method(sizeThatFits:))]
        fn size_that_fits(&self, proposed_size: NSSize) -> NSSize {
            self.measure_size_that_fits(proposed_size)
        }

        #[unsafe(method(setFrameSize:))]
        fn set_frame_size(&self, new_size: NSSize) {
            unsafe {
                let _: () = msg_send![super(self), setFrameSize: new_size];
            }
            self.ivars().tree.borrow_mut().clear_cache();
            self.mark_layout_dirty();
        }
    }
);

impl ReactiveFlexView {
    fn new(tree: Rc<RefCell<ViewTree>>) -> Retained<Self> {
        let this = Self::alloc(MainThreadMarker::new().unwrap()).set_ivars(FlexViewIvars {
            tree,
            tracker: Default::default(),
        });
        let this: Retained<Self> = unsafe { msg_send![super(this), init] };
        this
    }

    fn mark_layout_dirty(&self) {
        self.invalidateIntrinsicContentSize();
        self.setNeedsUpdateConstraints(true);
        self.setNeedsLayout(true);
    }

    fn set_layout_tracker(&self, tracker: FunctionTracker) {
        self.ivars().tracker.borrow_mut().replace(tracker);
    }

    fn get_known_dimensions(modifier: &Modifier) -> Size<Option<f32>> {
        let size_spec_fixed = |spec: SizeSpec| match spec {
            SizeSpec::Fixed(s) => Some(s as f32),
            SizeSpec::Unspecified => None,
        };

        let (width, height) = modifier.get_size().read();

        Size {
            width: size_spec_fixed(width),
            height: size_spec_fixed(height),
        }
    }

    #[instrument(skip(self), ret, level = "debug")]
    fn measure_intrinsic_size(&self) -> NSSize {
        let mut tree = self.ivars().tree.borrow_mut();
        let known_dimensions = Self::get_known_dimensions(tree.root_modifier().unwrap());
        let output = tree.compute_layout(
            RunMode::ComputeSize,
            known_dimensions,
            Size {
                width: AvailableSpace::MaxContent,
                height: AvailableSpace::MaxContent,
            },
            RequestedAxis::Both,
        );

        NSSize {
            width: CGFloat::from(output.size.width),
            height: CGFloat::from(output.size.height),
        }
    }

    #[instrument(skip(self), ret, level = "debug")]
    fn measure_size_that_fits(&self, proposed_size: NSSize) -> NSSize {
        let mut tree = self.ivars().tree.borrow_mut();
        let known_dimensions = Self::get_known_dimensions(tree.root_modifier().unwrap());
        let output = self
            .ivars()
            .tracker
            .borrow()
            .as_ref()
            .unwrap()
            .run_tracking(|| {
                tree.compute_layout(
                    RunMode::ComputeSize,
                    known_dimensions,
                    Size {
                        width: AvailableSpace::Definite(proposed_size.width as f32),
                        height: AvailableSpace::Definite(proposed_size.height as f32),
                    },
                    RequestedAxis::Both,
                )
            });

        NSSize {
            width: CGFloat::from(output.size.width),
            height: CGFloat::from(output.size.height),
        }
    }

    #[instrument(skip(self), level = "debug")]
    fn layout_flex_subviews(&self, my_size: CGSize) {
        let mut tree = self.ivars().tree.borrow_mut();
        self.ivars()
            .tracker
            .borrow()
            .as_ref()
            .unwrap()
            .run_tracking(|| {
                tree.compute_layout(
                    RunMode::PerformLayout,
                    Size {
                        width: Some(my_size.width as f32),
                        height: Some(my_size.height as f32),
                    },
                    Size {
                        width: AvailableSpace::Definite(my_size.width as f32),
                        height: AvailableSpace::Definite(my_size.height as f32),
                    },
                    RequestedAxis::Both,
                );
            });

        for (n, output) in tree.iter() {
            let rect = output
                .map(|layout| {
                    NSRect::new(
                        CGPoint::new(layout.location.x.into(), layout.location.y.into()),
                        CGSize::new(layout.size.width.into(), layout.size.height.into()),
                    )
                })
                .unwrap_or_default();

            tracing::debug!(?rect, view = view_debug_name(n), "Layout output");
            n.setFrame(rect);
        }
    }

    fn measure(
        &self,
        known_dimensions: Size<Option<f32>>,
        available_space: Size<AvailableSpace>,
    ) -> LayoutOutput {
        let mut tree = self.ivars().tree.borrow_mut();
        tree.compute_layout(
            RunMode::ComputeSize,
            known_dimensions,
            available_space,
            RequestedAxis::Both,
        )
    }
}

pub type Flex = CommonFlex<Retained<NSView>>;

fn propose_size(v: AvailableSpace) -> CGFloat {
    match v {
        AvailableSpace::Definite(width) => width as CGFloat,
        AvailableSpace::MinContent => 0.5, // 0 is special - it means no constraint, so we use a very small number to get the min content size
        AvailableSpace::MaxContent => CGFloat::INFINITY,
    }
}

fn view_debug_name(v: &Retained<NSView>) -> String {
    if let Some(v) = v.downcast_ref::<NSTextField>() {
        format!("{}(text = {})", v.className(), v.stringValue())
    } else {
        v.className().to_string()
    }
}

#[instrument(skip(v), ret, level = "debug", fields(view = view_debug_name(v)))]
fn measure_native_view(
    v: &Retained<NSView>,
    known_dimensions: Size<Option<f32>>,
    available_space: Size<AvailableSpace>,
) -> Size<f32> {
    // If we are measuring our own, just use measure
    if let Some(v) = v.downcast_ref::<ReactiveFlexView>() {
        return v.measure(known_dimensions, available_space).size;
    }

    match (
        known_dimensions.width,
        known_dimensions.height,
        available_space.width,
        available_space.height,
    ) {
        (Some(width), Some(height), _, _) => Size { width, height },

        // Given no fixed size, want to know natural size
        (None, None, available_width, available_height) => size_that_fits(
            v,
            propose_size(available_width),
            propose_size(available_height),
        ),

        // Given a fixed width, and wanting to know a height
        (Some(width), None, _, _) => {
            let height = size_that_fits(v, width.into(), CGFloat::INFINITY).height;
            Size { width, height }
        }

        // Given a fixed height, wanting to know a width (this should be rare)
        (None, Some(height), _, _) => {
            let width = size_that_fits(v, CGFloat::INFINITY, height.into()).width;
            Size { width, height }
        }
    }
}

fn size_that_fits(v: &NSView, width: CGFloat, height: CGFloat) -> Size<f32> {
    let size = if let Some(control) = v.downcast_ref::<NSControl>() {
        control.sizeThatFits(NSSize::new(width, height))
    } else {
        v.fittingSize()
    };

    Size {
        width: size.width as f32,
        height: size.height as f32,
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

        let tree = Rc::new(RefCell::new(ViewTree::new(
            ctx.scope(),
            props.read(),
            measure_native_view,
        )));

        let my_view = {
            let tree = tree.clone();
            let component_id = ctx.component_id();
            NativeView::new(
                {
                    let modifier = modifier.clone();
                    move |_| {
                        let view = ReactiveFlexView::new(tree.clone());
                        tree.borrow_mut().set_root(
                            view.clone().into_super().into_super(),
                            modifier,
                            component_id,
                        );
                        view.setTranslatesAutoresizingMaskIntoConstraints(false);
                        view
                    }
                },
                |view: Retained<ReactiveFlexView>| view.into_super().into_super(),
                |_, _| {},
                modifier,
                &super::VIEW_REGISTRY_KEY,
            )
            .setup_in_component(ctx)
        };

        let my_view_weak = Weak::from_retained(&my_view);
        let tracker = ctx.create_fn_tracking(move || {
            if let Some(my_view) = my_view_weak.load() {
                my_view.setNeedsLayout(true);
                my_view.invalidateIntrinsicContentSize();
            }
        });
        my_view.set_layout_tracker(tracker);

        ctx.create_effect({
            let tree = tree.clone();
            move |_, _| {
                let props = props.read();
                let root_view = tree.borrow().root_view().cloned();
                tree.borrow_mut().set_props(props);
                if let Some(root_view) = root_view {
                    root_view.invalidateIntrinsicContentSize();
                    root_view.setNeedsUpdateConstraints(true);
                    root_view.setNeedsLayout(true);
                }
            }
        });

        for child in children {
            let tree = tree.clone();
            ctx.child(move |child_ctx: &mut SetupContext| {
                child_ctx
                    .set_static_context(&super::VIEW_REGISTRY_KEY, Rc::new(ViewRegistry { tree }));
                child_ctx.boxed_child(child);
            });
        }
    }
}
