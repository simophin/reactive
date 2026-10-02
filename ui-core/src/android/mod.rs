//! Android backend rendering through Jetpack Compose.
//!
//! Every widget is a small Kotlin node object (`android-lib`, package `com.reactive`) whose
//! setters write Compose snapshot state. Rust creates the nodes, binds reactive props to their
//! setters and keeps the tree structure; Compose recomposes only the nodes whose state changed.
//! Kotlin reaches back into Rust through `NativeCallback` (events), `FlexNode.nativeMeasure`
//! (layout) and `ReactiveHost` (ticks).

mod callback;
mod host;
mod logcat;
mod node;
pub mod ui;

pub use callback::Callback;
pub use host::create_host;
pub use logcat::LogcatWriter;
pub use node::{JniArg, Node, env};

#[doc(hidden)]
pub use jni;

/// Declares the Android entry point of an app library.
///
/// `setup` builds the component tree once `ReactiveHost.create()` loads the library:
///
/// ```ignore
/// ui_core::android_main!(|ctx| {
///     ctx.child(<Android as Platform>::Label::new("Hello"));
/// });
/// ```
#[macro_export]
macro_rules! android_main {
    ($setup:expr) => {
        #[unsafe(no_mangle)]
        pub extern "system" fn Java_com_reactive_ReactiveHost_nativeCreate<'local>(
            env: $crate::android::jni::JNIEnv<'local>,
            _class: $crate::android::jni::objects::JClass<'local>,
            host: $crate::android::jni::objects::JObject<'local>,
        ) -> $crate::android::jni::sys::jlong {
            $crate::android::create_host(env, host, $setup)
        }
    };
}
