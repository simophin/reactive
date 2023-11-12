use std::net::IpAddr;

use derive_jni::{java_class, WithJavaObject};
use jni::objects::JObject;

#[java_class]
trait View {
    fn set_text(&self, text: Option<i32>) -> &[i32];
}

struct MyView<'a>(JObject<'a>);

impl WithJavaObject for MyView<'_> {
    fn get_java_object(&self) -> Result<&JObject<'_>, jni::errors::Error> {
        Ok(&self.0)
    }
}

impl<'a> ViewJavaObject for MyView<'a> {}

fn main() {
    let view = MyView(JObject::null());
    view.set_text(Some(2));
}
