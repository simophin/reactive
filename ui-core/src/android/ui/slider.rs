use std::cell::RefCell;
use std::ops::Range;
use std::rc::Rc;

use jni::objects::JValue;
use reactive_core::{Signal, SignalExt};
use ui_core::widgets::Slider;
use ui_core::Prop;

use crate::android::bindings;
use crate::android::ui::seek_bar_listener::ReactiveOnSeekBarChangeListener;
use crate::android::ui::view_component::{AndroidView, AndroidViewBuilder, AndroidViewComponent};

pub type AndroidSlider = AndroidViewComponent<AndroidView, ui_core::NoChild>;

pub static PROP_VALUE: &Prop<AndroidSlider, AndroidView, i32> = &Prop::new(|view, value| {
    let mut env = view.env();
    bindings::call_void::<bindings::seek_bar::setProgress, (jni::sys::jint,)>(
        &mut env,
        view.as_obj(),
        &[JValue::Int(value)],
    )
    .expect("set slider value");
});

pub static PROP_RANGE: &Prop<AndroidSlider, AndroidView, Range<i32>> = &Prop::new(|view, range| {
    let mut env = view.env();
    bindings::call_void::<bindings::seek_bar::setMin, (jni::sys::jint,)>(
        &mut env,
        view.as_obj(),
        &[JValue::Int(range.start)],
    )
    .expect("set slider min");
    bindings::call_void::<bindings::seek_bar::setMax, (jni::sys::jint,)>(
        &mut env,
        view.as_obj(),
        &[JValue::Int(range.end)],
    )
    .expect("set slider max");
});

impl Slider for AndroidSlider {
    fn new(
        value: impl Signal<Value = usize> + 'static,
        range: impl Signal<Value = Range<usize>> + 'static,
        on_change: impl Fn(usize) + 'static,
    ) -> Self {
        let on_change = Rc::new(on_change);
        AndroidViewComponent(
            AndroidViewBuilder::create_no_child(
                move |_ctx| {
                    let java_vm = AndroidView::java_vm();
                    let mut env = java_vm
                        .attach_current_thread_permanently()
                        .expect("attach thread");
                    let activity = AndroidView::activity();
                    let seek_bar = bindings::new_object::<bindings::seek_bar::SeekBar>(
                        &mut env,
                        "(Landroid/content/Context;)V",
                        &[JValue::Object(activity.as_obj())],
                    )
                    .expect("create SeekBar");

                    let listener = ReactiveOnSeekBarChangeListener {
                        on_change: RefCell::new(Some(Rc::new(move |progress| {
                            on_change(progress as usize);
                        }))),
                    };
                    let activity_obj = env
                        .new_local_ref(activity.as_obj())
                        .expect("clone activity ref");
                    let listener_obj = listener
                        .into_java(&mut env, activity_obj)
                        .expect("create seek listener");
                    bindings::call_void::<
                        bindings::seek_bar::setOnSeekBarChangeListener,
                        (jni::sys::jobject,),
                    >(
                        &mut env,
                        &seek_bar,
                        &[JValue::Object((&listener_obj).into())],
                    )
                    .expect("attach seek listener");

                    AndroidView::new(&mut env, &seek_bar)
                },
                |v| v,
            )
            .bind(PROP_VALUE, value.map_value(|v| v as i32))
            .bind(
                PROP_RANGE,
                range.map_value(|range| (range.start as i32)..(range.end as i32)),
            ),
        )
    }
}
