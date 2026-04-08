use jni::JavaVM;
use jni::objects::{GlobalRef, JObject};
use reactive_core::{
    BoxedComponent, Component, SetupContext, Signal,
};
use ui_core::{
    ChildStrategy, MultipleChildren, NoChild, PlatformViewBuilder,
    SingleChild,
};
use crate::ui::context::{CHILD_VIEW, CHILDREN_VIEWS};

#[derive(Clone, PartialEq, Eq)]
pub struct AndroidView {
    pub(crate) java_vm: JavaVM,
    pub(crate) global_ref: GlobalRef,
}

impl AndroidView {
    pub fn new(java_vm: JavaVM, obj: &JObject) -> Self {
        Self {
            java_vm,
            global_ref: GlobalRef::from(obj),
        }
    }

    pub fn env(&self) -> jni::JNIEnv {
        self.java_vm.get_env().unwrap()
    }

    pub fn as_obj(&self) -> &JObject {
        self.global_ref.as_obj()
    }
}

pub struct AndroidViewBuilder<W, C> {
    inner: PlatformViewBuilder<W, AndroidView, C>,
}

impl<W: Clone + PartialEq + Eq + 'static> AndroidViewBuilder<W, NoChild> {
    pub fn create_no_child(
        creator: impl FnOnce(&mut SetupContext) -> W + 'static,
        into_view: fn(W) -> AndroidView,
    ) -> Self {
        Self {
            inner: PlatformViewBuilder::create_no_child(
                creator,
                into_view,
                &CHILD_VIEW,
                &CHILDREN_VIEWS,
            ),
        }
    }
}

impl<W: Clone + PartialEq + Eq + 'static> AndroidViewBuilder<W, SingleChild> {
    pub fn create_with_child(
        creator: impl FnOnce(&mut SetupContext) -> W + 'static,
        into_view: fn(W) -> AndroidView,
        child: BoxedComponent,
    ) -> Self {
        Self {
            inner: PlatformViewBuilder::create_with_child(
                creator,
                into_view,
                &CHILD_VIEW,
                &CHILDREN_VIEWS,
                child,
            ),
        }
    }
}

impl<W: Clone + PartialEq + Eq + 'static> AndroidViewBuilder<W, MultipleChildren> {
    pub fn create_multiple_child(
        creator: impl FnOnce(&mut SetupContext) -> W + 'static,
        into_view: fn(W) -> AndroidView,
    ) -> Self {
        Self {
            inner: PlatformViewBuilder::create_multiple_child(
                creator,
                into_view,
                &CHILD_VIEW,
                &CHILDREN_VIEWS,
            ),
        }
    }

    pub fn add_child(mut self, child: BoxedComponent) -> Self {
        self.inner = self.inner.add_child(child);
        self
    }
}

impl<W: Clone + PartialEq + Eq + 'static, C> AndroidViewBuilder<W, C> {
    pub fn bind<T, ValueType>(
        mut self,
        prop: &'static ui_core::Prop<T, AndroidView, ValueType>,
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
        C: ChildStrategy,
    {
        self.inner.setup(
            ctx,
            |_| {},
            |native, layout| ui_core::ChildEntry { native, layout },
        )
    }
}

pub struct AndroidViewComponent<W, C>(pub AndroidViewBuilder<W, C>);

impl<W: Clone + PartialEq + Eq + 'static, C: ChildStrategy> Component
    for AndroidViewComponent<W, C>
{
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        self.0.setup(ctx);
    }
}
