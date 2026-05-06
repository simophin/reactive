use crate::widgets::taffy::FlexTaffyContainer;
use crate::widgets::{
    CommonModifiers, FlexProps, FlexScope, Modifier, NativeView, NativeViewRegistry, SizeSpec,
    WithModifier,
};
use gtk4::prelude::*;
use gtk4::subclass::prelude::*;
use gtk4::{glib, graphene, gsk, Orientation, SizeRequestMode, Widget};
use reactive_core::{BoxedComponent, Component, ComponentId, SetupContext, Signal};
use std::cell::RefCell;
use std::rc::Rc;
use taffy::{AvailableSpace, RequestedAxis, RunMode, Size};
use tracing::instrument;

type ViewTree = FlexTaffyContainer<Widget>;

mod layout_imp {
    use super::*;

    #[derive(Default)]
    pub struct ReactiveFlexLayout {
        pub tree: RefCell<Option<Rc<RefCell<ViewTree>>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ReactiveFlexLayout {
        const NAME: &'static str = "ReactiveFlexLayout";
        type Type = super::ReactiveFlexLayout;
        type ParentType = gtk4::LayoutManager;
    }

    impl ObjectImpl for ReactiveFlexLayout {}

    impl LayoutManagerImpl for ReactiveFlexLayout {
        fn request_mode(&self, _widget: &Widget) -> SizeRequestMode {
            SizeRequestMode::HeightForWidth
        }

        fn measure(
            &self,
            _widget: &Widget,
            orientation: Orientation,
            for_size: i32,
        ) -> (i32, i32, i32, i32) {
            let Some(tree) = self.tree.borrow().clone() else {
                return (0, 0, -1, -1);
            };

            let minimum = measure_tree(&tree, orientation, for_size, AvailableSpace::MinContent);
            let natural = measure_tree(&tree, orientation, for_size, AvailableSpace::MaxContent);

            (minimum, natural.max(minimum), -1, -1)
        }

        fn allocate(&self, _widget: &Widget, width: i32, height: i32, _baseline: i32) {
            let Some(tree) = self.tree.borrow().clone() else {
                return;
            };

            let mut tree = tree.borrow_mut();
            tree.compute_layout(
                RunMode::PerformLayout,
                Size {
                    width: Some(width as f32),
                    height: Some(height as f32),
                },
                Size {
                    width: AvailableSpace::Definite(width as f32),
                    height: AvailableSpace::Definite(height as f32),
                },
                RequestedAxis::Both,
            );

            for (child, layout) in tree.iter() {
                let Some(layout) = layout else {
                    continue;
                };

                let transform = gsk::Transform::new()
                    .translate(&graphene::Point::new(layout.location.x, layout.location.y));
                child.allocate(
                    f32_to_i32(layout.size.width),
                    f32_to_i32(layout.size.height),
                    -1,
                    Some(transform),
                );
            }
        }
    }
}

glib::wrapper! {
    pub struct ReactiveFlexLayout(ObjectSubclass<layout_imp::ReactiveFlexLayout>)
        @extends gtk4::LayoutManager;
}

impl ReactiveFlexLayout {
    fn new(
        scope: reactive_core::ReactiveScope,
        props: FlexProps,
        modifier: Modifier,
        component_id: ComponentId,
    ) -> Self {
        let layout: Self = glib::Object::new();
        let tree = Rc::new(RefCell::new(ViewTree::new(
            scope,
            props,
            measure_native_view,
        )));
        let root: Widget = gtk4::Box::new(Orientation::Horizontal, 0).upcast();
        tree.borrow_mut().set_root(root, modifier, component_id);
        layout.imp().tree.replace(Some(tree));
        layout
    }

    fn with_tree(&self, f: impl FnOnce(&Rc<RefCell<ViewTree>>)) {
        if let Some(tree) = self.imp().tree.borrow().as_ref() {
            f(tree);
        }
    }

    fn set_props(&self, props: FlexProps) {
        self.with_tree(|tree| tree.borrow_mut().set_props(props));
        self.layout_changed();
    }

    fn insert_child(&self, component_id: ComponentId, child: Widget, modifier: Modifier) {
        self.with_tree(|tree| {
            tree.borrow_mut()
                .insert_child(child, modifier, component_id);
        });
        self.layout_changed();
    }

    fn remove_child(&self, child: &Widget) {
        self.with_tree(|tree| {
            tree.borrow_mut().remove_child(child);
        });
        self.layout_changed();
    }
}

mod view_imp {
    use super::*;

    #[derive(Default)]
    pub struct ReactiveFlexView;

    #[glib::object_subclass]
    impl ObjectSubclass for ReactiveFlexView {
        const NAME: &'static str = "ReactiveFlexView";
        type Type = super::ReactiveFlexView;
        type ParentType = Widget;
    }

    impl ObjectImpl for ReactiveFlexView {
        fn dispose(&self) {
            while let Some(child) = self.obj().first_child() {
                child.unparent();
            }
        }
    }

    impl WidgetImpl for ReactiveFlexView {}
}

glib::wrapper! {
    pub struct ReactiveFlexView(ObjectSubclass<view_imp::ReactiveFlexView>)
        @extends Widget;
}

impl ReactiveFlexView {
    fn new() -> Self {
        glib::Object::new()
    }
}

struct ViewRegistry {
    layout: ReactiveFlexLayout,
    parent: ReactiveFlexView,
}

impl NativeViewRegistry<Widget> for ViewRegistry {
    fn update_view(&self, component_id: ComponentId, view: Widget, modifier: Modifier) {
        if view.parent().as_ref() != Some(self.parent.upcast_ref()) {
            view.set_parent(&self.parent);
        }
        self.layout.insert_child(component_id, view, modifier);
        self.parent.queue_resize();
    }

