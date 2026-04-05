use crate::context::{CHILDREN_VIEWS, CHILD_VIEW};
use apple::Prop;
use objc2::rc::Retained;
use objc2::Message;
use objc2_app_kit::{NSUserInterfaceItemIdentification, NSView};
use objc2_foundation::NSString;
use reactive_core::{BoxedComponent, Component, SetupContext, Signal};
use std::any::type_name;
use ui_core::{AtMostOneChild, MultipleChildren, NoChild, PlatformViewBuilder, SingleChild};

pub use ui_core::{
    AtMostOneChild as AtMostOneChildView, ChildStrategy as ChildViewStrategy,
    MultipleChildren as MultipleChildView, NoChild as NoChildView, SingleChild as SingleChildView,
};

pub struct AppKitViewBuilder<V, C> {
    inner: PlatformViewBuilder<Retained<V>, Retained<NSView>, C>,
    debug_identifier: Option<String>,
}

impl<V: Message + 'static> AppKitViewBuilder<V, NoChild> {
    pub fn create_no_child(
        creator: impl FnOnce(&mut SetupContext) -> Retained<V> + 'static,
        into_nsview: fn(Retained<V>) -> Retained<NSView>,
    ) -> Self {
        Self {
            inner: PlatformViewBuilder::create_no_child(
                creator,
                into_nsview,
                &CHILD_VIEW,
                &CHILDREN_VIEWS,
            ),
            debug_identifier: None,
        }
    }
}

impl<V: Message + 'static> AppKitViewBuilder<V, SingleChild> {
    pub fn create_with_child(
        creator: impl FnOnce(&mut SetupContext) -> Retained<V> + 'static,
        into_nsview: fn(Retained<V>) -> Retained<NSView>,
        child: BoxedComponent,
    ) -> Self {
        Self {
            inner: PlatformViewBuilder::create_with_child(
                creator,
                into_nsview,
                &CHILD_VIEW,
                &CHILDREN_VIEWS,
                child,
            ),
            debug_identifier: None,
        }
    }
}

impl<V: Message + 'static> AppKitViewBuilder<V, MultipleChildren> {
    pub fn create_multiple_child(
        creator: impl FnOnce(&mut SetupContext) -> Retained<V> + 'static,
        into_nsview: fn(Retained<V>) -> Retained<NSView>,
    ) -> Self {
        Self {
            inner: PlatformViewBuilder::create_multiple_child(
                creator,
                into_nsview,
                &CHILD_VIEW,
                &CHILDREN_VIEWS,
            ),
            debug_identifier: None,
        }
    }

    pub fn add_child(mut self, c: BoxedComponent) -> Self {
        self.inner = self.inner.add_child(c);
        self
    }
}

impl<V: Message + 'static> AppKitViewBuilder<V, AtMostOneChild> {
    pub fn create_with_optional_child(
        creator: impl FnOnce(&mut SetupContext) -> Retained<V> + 'static,
        into_nsview: fn(Retained<V>) -> Retained<NSView>,
        child: Option<BoxedComponent>,
    ) -> Self {
        Self {
            inner: PlatformViewBuilder::create_with_optional_child(
                creator,
                into_nsview,
                &CHILD_VIEW,
                &CHILDREN_VIEWS,
                child,
            ),
            debug_identifier: None,
        }
    }
}

impl<V: 'static, C> AppKitViewBuilder<V, C> {
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
        self.inner = self.inner.bind(prop, value);
        self
    }

    pub fn setup(self, ctx: &mut SetupContext) -> Retained<V>
    where
        V: Message,
        C: ChildViewStrategy,
    {
        let AppKitViewBuilder {
            inner,
            debug_identifier,
        } = self;
        inner.setup(
            ctx,
            |nsview| {
                nsview.setTranslatesAutoresizingMaskIntoConstraints(false);
                let name = debug_identifier.unwrap_or_else(|| short_type_name::<V>().to_string());
                NSUserInterfaceItemIdentification::setIdentifier(
                    &**nsview,
                    Some(&NSString::from_str(&name)),
                );
            },
            |native, layout| ui_core::ChildEntry { native, layout },
        )
    }
}

fn short_type_name<T>() -> &'static str {
    type_name::<T>().rsplit("::").next().unwrap_or("NSView")
}

pub struct AppKitViewComponent<V, C>(pub AppKitViewBuilder<V, C>);

impl<V: Message + 'static, C: ChildViewStrategy> Component for AppKitViewComponent<V, C> {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        self.0.setup(ctx);
    }
}
