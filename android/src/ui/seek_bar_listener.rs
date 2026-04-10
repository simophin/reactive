use jni::objects::JObject;
use std::cell::RefCell;
use std::rc::Rc;

dexer::dex_class! {
    #[java_class("com.reactive.ReactiveOnSeekBarChangeListener")]
    pub struct ReactiveOnSeekBarChangeListener {
        pub on_change: RefCell<Option<Rc<dyn Fn(i32)>>>,
    }
    extends "java/lang/Object";
    implements "android/widget/SeekBar$OnSeekBarChangeListener";

    #[constructor]
    pub fn init(
        _env: &mut jni::JNIEnv,
        #[class("android/content/Context")] _context: JObject,
    ) -> Self {
        Self {
            on_change: RefCell::new(None),
        }
    }

    #[method(name = "onProgressChanged")]
    pub fn on_progress_changed(
        &mut self,
        _env: &mut jni::JNIEnv,
        #[class("android/widget/SeekBar")] _seek_bar: JObject,
        progress: jni::sys::jint,
        _from_user: jni::sys::jboolean,
    ) {
        if let Some(cb) = self.on_change.borrow().as_ref() {
            cb(progress);
        }
    }

    #[method(name = "onStartTrackingTouch")]
    pub fn on_start_tracking_touch(
        &mut self,
        _env: &mut jni::JNIEnv,
        #[class("android/widget/SeekBar")] _seek_bar: JObject,
    ) {
    }

    #[method(name = "onStopTrackingTouch")]
    pub fn on_stop_tracking_touch(
        &mut self,
        _env: &mut jni::JNIEnv,
        #[class("android/widget/SeekBar")] _seek_bar: JObject,
    ) {
    }
}
