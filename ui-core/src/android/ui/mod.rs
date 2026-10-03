use super::java::JavaObject;
use crate::widgets::NativeViewRegistry;
use reactive_core::{ContextKey, SetupContext, Signal};
use std::rc::Rc;

pub mod button;
pub mod flex;
pub mod image;
pub mod label;
pub mod layout;
pub mod platform;
pub mod unsupported;
pub mod window;

pub(crate) static VIEW_REGISTRY_KEY: ContextKey<Rc<dyn NativeViewRegistry<JavaObject>>> =
    ContextKey::new();

/// The hosting `android.app.Activity`, provided at the root of the tree.
pub static ACTIVITY: ContextKey<JavaObject> = ContextKey::new();

pub(crate) fn activity(ctx: &SetupContext) -> JavaObject {
    ctx.use_context(&ACTIVITY)
        .expect("component must be set up under a ReactiveActivity")
        .read()
}

/// Creates a view whose only constructor argument is a `Context`.
pub(crate) fn new_view(ctx: &SetupContext, class: &str) -> JavaObject {
    JavaObject::construct(
        class,
        "(Landroid/content/Context;)V",
        &[jni::objects::JValue::Object(activity(ctx).as_obj())],
    )
}

/// Declares typed wrappers over [`JavaObject`] so each widget is a distinct
/// `NativeView` type, mirroring how other backends use the native view class.
macro_rules! java_view_types {
    ($($(#[$meta:meta])* $name:ident),* $(,)?) => {$(
        $(#[$meta])*
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub struct $name(pub JavaObject);

        impl std::ops::Deref for $name {
            type Target = JavaObject;

            fn deref(&self) -> &JavaObject {
                &self.0
            }
        }

        impl From<$name> for JavaObject {
            fn from(view: $name) -> JavaObject {
                view.0
            }
        }
    )*};
}

java_view_types! {
    /// `android.widget.TextView`
    TextView,
    /// `android.widget.ImageView`
    ImageView,
    /// `android.widget.Button`
    ButtonView,
    /// `com.reactive.ReactiveLayout` driven by [`flex::Flex`]
    FlexLayout,
    /// `android.widget.FrameLayout` hosting the activity content.
    ContentFrame,
    /// A plain `android.view.View` standing in for unimplemented widgets.
    PlaceholderView,
}
