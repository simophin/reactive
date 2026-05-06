use crate::widgets::taffy::FlexTaffyContainer;
use crate::widgets::{
    CommonModifiers, FlexProps, FlexScope, Modifier, NativeView, NativeViewRegistry, SizeSpec,
    WithModifier,
};
use objc2::rc::Retained;
use objc2::{define_class, msg_send, DefinedClass, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{NSControl, NSLayoutConstraint, NSTextField, NSView};
use objc2_core_foundation::{CGFloat, CGPoint, CGSize};
use objc2_foundation::{NSArray, NSObjectProtocol, NSRect, NSSize};
use reactive_core::{BoxedComponent, Component, ComponentId, SetupContext, Signal};
use std::cell::RefCell;
use std::rc::Rc;
use taffy::{AvailableSpace, RequestedAxis, RunMode, Size};
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
        root_view.setNeedsLayout(true);
        root_view.setNeedsUpdateConstraints(true);
        root_view.invalidateIntrinsicContentSize();
    }
}

struct FlexViewIvars {
    tree: Rc<RefCell<ViewTree>>,
    min_width_constraint: RefCell<Option<Retained<NSLayoutConstraint>>>,
    min_height_constraint: RefCell<Option<Retained<NSLayoutConstraint>>>,
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

        #[unsafe(method(updateConstraints))]
        fn update_constraints(&self) {
            self.update_min_size_constraints();
            unsafe {
                let _: () = msg_send![super(self), updateConstraints];
            }
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
            min_width_constraint: RefCell::new(None),
            min_height_constraint: RefCell::new(None),
        });
        let this: Retained<Self> = unsafe { msg_send![super(this), init] };
        this.install_min_size_constraints();
        this
    }

    fn mark_layout_dirty(&self) {
        self.invalidateIntrinsicContentSize();
        self.setNeedsUpdateConstraints(true);
        self.setNeedsLayout(true);
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
    fn measure_min_content_size(&self) -> NSSize {
        let mut tree = self.ivars().tree.borrow_mut();
        let known_dimensions = Self::get_known_dimensions(tree.root_modifier().unwrap());
        let output = tree.compute_layout(
            RunMode::ComputeSize,
            known_dimensions,
            Size {
                width: AvailableSpace::MinContent,
                height: AvailableSpace::MinContent,
            },
            RequestedAxis::Both,
        );

        NSSize {
            width: CGFloat::from(output.size.width),
            height: CGFloat::from(output.size.height),
        }
    }

    fn install_min_size_constraints(&self) {
        let min_width = self
            .widthAnchor()
            .constraintGreaterThanOrEqualToConstant(0.0);
        let min_height = self
            .heightAnchor()
            .constraintGreaterThanOrEqualToConstant(0.0);

        let constraints = NSArray::from_retained_slice(&[min_width.clone(), min_height.clone()]);
        NSLayoutConstraint::activateConstraints(&constraints);

        self.ivars().min_width_constraint.replace(Some(min_width));
        self.ivars().min_height_constraint.replace(Some(min_height));
        self.setNeedsUpdateConstraints(true);
    }

    fn update_min_size_constraints(&self) {
        let min_size = self.measure_min_content_size();

        if let Some(constraint) = self.ivars().min_width_constraint.borrow().as_ref() {
            constraint.setConstant(min_size.width);
        }

        if let Some(constraint) = self.ivars().min_height_constraint.borrow().as_ref() {
            constraint.setConstant(min_size.height);
        }
    }

    #[instrument(skip(self), ret, level = "debug")]
    fn measure_size_that_fits(&self, proposed_size: NSSize) -> NSSize {
        let mut tree = self.ivars().tree.borrow_mut();
        let known_dimensions = Self::get_known_dimensions(tree.root_modifier().unwrap());
        let output = tree.compute_layout(
            RunMode::ComputeSize,
            known_dimensions,
            Size {
                width: AvailableSpace::Definite(proposed_size.width as f32),
                height: AvailableSpace::Definite(proposed_size.height as f32),
            },
            RequestedAxis::Both,
        );

        NSSize {
            width: CGFloat::from(output.size.width),
            height: CGFloat::from(output.size.height),
        }
    }

    #[instrument(skip(self), level = "debug")]
    fn layout_flex_subviews(&self, my_size: CGSize) {
        let mut tree = self.ivars().tree.borrow_mut();
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

        for (index, (n, output)) in tree.iter().enumerate() {
            let rect = output
                .map(|layout| {
                    NSRect::new(
                        CGPoint::new(layout.location.x.into(), layout.location.y.into()),
                        CGSize::new(layout.size.width.into(), layout.size.height.into()),
                    )
                })
                .unwrap_or_default();

            tracing::debug!(?rect, index, "Layout output");
            n.setFrame(rect);
        }
    }
}

