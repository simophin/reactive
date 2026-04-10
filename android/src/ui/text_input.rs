use std::cell::{Cell, RefCell};
use std::fmt;
use std::ops::Range;
use std::rc::Rc;

use jni::objects::{JObject, JString, JValue};
use reactive_core::Signal;
use ui_core::widgets::{PlatformTextType, TextChange, TextInput, TextInputState};
use ui_core::Prop;

use crate::bindings;
use crate::ui::text_watcher::ReactiveTextWatcher;
use crate::ui::view_component::{AndroidView, AndroidViewBuilder, AndroidViewComponent};

pub type AndroidTextInput = AndroidViewComponent<AndroidView, ui_core::NoChild>;

#[derive(Clone, PartialEq, Eq)]
pub struct AndroidText(pub String);

impl fmt::Display for AndroidText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<&str> for AndroidText {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

unsafe impl Send for AndroidText {}
unsafe impl Sync for AndroidText {}

impl PlatformTextType for AndroidText {
    type RefType<'a> = &'a str;

    fn len(&self) -> usize {
        self.0.encode_utf16().count()
    }

    fn replace(&self, range: Range<usize>, with: &Self::RefType<'_>) -> Self {
        let mut utf16: Vec<u16> = self.0.encode_utf16().collect();
        let replacement: Vec<u16> = with.encode_utf16().collect();
        utf16.splice(range, replacement);
        Self(String::from_utf16(&utf16).expect("valid UTF-16"))
    }

    fn as_str(&self) -> Option<&str> {
        Some(&self.0)
    }
}

pub static PROP_FONT_SIZE: &Prop<AndroidTextInput, AndroidView, f64> = &Prop::new(|view, size| {
    let mut env = view.env();
    bindings::call_void::<bindings::edit_text::setTextSize, (jni::sys::jfloat,)>(
        &mut env,
        view.as_obj(),
        &[JValue::Float(size as f32)],
    )
    .expect("set text size");
});

impl TextInput for AndroidTextInput {
    type PlatformTextType = AndroidText;

    fn new(
        value: impl Signal<Value = TextInputState<AndroidText>> + 'static,
        on_change: impl for<'a> FnMut(TextChange<&'a str>) + 'static,
    ) -> Self {
        let on_change = Rc::new(RefCell::new(on_change));
        let updating = Rc::new(Cell::new(false));

        AndroidViewComponent(AndroidViewBuilder::create_no_child(
            move |ctx| {
                let java_vm = AndroidView::java_vm();
                let mut env = java_vm
                    .attach_current_thread_permanently()
                    .expect("attach thread");
                let activity = AndroidView::activity();
                let edit_text = bindings::new_object::<bindings::edit_text::EditText>(
                    &mut env,
                    "(Landroid/content/Context;)V",
                    &[JValue::Object(activity.as_obj())],
                )
                .expect("create EditText");
                let view = AndroidView::new(&mut env, &edit_text);

                let watcher = ReactiveTextWatcher {
                    after_change: RefCell::new(Some(Rc::new({
                        let view = view.clone();
                        let updating = updating.clone();
                        let on_change = on_change.clone();
                        move || {
                            if updating.get() {
                                return;
                            }
                            let mut env = view.env();
                            let editable =
                                bindings::call_object::<bindings::edit_text::getText, ()>(
                                    &mut env,
                                    view.as_obj(),
                                    &[],
                                )
                                .expect("read edit text");
                            let string_obj =
                                bindings::call_object::<bindings::object::toString, ()>(
                                    &mut env,
                                    &editable,
                                    &[],
                                )
                                .expect("editable toString");
                            let text = env
                                .get_string(&JString::from(string_obj))
                                .expect("string contents")
                                .to_string_lossy()
                                .into_owned();
                            on_change.borrow_mut()(TextChange::Replacement {
                                replace: 0..text.encode_utf16().count(),
                                with: &text,
                            });
                        }
                    }))),
                };

                let activity_obj = env
                    .new_local_ref(activity.as_obj())
                    .expect("clone activity ref");
                let watcher_obj = watcher
                    .into_java(&mut env, activity_obj)
                    .expect("create text watcher");
                bindings::call_void::<
                    bindings::edit_text::addTextChangedListener,
                    (jni::sys::jobject,),
                >(
                    &mut env,
                    &edit_text,
                    &[JValue::Object((&watcher_obj).into())],
                )
                .expect("attach text watcher");

                let value_view = view.clone();
                let updating_effect = updating.clone();
                ctx.create_effect(move |_, _| {
                    let state = value.read();
                    let current = current_edit_text_string(&value_view);

                    updating_effect.set(true);
                    if current != state.text.0 {
                        set_edit_text_string(&value_view, &state.text.0);
                    }
                    let mut env = value_view.env();
                    bindings::call_void::<
                        bindings::edit_text::setSelection,
                        (jni::sys::jint, jni::sys::jint),
                    >(
                        &mut env,
                        value_view.as_obj(),
                        &[
                            JValue::Int(state.selection.start as i32),
                            JValue::Int(state.selection.end as i32),
                        ],
                    )
                    .expect("set selection");
                    updating_effect.set(false);
                });

                view
            },
            |v| v,
        ))
    }

    fn font_size(self, size: impl Signal<Value = f64> + 'static) -> Self {
        AndroidViewComponent(self.0.bind(PROP_FONT_SIZE, size))
    }
}

fn current_edit_text_string(view: &AndroidView) -> String {
    let mut env = view.env();
    let current_obj =
        bindings::call_object::<bindings::edit_text::getText, ()>(&mut env, view.as_obj(), &[])
            .expect("read current text");
    let current_str_obj =
        bindings::call_object::<bindings::object::toString, ()>(&mut env, &current_obj, &[])
            .expect("current text toString");
    env.get_string(&JString::from(current_str_obj))
        .expect("current string")
        .to_string_lossy()
        .into_owned()
}

fn set_edit_text_string(view: &AndroidView, text: &str) {
    let mut env = view.env();
    let java_text = bindings::new_java_string(&mut env, text).expect("new string");
    let java_text_obj = JObject::from(java_text);
    bindings::call_void::<bindings::edit_text::setText, (jni::sys::jobject,)>(
        &mut env,
        view.as_obj(),
        &[JValue::Object(&java_text_obj)],
    )
    .expect("set edit text");
}
