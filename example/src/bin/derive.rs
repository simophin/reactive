// use derive_jni::{java_class, WithJavaObject};
// use jni::{
//     objects::{AutoLocal, JObject},
//     InitArgsBuilder, JNIEnv, JavaVM,
// };
//
// #[java_class("java/util/Date")]
// trait Date {
//     fn new_with_mills(mills: i64) -> Self;
//     fn new() -> Self;
//     fn get_month(&self) -> i32;
//     fn hash_code(&self) -> i32;
//     fn to_string(&self) -> Option<String>;
// }
//
// fn main() {
//     let vm = JavaVM::new(
//         InitArgsBuilder::default()
//             .build()
//             .expect("To build init args"),
//     )
//     .expect("To run java");
//
//     let mut guard = vm.attach_current_thread().expect("To attach thread");
//
//     let date = DateJavaObject::new(&mut guard).expect("To create object");
//
//     let code = date.hash_code(&mut guard).expect("To run hashcode");
//     let str = date.to_string(&mut guard).expect("To run toString");
//     println!("Got hashcode {code}, str = {str:?}");
//
//     // view.set_text(Some(2));
// }
