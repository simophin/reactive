use std::sync::{Mutex, OnceLock};

use jni::objects::{GlobalRef, JObject};
use jni::{JNIEnv, JavaVM};
use reactive_core::{BoxedComponent, Component, ContextKey, SetupContext, Signal, StoredSignal};
use ui_core::{
    AtMostOneChild, ChildEntry, ChildStrategy, MultipleChildren, NoChild, PlatformViewBuilder,
    SingleChild,
};

static JAVA_VM: OnceLock<JavaVM> = OnceLock::new();
static ACTIVITY: Mutex<Option<GlobalRef>> = Mutex::new(None);

#[derive(Clone)]
pub struct AndroidView {
    pub(crate) global_ref: GlobalRef,
}

impl PartialEq for AndroidView {
    fn eq(&self, other: &Self) -> bool {
        self.global_ref.as_obj().as_raw() == other.global_ref.as_obj().as_raw()
    }
}

impl Eq for AndroidView {}

impl AndroidView {
    pub fn new(env: &mut JNIEnv<'_>, obj: &JObject<'_>) -> Self {
        Self {
            global_ref: env.new_global_ref(obj).expect("create global ref"),
        }
    }

    pub fn from_global_ref(global_ref: GlobalRef) -> Self {
        Self { global_ref }
    }

    pub fn env(&self) -> JNIEnv<'static> {
        Self::java_vm()
            .attach_current_thread_permanently()
            .expect("attach current thread to JVM")
    }

    pub fn as_obj(&self) -> &JObject<'_> {
        self.global_ref.as_obj()
    }

    pub fn java_vm() -> &'static JavaVM {
        JAVA_VM.get().expect("JAVA_VM not initialized")
    }

    pub fn set_java_vm(java_vm: JavaVM) {
        let _ = JAVA_VM.set(java_vm);
    }

    pub fn set_activity(env: &mut JNIEnv<'_>, activity: JObject<'_>) {
        let activity = env
            .new_global_ref(activity)
            .expect("create activity global ref");
        *ACTIVITY.lock().expect("lock activity") = Some(activity);
    }

    pub fn clear_activity() {
        *ACTIVITY.lock().expect("lock activity") = None;
    }

    pub fn activity() -> AndroidView {
        let global_ref = ACTIVITY
            .lock()
            .expect("lock activity")
            .as_ref()
            .expect("activity not attached")
            .clone();
        AndroidView::from_global_ref(global_ref)
    }
}

pub static CHILD_VIEW: ContextKey<StoredSignal<Option<ChildEntry<AndroidView>>>> =
    ContextKey::new();

pub static CHILDREN_VIEWS: ContextKey<Vec<StoredSignal<Option<ChildEntry<AndroidView>>>>> =
    ContextKey::new();

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

impl<W: Clone + PartialEq + Eq + 'static> AndroidViewBuilder<W, AtMostOneChild> {
    pub fn create_with_optional_child(
        creator: impl FnOnce(&mut SetupContext) -> W + 'static,
        into_view: fn(W) -> AndroidView,
        child: Option<BoxedComponent>,
    ) -> Self {
        Self {
            inner: PlatformViewBuilder::create_with_optional_child(
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
        prop: &'static ui_core::Prop<T, W, ValueType>,
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
        self.inner
            .setup(ctx, |_| {}, |native, layout, modifier| ChildEntry {
                native,
                layout,
                modifier,
            })
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
