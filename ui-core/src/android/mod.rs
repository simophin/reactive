//! Android backend built on the classic `android.view.View` system.
//!
//! Rust owns real `View` objects through JNI. The only Java support code lives
//! in `android-lib`: `ReactiveActivity` (lifecycle), `ReactiveLayout`
//! (a `ViewGroup` whose measure/layout run in Rust, e.g. Flex) and `NativeCallback`
//! (listener trampolines).

mod app;
mod callback;
mod java;
pub mod ui;

pub use app::on_load;
pub use java::JavaObject;
pub use ui::*;

#[doc(hidden)]
pub use jni as __jni;

/// Declares the app's entry point. Expands to `JNI_OnLoad`, which registers
/// the backend's native methods and remembers `setup` for every
/// `ReactiveActivity` that starts.
///
/// ```ignore
/// fn app(ctx: &mut SetupContext) { /* build components */ }
/// ui_core::android_main!(app);
/// ```
#[macro_export]
macro_rules! android_main {
    ($setup:expr) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "system" fn JNI_OnLoad(
            vm: *mut $crate::android::__jni::sys::JavaVM,
            _reserved: *mut ::std::ffi::c_void,
        ) -> $crate::android::__jni::sys::jint {
            unsafe { $crate::android::on_load(vm, $setup) }
        }
    };
}
