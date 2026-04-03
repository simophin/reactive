use crate::context::{CHILD_WIDGET, CHILDREN_WIDGETS};
use reactive_core::{BoxedComponent, Component, SetupContext, Signal};
use ui_core::Prop;
use ui_core::{MultipleChildren, NoChild, PlatformViewBuilder, SingleChild};

pub use ui_core::{
    ChildStrategy as ChildWidgetStrategy, MultipleChildren as MultipleChildWidget,
    NoChild as NoChildWidget, SingleChild as SingleChildWidget,
};

pub struct GtkViewBuilder<W, C> {
    inner: PlatformViewBuilder<W, gtk4::Widget, C>,
}

impl<W: Clone + PartialEq + Eq + 'static> GtkViewBuilder<W, NoChild> {
    pub fn create_no_child(
        creator: impl FnOnce(&mut SetupContext) -> W + 'static,
        into_widget: fn(W) -> gtk4::Widget,
    ) -> Self {
        Self {
            inner: PlatformViewBuilder::create_no_child(
                creator,
                into_widget,
                &CHILD_WIDGET,
                &CHILDREN_WIDGETS,
            ),
        }
    }
}

impl<W: Clone + PartialEq + Eq + 'static> GtkViewBuilder<W, SingleChild> {
    pub fn create_with_child(
        creator: impl FnOnce(&mut SetupContext) -> W + 'static,
        into_widget: fn(W) -> gtk4::Widget,
        child: BoxedComponent,
    ) -> Self {
        Self {
            inner: PlatformViewBuilder::create_with_child(
                creator,
                into_widget,
                &CHILD_WIDGET,
                &CHILDREN_WIDGETS,
                child,
            ),
        }
    }
}

impl<W: Clone + PartialEq + Eq + 'static> GtkViewBuilder<W, MultipleChildren> {
    pub fn create_multiple_child(
        creator: impl FnOnce(&mut SetupContext) -> W + 'static,
        into_widget: fn(W) -> gtk4::Widget,
    ) -> Self {
        Self {
            inner: PlatformViewBuilder::create_multiple_child(
                creator,
                into_widget,
                &CHILD_WIDGET,
                &CHILDREN_WIDGETS,
            ),
        }
    }

    pub fn add_child(mut self, child: BoxedComponent) -> Self {
        self.inner = self.inner.add_child(child);
        self
    }
}

impl<W: Clone + PartialEq + Eq + 'static, C> GtkViewBuilder<W, C> {
    pub fn bind<T, ValueType>(
        mut self,
        prop: &'static Prop<T, W, ValueType>,
        value: impl Signal<Value = ValueType> + 'static,
    ) -> Self
    where
        ValueType: 'static,
    {
        self.inner = self.inner.bind(prop, value);
        self
    }

    pub fn setup(self, ctx: &mut SetupContext) -> W
    where
        C: ChildWidgetStrategy,
    {
        self.inner.setup(ctx, |_| {})
    }
}

pub struct GtkViewComponent<W, C>(pub GtkViewBuilder<W, C>);

impl<W: Clone + PartialEq + Eq + 'static, C: ChildWidgetStrategy> Component
    for GtkViewComponent<W, C>
{
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        self.0.setup(ctx);
    }
}
