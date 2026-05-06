use crate::widgets::taffy::FlexTaffyContainer;
use crate::widgets::{
    CommonModifiers, FlexProps, FlexScope, Modifier, NativeView, NativeViewRegistry, SizeSpec,
    WithModifier,
};
use objc2::rc::Retained;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{NSControl, NSTextField, NSView};
use objc2_core_foundation::{CGFloat, CGPoint, CGSize};
use objc2_foundation::{NSObjectProtocol, NSRect, NSSize};
use reactive_core::{BoxedComponent, Component, ComponentId, SetupContext, Signal};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use taffy::{AvailableSpace, LayoutOutput, RequestedAxis, RunMode, Size};

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
        root_view.invalidateIntrinsicContentSize();

        tree.insert_child(view, modifier, component_id);
    }

    fn clear_view(&self, _component_id: ComponentId, view: Retained<NSView>) {
        let mut tree = self.tree.borrow_mut();
        tree.remove_child(&view);

        let root_view = tree.root_view().unwrap();
        root_view.setNeedsLayout(true);
        root_view.invalidateIntrinsicContentSize();
    }
}

struct FlexViewIvars {
    props: Cell<FlexProps>,
    tree: Rc<RefCell<ViewTree>>,
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
            self.mark_layout_dirty();
        }
    }
);

impl ReactiveFlexView {
    fn new(tree: Rc<RefCell<ViewTree>>) -> Retained<Self> {
        let this = Self::alloc(MainThreadMarker::new().unwrap()).set_ivars(FlexViewIvars {
            props: Default::default(),
            tree,
        });
        unsafe { msg_send![super(this), init] }
    }

    fn apply_props(&self, props: FlexProps) {
        if self.ivars().props.replace(props) != props {
            self.mark_layout_dirty();
        }
    }

    fn mark_layout_dirty(&self) {
        self.invalidateIntrinsicContentSize();
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

        for (n, output) in tree.iter() {
            n.setFrame(
                output
                    .map(|layout| {
                        NSRect::new(
                            CGPoint::new(layout.location.x.into(), layout.location.y.into()),
                            CGSize::new(layout.size.width.into(), layout.size.height.into()),
                        )
                    })
                    .unwrap_or_default(),
            );
        }
    }
}

pub struct Flex {
    props: Box<dyn Signal<Value = FlexProps>>,
    children: Vec<BoxedComponent>,
    modifier: Modifier,
}

fn width_for_height(v: &NSView, height: f32) -> f32 {
    if let Some(control) = v.downcast_ref::<NSControl>() {
        return control
            .sizeThatFits(NSSize::new(CGFloat::INFINITY, height.into()))
            .width as f32;
    }

    v.fittingSize().width as f32
}

fn height_for_width(v: &NSView, width: f32) -> f32 {
    if let Some(control) = v.downcast_ref::<NSControl>() {
        return control
            .sizeThatFits(NSSize::new(width.into(), CGFloat::INFINITY))
            .height as f32;
    }

    if let Some(text) = v.downcast_ref::<NSTextField>() {
        text.setPreferredMaxLayoutWidth(width.into());
    }

    v.fittingSize().height as f32
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
            |view, available_space| match (available_space.width, available_space.height) {
                (AvailableSpace::Definite(width), AvailableSpace::Definite(height)) => {
                    LayoutOutput::from_outer_size(Size { width, height })
                }
                (_, AvailableSpace::Definite(height)) => LayoutOutput::from_outer_size(Size {
                    width: height_for_width(&view, height),
                    height,
                }),

                (AvailableSpace::Definite(width), _) => LayoutOutput::from_outer_size(Size {
                    width,
                    height: width_for_height(&view, width),
                }),

                _ => {
                    let size = view.intrinsicContentSize();
                    LayoutOutput::from_outer_size(Size {
                        width: size.width as f32,
                        height: size.height as f32,
                    })
                }
            },
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
