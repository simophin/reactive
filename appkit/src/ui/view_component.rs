use crate::context::{CHILD_VIEW, CHILDREN_VIEWS, ChildViewEntry};
use apple::{Prop, ViewBuilder};
use objc2::Message;
use objc2::rc::Retained;
use objc2_app_kit::NSView;
use reactive_core::{BoxedComponent, Component, IntoSignal, SetupContext, Signal, StoredSignal};
use ui_core::layout::LAYOUT_HINTS;

pub trait ChildViewStrategy {
    fn into_data(self) -> Vec<BoxedComponent>;
}

pub struct NoChildView;
pub struct SingleChildView(BoxedComponent);
pub struct AtMostOneChildView(Option<BoxedComponent>);
pub struct MultipleChildView(Vec<BoxedComponent>);

impl ChildViewStrategy for NoChildView {
    fn into_data(self) -> Vec<BoxedComponent> {
        Default::default()
    }
}

impl ChildViewStrategy for SingleChildView {
    fn into_data(self) -> Vec<BoxedComponent> {
        vec![self.0]
    }
}

impl ChildViewStrategy for AtMostOneChildView {
    fn into_data(self) -> Vec<BoxedComponent> {
        if let Some(v) = self.0 {
            vec![v]
        } else {
            Default::default()
        }
    }
}

impl ChildViewStrategy for MultipleChildView {
    fn into_data(self) -> Vec<BoxedComponent> {
        self.0
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
    ) -> Self {
        Self {
            builder: ViewBuilder::new(creator),
            children: SingleChildView(child),
            into_nsview,
        }
    }
}

impl<V: Message> AppKitViewBuilder<V, MultipleChildView> {
    pub fn create_multiple_child(
        creator: impl FnOnce(&mut SetupContext) -> Retained<V> + 'static,
        into_nsview: fn(Retained<V>) -> Retained<NSView>,
    ) -> Self {
        Self {
            builder: ViewBuilder::new(creator),
            children: MultipleChildView(Vec::new()),
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
    ) -> Self {
        Self {
            builder: ViewBuilder::new(creator),
            children: AtMostOneChildView(child),
            into_nsview,
        }
    }
}

struct IndexedChild {
    child_view_entry: StoredSignal<Option<ChildViewEntry>>,
    child: BoxedComponent,
}

impl Component for IndexedChild {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let Self {
            child,
            child_view_entry,
        } = *self;

        ctx.set_context(&CHILD_VIEW, child_view_entry.into_signal());
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

        // Asked to add this view as a child
        if let Some(child_view) = ctx.use_context(&CHILD_VIEW) {
            let nsview = nsview.clone();
            let layout_hints = ctx.use_context(&LAYOUT_HINTS);
            ctx.create_effect(move |_, _| {
                child_view.read().update_if_changes(Some(ChildViewEntry {
                    view: nsview.clone(),
                    layout_hints: layout_hints.read().unwrap_or_default(),
                }));
            });
        }

        // If we have children, we'll provide the child adder through IndexedChild component
        let children = children.into_data();
        if !children.is_empty() {
            let children_views = ctx
                .provide_context(
                    &CHILDREN_VIEWS,
                    vec![ctx.create_signal(None); children.len()],
                )
                .read();

            for (child, child_view_entry) in children.into_iter().zip(children_views.into_iter()) {
                ctx.child(Box::new(IndexedChild {
                    child,
                    child_view_entry,
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
