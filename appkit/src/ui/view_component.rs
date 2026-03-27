use crate::context::CHILD_ADDER;
use apple::{Prop, ViewBuilder};
use objc2::Message;
use objc2::rc::Retained;
use objc2_app_kit::NSView;
use reactive_core::{BoxedComponent, Component, SetupContext, Signal, SignalExt};
use std::rc::Rc;
use ui_core::layout::{LAYOUT_HINTS, LayoutHints};

//parent_view, child_index, child
pub type OnSetChildView =
    Rc<dyn Fn(&Retained<NSView>, usize, Retained<NSView>, Rc<dyn Signal<Value = LayoutHints>>)>;

pub trait ChildViewStrategy {
    fn into_data(self) -> Option<(OnSetChildView, Vec<BoxedComponent>)>;
}

pub trait MayHaveChild {}

pub struct NoChildView;
pub struct SingleChildView(BoxedComponent, OnSetChildView);
pub struct AtMostOneChildView(Option<BoxedComponent>, OnSetChildView);
pub struct MultipleChildView(Vec<BoxedComponent>, OnSetChildView);

impl MayHaveChild for SingleChildView {}
impl MayHaveChild for AtMostOneChildView {}
impl MayHaveChild for MultipleChildView {}

impl ChildViewStrategy for NoChildView {
    fn into_data(self) -> Option<(OnSetChildView, Vec<BoxedComponent>)> {
        None
    }
}

impl ChildViewStrategy for SingleChildView {
    fn into_data(self) -> Option<(OnSetChildView, Vec<BoxedComponent>)> {
        Some((self.1, vec![self.0]))
    }
}

impl ChildViewStrategy for AtMostOneChildView {
    fn into_data(self) -> Option<(OnSetChildView, Vec<BoxedComponent>)> {
        if let Some(v) = self.0 {
            Some((self.1, vec![v]))
        } else {
            Some((self.1, Default::default()))
        }
    }
}

impl ChildViewStrategy for MultipleChildView {
    fn into_data(self) -> Option<(OnSetChildView, Vec<BoxedComponent>)> {
        Some((self.1, self.0))
    }
}

pub struct AppKitViewBuilder<V, C> {
    builder: ViewBuilder<V>,
    children: C,
    into_nsview: fn(Retained<V>) -> Retained<NSView>,
}

impl<V: Message> AppKitViewBuilder<V, NoChildView> {
    pub fn create_no_child(
        creator: impl FnOnce(&mut SetupContext) -> Retained<V> + 'static,
        into_nsview: fn(Retained<V>) -> Retained<NSView>,
    ) -> Self {
        Self {
            builder: ViewBuilder::new(creator),
            children: NoChildView,
            into_nsview,
        }
    }
}

impl<V: Message> AppKitViewBuilder<V, SingleChildView> {
    pub fn create_with_child(
        creator: impl FnOnce(&mut SetupContext) -> Retained<V> + 'static,
        into_nsview: fn(Retained<V>) -> Retained<NSView>,
        child: BoxedComponent,
        on_set_child_view: OnSetChildView,
    ) -> Self {
        Self {
            builder: ViewBuilder::new(creator),
            children: SingleChildView(child, on_set_child_view),
            into_nsview,
        }
    }
}

impl<V: Message> AppKitViewBuilder<V, MultipleChildView> {
    pub fn create_multiple_child(
        creator: impl FnOnce(&mut SetupContext) -> Retained<V> + 'static,
        into_nsview: fn(Retained<V>) -> Retained<NSView>,
        on_set_child_view: OnSetChildView,
    ) -> Self {
        Self {
            builder: ViewBuilder::new(creator),
            children: MultipleChildView(Vec::new(), on_set_child_view),
            into_nsview,
        }
    }

    pub fn add_child(mut self, c: BoxedComponent) -> Self {
        self.children.0.push(c);
        self
    }
}

impl<V: Message> AppKitViewBuilder<V, AtMostOneChildView> {
    pub fn create_with_optional_child(
        creator: impl FnOnce(&mut SetupContext) -> Retained<V> + 'static,
        into_nsview: fn(Retained<V>) -> Retained<NSView>,
        child: Option<BoxedComponent>,
        on_set_child_view: OnSetChildView,
    ) -> Self {
        Self {
            builder: ViewBuilder::new(creator),
            children: AtMostOneChildView(child, on_set_child_view),
            into_nsview,
        }
    }
}

struct IndexedChild<C> {
    index: usize,
    parent: Retained<NSView>,
    child: C,
    on_set_child: OnSetChildView,
}

impl Component for IndexedChild<BoxedComponent> {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let Self {
            index,
            child,
            on_set_child: on_set_child,
            parent,
        } = *self;

        ctx.set_context(&CHILD_ADDER, move || {
            let on_set_child = on_set_child.clone();
            let parent = parent.clone();
            Rc::new(move |child, hints| on_set_child(&parent, index, child, hints))
                as Rc<dyn Fn(_, _)>
        });

        ctx.child(child);
    }
}

impl<V, Children> AppKitViewBuilder<V, Children> {
    pub fn bind<T, ValueType>(
        mut self,
        prop: &'static Prop<T, V, ValueType>,
        value: impl Signal<Value = ValueType> + 'static,
    ) -> Self
    where
        V: Message,
        ValueType: 'static,
    {
        self.builder.bind(prop, value);
        self
    }

    pub fn setup(self, ctx: &mut SetupContext)
    where
        V: Message,
        Children: ChildViewStrategy,
    {
        let AppKitViewBuilder {
            builder,
            children,
            into_nsview,
        } = self;
        let view = builder.setup(ctx);
        let nsview = into_nsview(view);
        nsview.setTranslatesAutoresizingMaskIntoConstraints(false);

        // Asked to add this view as a child of something
        if let Some(adder) = ctx.use_context(&CHILD_ADDER) {
            adder.read()(
                nsview.clone(),
                Rc::new(
                    ctx.use_context(&LAYOUT_HINTS)
                        .map_value(|h| h.unwrap_or_default()),
                ),
            );
        }

        // If we have children, we'll provide the child adder through IndexedChild component
        if let Some((on_set_child, children)) = children.into_data() {
            for (index, child) in children.into_iter().enumerate() {
                ctx.child(Box::new(IndexedChild {
                    index,
                    child,
                    on_set_child: on_set_child.clone(),
                    parent: nsview.clone(),
                }));
            }
        }
    }
}

pub struct AppKitViewComponent<V, C>(pub AppKitViewBuilder<V, C>);

impl<V: Message, C> Component for AppKitViewComponent<V, C>
where
    V: Message,
    C: ChildViewStrategy,
{
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        self.0.setup(ctx);
    }
}
