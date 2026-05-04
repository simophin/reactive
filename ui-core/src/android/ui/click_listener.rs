use jni::objects::JObject;
use std::cell::RefCell;
use std::rc::Rc;

dexer::dex_class! {
    #[java_class("com.reactive.ReactiveOnClickListener")]
    pub struct ReactiveOnClickListener {
        pub on_click: RefCell<Option<Rc<dyn Fn()>>>,
    }
    extends "java/lang/Object";
    implements "android/view/View$OnClickListener";

    #[constructor]
    pub fn init(
        _env: &mut jni::JNIEnv,
        #[class("android/content/Context")] _context: JObject,
    ) -> Self {
        Self {
            on_click: RefCell::new(None),
        }
    }

    #[method(name = "onClick")]
    pub fn on_click(
        &mut self,
        _env: &mut jni::JNIEnv,
        #[class("android/view/View")] _v: JObject,
    ) {
        if let Some(cb) = self.on_click.borrow().as_ref() {
            cb();
        }
    }
}