pub struct Flex {
    props: Box<dyn Signal<Value = FlexProps>>,
    children: Vec<BoxedComponent>,
    modifier: Modifier,
}

fn proposed_dimension(space: AvailableSpace) -> Option<f32> {
    match space {
        AvailableSpace::Definite(value) => Some(value),
        AvailableSpace::MinContent | AvailableSpace::MaxContent => None,
    }
}

#[instrument(skip(v), ret, level = "debug")]
fn measure_native_view(
    v: &Retained<NSView>,
    known_dimensions: Size<Option<f32>>,
    available_space: Size<AvailableSpace>,
) -> Size<f32> {
    let proposed_width = known_dimensions
        .width
        .or_else(|| proposed_dimension(available_space.width));
    let proposed_height = known_dimensions
        .height
        .or_else(|| proposed_dimension(available_space.height));

    let width = proposed_width
        .map(CGFloat::from)
        .unwrap_or(CGFloat::INFINITY);
    let height = proposed_height
        .map(CGFloat::from)
        .unwrap_or(CGFloat::INFINITY);

    if let Some(text) = v.downcast_ref::<NSTextField>() {
        let text_width = known_dimensions
            .width
            .unwrap_or(match available_space.width {
                AvailableSpace::Definite(value) => value,
                AvailableSpace::MinContent => 0.0,
                AvailableSpace::MaxContent => f32::INFINITY,
            });
        let width = CGFloat::from(text_width);
        text.setPreferredMaxLayoutWidth(width);

        let fitted = if let Some(control) = v.downcast_ref::<NSControl>() {
            control.sizeThatFits(NSSize::new(width, height))
        } else {
            v.fittingSize()
        };

        return Size {
            width: known_dimensions
                .width
                .unwrap_or(match available_space.width {
                    AvailableSpace::Definite(value) => value,
                    AvailableSpace::MinContent => 0.0,
                    AvailableSpace::MaxContent => fitted.width as f32,
                }),
            height: known_dimensions.height.unwrap_or(fitted.height as f32),
        };
    }

    let fitted = if let Some(control) = v.downcast_ref::<NSControl>() {
        control.sizeThatFits(NSSize::new(width, height))
    } else {
        v.fittingSize()
    };

    Size {
        width: known_dimensions.width.unwrap_or(fitted.width as f32),
        height: known_dimensions.height.unwrap_or(fitted.height as f32),
    }
}

impl Component for Flex {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let Self {
            props,
            children,
            modifier,
        } = *self;

        let tree = Rc::new(RefCell::new(ViewTree::new(
            ctx.scope(),
            props.read(),
            measure_native_view,
        )));

        {
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
            .setup_in_component(ctx);
        };

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

impl WithModifier for Flex {
    fn modifier(mut self, modifier: Modifier) -> Self {
        self.modifier = modifier;
        self
    }
}

impl crate::widgets::Flex for Flex {
    fn new(props: impl Signal<Value = FlexProps> + 'static) -> Self {
        Self {
            props: Box::new(props),
            children: Default::default(),
            modifier: Default::default(),
        }
    }

    fn with_child<C: Component + 'static>(mut self, factory: impl FnOnce(FlexScope) -> C) -> Self {
        self.children.push(Box::new(factory(FlexScope)));
        self
    }
}
