use derive_jni::{java_class, WithJavaObject};
use jni::{objects::JObject, InitArgsBuilder, JavaVM};

#[java_class]
trait View {
    fn hash_code(&self) -> i32;
}

struct MyView<'a>(JObject<'a>);

impl WithJavaObject for MyView<'_> {
    fn get_java_object(&self) -> Result<&JObject<'_>, jni::errors::Error> {
        Ok(&self.0)
    }
}

impl<'a> ViewJavaObject for MyView<'a> {}

fn main() {
    let vm = JavaVM::new(
        InitArgsBuilder::default()
            .build()
            .expect("To build init args"),
    )
    .expect("To run java");

    let mut guard = vm.attach_current_thread().expect("To attach thread");
    let b = guard
        .new_object("java/lang/Boolean", "(Z)V", &[true.into()])
        .expect("To create boolean");

    let code = MyView(b).hash_code(&mut guard).expect("To run hashcode");
    println!("Got hashcode {code}");

    // view.set_text(Some(2));
}
