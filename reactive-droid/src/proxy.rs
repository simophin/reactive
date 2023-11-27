use std::{
    borrow::Cow,
    sync::{Arc, Weak},
};

use jni::{
    errors::Result,
    objects::{AutoLocal, JClass, JObject, JObjectArray, JString, JValueGen},
    sys::jlong,
    JNIEnv,
};

use crate::value::ext::JValueGenExt;
use crate::value::IntoJValue;

type ProxyCallbackDyn = dyn for<'a> Fn(&mut JNIEnv<'a>, JObject<'a>, &str, JObjectArray<'a>) -> JObject<'a>
    + Send
    + Sync
    + 'static;

pub type ProxyCallback = Box<ProxyCallbackDyn>;

struct ProxyData(Weak<ProxyCallbackDyn>);

impl ProxyData {
    pub fn from_raw(instance: jlong) -> &'static mut Self {
        unsafe { &mut *(instance as *mut _) }
    }
}

pub struct ProxyHandle(Arc<ProxyCallbackDyn>);

pub fn new_java_proxy<'local>(
    env: &mut JNIEnv<'local>,
    class_name: impl AsRef<str>,
    callback: ProxyCallback,
) -> Result<(AutoLocal<'local, JObject<'local>>, ProxyHandle)> {
    let class_name = class_name.as_ref().into_jvalue(env)?;
    let callback: Arc<ProxyCallbackDyn> = Arc::new(callback);
    let call = Box::leak(Box::new(ProxyData(Arc::downgrade(&callback))));
    let JValueGen::Object(obj) = env.call_static_method(
        "dev/fanchao/reactive/ReactiveContext",
        "requestProxy",
        "(Ljava/lang/String;J)Ljava/lang/Object;",
        &[class_name.as_value_ref(), (call as *const _ as i64).into()],
    )?
    else {
        let _ = unsafe { Box::from_raw(call) };
        panic!("Expecting object from requestProxy");
    };

    Ok((AutoLocal::new(obj, env), ProxyHandle(callback)))
}

#[warn(non_snake_case)]
#[no_mangle]
pub extern "system" fn Java_dev_fanchao_reactive_ReactiveContext_onProxyCalled<'local>(
    mut env: JNIEnv<'local>,
    _obj: JClass<'local>,
    instance: jlong,
    proxy_instance: JObject<'local>,
    method_name: JString<'local>,
    args: JObjectArray<'local>,
) -> JObject<'local> {
    match on_proxy_called(
        unsafe { env.unsafe_clone() },
        instance,
        proxy_instance,
        method_name,
        args,
    ) {
        Ok(v) => v,
        Err(e) => {
            let _ = env.throw_new(
                "java/lang/RuntimeException",
                format!("Failed to call proxy: {e}"),
            );
            JObject::null()
        }
    }
}

#[warn(non_snake_case)]
#[no_mangle]
pub extern "system" fn Java_dev_fanchao_reactive_ReactiveContext_onProxyDestroyed<'local>(
    _env: JNIEnv<'local>,
    _obj: JClass<'local>,
    instance: jlong,
) {
    if instance == 0 {
        return;
    }

    let _ = unsafe { Box::from_raw(instance as *mut ProxyData) };
}

fn on_proxy_called<'local>(
    mut env: JNIEnv<'local>,
    instance: jlong,
    proxy_instance: JObject<'local>,
    method_name: JString<'local>,
    args: JObjectArray<'local>,
) -> std::result::Result<JObject<'local>, Cow<'static, str>> {
    if instance == 0 {
        return Err(Cow::Borrowed("native instance is null"));
    }

    let method_name = unsafe {
        env.get_string_unchecked(&method_name)
            .map_err(|e| format!("Error getting method: {e}"))?
    };
    let method_name = method_name
        .to_str()
        .map_err(|e| format!("Error converting java string to utf-8 string: {e}"))?;

    let instance = ProxyData::from_raw(instance);
    if let Some(callback) = instance.0.upgrade() {
        return Ok((callback)(&mut env, proxy_instance, method_name, args));
    }

    log::warn!("Native instance is gone when proxy is called");
    Ok(JObject::null())
}
