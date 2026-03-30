use crate::context::{CHILD_VIEW, CHILDREN_VIEWS, ChildViewEntry};
use apple::Prop;
use objc2::Message;
use objc2::rc::Retained;
use objc2_app_kit::{NSUserInterfaceItemIdentification, NSView};
use objc2_foundation::NSString;
use reactive_core::{BoxedComponent, Component, IntoSignal, SetupContext, Signal, StoredSignal};
use std::any::type_name;
use ui_core::ViewBuilder;
use ui_core::layout::{
    BOX_MODIFIERS, BoxModifierChain, ChildLayoutInfo, FLEX_PARENT_DATA, FlexParentData,
};

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
    builder: ViewBuilder<Retained<V>>,
    children: C,
    into_nsview: fn(Retained<V>) -> Retained<NSView>,
    debug_identifier: Option<String>,
}

impl<V: Message + 'static> AppKitViewBuilder<V, NoChildView> {
    pub fn create_no_child(
        creator: impl FnOnce(&mut SetupContext) -> Retained<V> + 'static,
        into_nsview: fn(Retained<V>) -> Retained<NSView>,
    ) -> Self {
        Self {
            builder: ViewBuilder::new(creator),
            children: NoChildView,
            into_nsview,
            debug_identifier: None,
        }
    }
}

impl<V: Message + 'static> AppKitViewBuilder<V, SingleChildView> {
    pub fn create_with_child(
        creator: impl FnOnce(&mut SetupContext) -> Retained<V> + 'static,
        into_nsview: fn(Retained<V>) -> Retained<NSView>,
        child: BoxedComponent,
    ) -> Self {
        Self {
            builder: ViewBuilder::new(creator),
            children: SingleChildView(child),
            into_nsview,
            debug_identifier: None,
        }
    }
}

impl<V: Message + 'static> AppKitViewBuilder<V, MultipleChildView> {
    pub fn create_multiple_child(
        creator: impl FnOnce(&mut SetupContext) -> Retained<V> + 'static,
        into_nsview: fn(Retained<V>) -> Retained<NSView>,
    ) -> Self {
        Self {
            builder: ViewBuilder::new(creator),
            children: MultipleChildView(Vec::new()),
            into_nsview,
            debug_identifier: None,
        }
    }

    pub fn add_child(mut self, c: BoxedComponent) -> Self {
        self.children.0.push(c);
        self
    }
}

impl<V: Message + 'static> AppKitViewBuilder<V, AtMostOneChildView> {
    pub fn create_with_optional_child(
        creator: impl FnOnce(&mut SetupContext) -> Retained<V> + 'static,
        into_nsview: fn(Retained<V>) -> Retained<NSView>,
        child: Option<BoxedComponent>,
    ) -> Self {
        Self {
            builder: ViewBuilder::new(creator),
            children: AtMostOneChildView(child),
            into_nsview,
            debug_identifier: None,
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
        ctx.boxed_child(child);
    }
}

impl<V: 'static, Children> AppKitViewBuilder<V, Children> {
    pub fn debug_identifier(mut self, identifier: impl Into<String>) -> Self {
        self.debug_identifier = Some(identifier.into());
        self
    }

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

    pub fn setup(self, ctx: &mut SetupContext) -> Retained<V>
    where
        V: Message,
        Children: ChildViewStrategy,
    {
        let AppKitViewBuilder {
            builder,
            children,
            into_nsview,
            debug_identifier,
        } = self;
        let view = builder.setup(ctx);
        let nsview = into_nsview(view.clone());
        nsview.setTranslatesAutoresizingMaskIntoConstraints(false);
        let debug_identifier =
            debug_identifier.unwrap_or_else(|| short_type_name::<V>().to_string());
        NSUserInterfaceItemIdentification::setIdentifier(
            &*nsview,
            Some(&NSString::from_str(&debug_identifier)),
        );

        if let Some(child_view) = ctx.use_context(&CHILD_VIEW) {
            let nsview = nsview.clone();
            let box_modifiers = ctx.use_context(&BOX_MODIFIERS);
            let flex_parent_data = ctx.use_context(&FLEX_PARENT_DATA);
            ctx.create_effect(move |_, _| {
                child_view.read().update_if_changes(Some(ChildViewEntry {
                    view: nsview.clone(),
                    layout: ChildLayoutInfo {
                        box_modifiers: box_modifiers.read().unwrap_or_default(),
                        flex: flex_parent_data.read().unwrap_or_default(),
                    },
                }));
            });
        }

        ctx.set_context(&BOX_MODIFIERS, BoxModifierChain::default().into_signal());
        ctx.set_context(&FLEX_PARENT_DATA, FlexParentData::default().into_signal());

        let children = children.into_data();
        if !children.is_empty() {
            let children_views_signal = ctx.provide_context(
                &CHILDREN_VIEWS,
                (0..children.len())
                    .map(|_| ctx.create_signal(None))
                    .collect::<Vec<_>>(),
            );
            let children_views = children_views_signal.read();

            for (child, child_view_entry) in children.into_iter().zip(children_views.into_iter()) {
                ctx.boxed_child(Box::new(IndexedChild {
                    child,
                    child_view_entry,
                }));
            }
        }

        view
    }
}

fn short_type_name<T>() -> &'static str {
    type_name::<T>().rsplit("::").next().unwrap_or("NSView")
}

pub struct AppKitViewComponent<V, C>(pub AppKitViewBuilder<V, C>);

impl<V: Message + 'static, C> Component for AppKitViewComponent<V, C>
where
    V: Message,
    C: ChildViewStrategy,
{
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        self.0.setup(ctx);
    }
}
