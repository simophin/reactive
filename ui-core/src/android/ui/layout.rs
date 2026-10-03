//! Custom layouts backed by `com.reactive.ReactiveLayout`, a `ViewGroup`
//! whose `onMeasure`/`onLayout` call into a Rust [`ViewGroupLayout`].
//!
//! The Java class knows nothing about any layout algorithm. A container
//! creates the view with [`new_layout_view`], adds its children with
//! `addView`, and [`attach`]es an object that measures and places them.
//! [`Flex`](super::flex::Flex) is one such layout.

use crate::android::JavaObject;
use crate::android::app::register;
use jni::JNIEnv;
use jni::objects::{JClass, JValue};
use jni::sys::{jint, jlong};
use reactive_core::SetupContext;
use std::ffi::c_void;
use std::rc::Rc;

const CLASS: &str = "com/reactive/ReactiveLayout";

/// Measure and layout passes of a `ReactiveLayout`, all values in pixels.
pub trait ViewGroupLayout {
    /// `onMeasure`: returns the measured `(width, height)`. The result should
    /// honour both specs, e.g. through [`resolve_size`].
    fn measure(&self, width_spec: i32, height_spec: i32) -> (i32, i32);

    /// `onLayout`: positions the children within a `width` × `height` box,
    /// e.g. through [`measure_view`] and [`layout_view`].
    fn layout(&self, width: i32, height: i32);
}

/// Creates a `ReactiveLayout` with no layout attached yet. Until one is
/// attached it measures as small as its specs allow and places nothing.
pub fn new_layout_view(ctx: &SetupContext) -> JavaObject {
    super::new_view(ctx, CLASS)
}

/// Makes `layout` drive the measure and layout passes of `view`, a view from
/// [`new_layout_view`], until the component of `ctx` is disposed.
pub fn attach(ctx: &SetupContext, view: &JavaObject, layout: Rc<dyn ViewGroupLayout>) {
    // A thin pointer to the fat `Rc<dyn _>` fits the Java `long` field.
    let handle = Box::into_raw(Box::new(layout));
    view.set_long_field("nativeHandle", handle as jlong);
    ctx.on_cleanup({
        let view = view.clone();
        move || {
            view.set_long_field("nativeHandle", 0);
            drop(unsafe { Box::from_raw(handle) });
        }
    });
}

pub(crate) fn register_natives(env: &mut JNIEnv) {
    register(
        env,
        CLASS,
        &[
            ("nativeMeasure", "(JII)J", native_measure as *mut c_void),
            ("nativeLayout", "(JII)V", native_layout as *mut c_void),
        ],
    );
}

/// The `handle` must come from [`attach`] and not yet be cleaned up; the Java
/// side only calls with a non-zero handle.
unsafe fn layout_from_handle<'a>(handle: jlong) -> &'a dyn ViewGroupLayout {
    unsafe { &**(handle as *const Rc<dyn ViewGroupLayout>) }
}

extern "system" fn native_measure(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
    width_spec: jint,
    height_spec: jint,
) -> jlong {
    let (width, height) = unsafe { layout_from_handle(handle) }.measure(width_spec, height_spec);
    ((width as jlong) << 32) | (height as u32 as jlong)
}

extern "system" fn native_layout(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
    width: jint,
    height: jint,
) {
    unsafe { layout_from_handle(handle) }.layout(width, height);
}

/// `View.MeasureSpec`.
pub mod measure_spec {
    const MODE_MASK: i32 = 0b11 << 30;
    pub const UNSPECIFIED: i32 = 0;
    pub const EXACTLY: i32 = 1 << 30;
    pub const AT_MOST: i32 = 2 << 30;

    /// `MeasureSpec.makeMeasureSpec`; negative sizes clamp to zero.
    pub fn make(size: i32, mode: i32) -> i32 {
        (size.max(0) & !MODE_MASK) | mode
    }

    /// `MeasureSpec.getMode`
    pub fn mode(spec: i32) -> i32 {
        spec & MODE_MASK
    }

    /// `MeasureSpec.getSize`
    pub fn size(spec: i32) -> i32 {
        spec & !MODE_MASK
    }
}

/// Equivalent of `View.resolveSize`.
pub fn resolve_size(desired: i32, spec: i32) -> i32 {
    match measure_spec::mode(spec) {
        measure_spec::EXACTLY => measure_spec::size(spec),
        measure_spec::AT_MOST => desired.min(measure_spec::size(spec)),
        _ => desired,
    }
}

/// Calls `View.measure` on a child and returns its measured size in pixels.
pub fn measure_view(view: &JavaObject, width_spec: i32, height_spec: i32) -> (i32, i32) {
    view.call_void(
        "measure",
        "(II)V",
        &[JValue::Int(width_spec), JValue::Int(height_spec)],
    );
    (
        view.call_int("getMeasuredWidth"),
        view.call_int("getMeasuredHeight"),
    )
}

/// Calls `View.layout` on a child with edges relative to its parent.
pub fn layout_view(view: &JavaObject, left: i32, top: i32, right: i32, bottom: i32) {
    view.call_void(
        "layout",
        "(IIII)V",
        &[
            JValue::Int(left),
            JValue::Int(top),
            JValue::Int(right),
            JValue::Int(bottom),
        ],
    );
}
