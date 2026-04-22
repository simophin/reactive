use objc2::MainThreadMarker;
use objc2_app_kit::NSView;
use objc2_foundation::NSArray;
use reactive_core::{BoxedComponent, Component, SetupContext, Signal};
use ui_core::widgets::{
    FlexProps, FlexScope, Modifier, NativeView, WithModifier, setup_indexed_native_view_manager,
};

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

        let children_views =
            setup_indexed_native_view_manager(ctx, &super::VIEW_REGISTRY_KEY, children);

        let my_view = NativeView::new(
            |_| {
                let view = NSView::new(MainThreadMarker::new().unwrap());
                view.setTranslatesAutoresizingMaskIntoConstraints(false);
                view
            },
            |view| view,
            |_, _| {},
            modifier,
            &super::VIEW_REGISTRY_KEY,
        )
        .setup_in_component(ctx);

        ctx.create_effect(move |_, _| {
            my_view.setSubviews(&NSArray::new());
            let children_views = children_views.read();
            for (child_view, modifier) in children_views.iter().filter_map(|s| s.as_ref()) {
                my_view.addSubview(&child_view);
                //TODO: Do something with the modifier...Can we save it into the view itself?
            }
        })
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
