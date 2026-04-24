use objc2::rc::Retained;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{NSView, NSViewNoIntrinsicMetric};
use objc2_foundation::{NSObjectProtocol, NSSize};
use reactive_core::{BoxedComponent, Component, ComponentId, IntoSignal, SetupContext, Signal};
use std::cell::Cell;
use std::rc::Rc;
use ui_core::widgets::taffy::TaffyTreeManager;
use ui_core::widgets::{
    FlexProps, FlexScope, Modifier, NativeView, NativeViewRegistry, WithModifier,
};

struct ViewRegistry {
    tree: TaffyTreeManager<Retained<NSView>>,
    my_view: Retained<NSView>,
}

impl NativeViewRegistry<Retained<NSView>> for ViewRegistry {
    fn update_view(&self, component_id: ComponentId, view: Retained<NSView>, modifier: Modifier) {
        self.my_view.addSubview(&view);
        self.tree.upsert_node(component_id, view, modifier);
        self.my_view.setNeedsLayout(true);
        self.my_view.invalidateIntrinsicContentSize();
    }

    fn clear_view(&self, component_id: ComponentId, view: Retained<NSView>) {
        self.tree.remove_node(component_id, view);
    }
}

struct FlexViewIvars {
    props: Cell<FlexProps>,
    taffy_tree_manager: TaffyTreeManager<Retained<NSView>>,
}

define_class!(
    #[unsafe(super(NSView))]
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
            self.layout_flex_subviews();
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
    fn new(taffy_tree_manager: TaffyTreeManager<Retained<NSView>>) -> Retained<Self> {
        let this = Self::alloc(MainThreadMarker::new().unwrap()).set_ivars(FlexViewIvars {
            props: Default::default(),
            taffy_tree_manager,
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

    fn measure_intrinsic_size(&self) -> NSSize {
        let _props = self.ivars().props.get();

        // Placeholder until Flex measurement is implemented.
        NSSize {
            width: unsafe { NSViewNoIntrinsicMetric },
            height: unsafe { NSViewNoIntrinsicMetric },
        }
    }

    fn measure_size_that_fits(&self, proposed_size: NSSize) -> NSSize {
        let _props = self.ivars().props.get();
        let _proposed_size = proposed_size;

        // Placeholder until Flex measurement is implemented.
        self.measure_intrinsic_size()
    }

    fn layout_flex_subviews(&self) {
        let _props = self.ivars().props.get();

        // Placeholder until Flex child measurement and placement is implemented.
    }
}

pub struct Flex {
    props: Box<dyn Signal<Value = FlexProps>>,
    children: Vec<BoxedComponent>,
    modifier: Modifier,
}

impl Component for Flex {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let Self {
            props,
            children,
            modifier,
        } = *self;

        let tree = TaffyTreeManager::new(ctx.scope());

        let my_view = {
            let tree = tree.clone();
            NativeView::new(
                move |_| {
                    let view = ReactiveFlexView::new(tree.clone());
                    view.setTranslatesAutoresizingMaskIntoConstraints(false);
                    view
                },
                |view: Retained<ReactiveFlexView>| view.into_super(),
                |_, _| {},
                modifier,
                &super::VIEW_REGISTRY_KEY,
            )
            .setup_in_component(ctx)
        };

        for child in children {
            let my_view = my_view.clone().into_super();
            let tree = tree.clone();
            ctx.child(move |child_ctx: &mut SetupContext| {
                let registry: Rc<dyn NativeViewRegistry<_>> =
                    Rc::new(ViewRegistry { tree, my_view });
                child_ctx.set_context(&super::VIEW_REGISTRY_KEY, registry.into_signal());
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

impl ui_core::widgets::Flex for Flex {
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
