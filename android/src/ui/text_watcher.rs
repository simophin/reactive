use jni::objects::JObject;
use std::cell::RefCell;
use std::rc::Rc;

dexer::dex_class! {
    #[java_class("com.reactive.ReactiveTextWatcher")]
    pub struct ReactiveTextWatcher {
        pub after_change: RefCell<Option<Rc<dyn Fn()>>>,
    }
    extends "java/lang/Object";
    implements "android/text/TextWatcher";

    #[constructor]
    pub fn init(
        _env: &mut jni::JNIEnv,
        #[class("android/content/Context")] _context: JObject,
    ) -> Self {
        Self {
            after_change: RefCell::new(None),
        }
    }

    #[method(name = "beforeTextChanged")]
    pub fn before_text_changed(
        &mut self,
        _env: &mut jni::JNIEnv,
        #[class("java/lang/CharSequence")] _text: JObject,
        _start: jni::sys::jint,
        _count: jni::sys::jint,
        _after: jni::sys::jint,
    ) {
    }

    #[method(name = "onTextChanged")]
    pub fn on_text_changed(
        &mut self,
        _env: &mut jni::JNIEnv,
        #[class("java/lang/CharSequence")] _text: JObject,
        _start: jni::sys::jint,
        _before: jni::sys::jint,
        _count: jni::sys::jint,
    ) {
    }

    #[method(name = "afterTextChanged")]
    pub fn after_text_changed(
        &mut self,
        _env: &mut jni::JNIEnv,
        #[class("android/text/Editable")] _editable: JObject,
    ) {
        if let Some(cb) = self.after_change.borrow().as_ref() {
            cb();
        }
    }
}