    fn clear_view(&self, _component_id: ComponentId, view: Widget) {
        self.layout.remove_child(&view);
        if view.parent().as_ref() == Some(self.parent.upcast_ref()) {
            view.unparent();
        }
        self.parent.queue_resize();
    }
}

pub struct Flex {
    props: Box<dyn Signal<Value = FlexProps>>,
    children: Vec<BoxedComponent>,
    modifier: Modifier,
}

fn f32_to_i32(value: f32) -> i32 {
    if value.is_finite() {
        value.ceil().max(0.0) as i32
    } else {
        i32::MAX
    }
}

fn root_known_dimensions(tree: &ViewTree) -> Size<Option<f32>> {
    let size_spec_fixed = |spec: SizeSpec| match spec {
        SizeSpec::Fixed(s) => Some(s as f32),
        SizeSpec::Unspecified => None,
    };

    let (width, height) = tree.root_modifier().unwrap().get_size().read();

    Size {
        width: size_spec_fixed(width),
        height: size_spec_fixed(height),
    }
}

fn cross_axis_space(for_size: i32) -> AvailableSpace {
    if for_size >= 0 {
        AvailableSpace::Definite(for_size as f32)
    } else {
        AvailableSpace::MaxContent
    }
}

#[instrument(skip(tree), ret, level = "debug")]
fn measure_tree(
    tree: &Rc<RefCell<ViewTree>>,
    orientation: Orientation,
    for_size: i32,
    requested_space: AvailableSpace,
) -> i32 {
    let mut tree = tree.borrow_mut();
    let mut known_dimensions = root_known_dimensions(&tree);
    let available_space = match orientation {
        Orientation::Horizontal => {
            if for_size >= 0 {
                known_dimensions.height.get_or_insert(for_size as f32);
            }
            Size {
                width: requested_space,
                height: cross_axis_space(for_size),
            }
        }
        Orientation::Vertical => {
            if for_size >= 0 {
                known_dimensions.width.get_or_insert(for_size as f32);
            }
            Size {
                width: cross_axis_space(for_size),
                height: requested_space,
            }
        }
        _ => Size {
            width: requested_space,
            height: requested_space,
        },
    };

    let output = tree.compute_layout(
        RunMode::ComputeSize,
        known_dimensions,
        available_space,
        match orientation {
            Orientation::Horizontal => RequestedAxis::Horizontal,
            Orientation::Vertical => RequestedAxis::Vertical,
            _ => RequestedAxis::Both,
        },
    );

    match orientation {
        Orientation::Horizontal => f32_to_i32(output.size.width),
        Orientation::Vertical => f32_to_i32(output.size.height),
        _ => f32_to_i32(output.size.width.max(output.size.height)),
    }
}

fn measure_axis(
    view: &Widget,
    orientation: Orientation,
    for_size: Option<f32>,
    available_space: AvailableSpace,
) -> f32 {
    let (minimum, natural, _, _) =
        view.measure(orientation, for_size.map(f32_to_i32).unwrap_or(-1));
    match available_space {
        AvailableSpace::MinContent => minimum as f32,
        AvailableSpace::MaxContent => natural as f32,
        AvailableSpace::Definite(value) => (natural as f32).min(value).max(minimum as f32),
    }
}

#[instrument(skip(view), ret, level = "debug")]
fn measure_native_view(
    view: &Widget,
    known_dimensions: Size<Option<f32>>,
    available_space: Size<AvailableSpace>,
) -> Size<f32> {
    let width = known_dimensions.width.unwrap_or_else(|| {
        measure_axis(
            view,
            Orientation::Horizontal,
            known_dimensions.height.or(match available_space.height {
                AvailableSpace::Definite(value) => Some(value),
                AvailableSpace::MinContent | AvailableSpace::MaxContent => None,
            }),
            available_space.width,
        )
    });

    let height = known_dimensions.height.unwrap_or_else(|| {
        measure_axis(
            view,
            Orientation::Vertical,
            Some(width),
            available_space.height,
        )
    });

    Size { width, height }
}

impl Component for Flex {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let Self {
            props,
            children,
            modifier,
        } = *self;

        let flex_view = ReactiveFlexView::new();
        let layout = ReactiveFlexLayout::new(
            ctx.scope(),
            props.read(),
            modifier.clone(),
            ctx.component_id(),
        );
        flex_view.set_layout_manager(Some(layout.clone()));

        NativeView::new(
            {
                let flex_view = flex_view.clone();
                move |_| flex_view
            },
            |view| view.upcast(),
            |_, _| {},
            modifier,
            &super::VIEW_REGISTRY_KEY,
        )
        .setup_in_component(ctx);

        ctx.create_effect({
            let layout = layout.clone();
            let flex_view = flex_view.clone();
            move |_, _| {
                layout.set_props(props.read());
                flex_view.queue_resize();
            }
        });

        for child in children {
            let registry: Rc<dyn NativeViewRegistry<_>> = Rc::new(ViewRegistry {
                layout: layout.clone(),
                parent: flex_view.clone(),
            });
            ctx.child(move |child_ctx: &mut SetupContext| {
                child_ctx.set_static_context(&super::VIEW_REGISTRY_KEY, registry);
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
