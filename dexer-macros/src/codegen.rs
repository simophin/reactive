use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Ident, LitStr, ReturnType, Type};

use crate::parse::{DexClass, DexMethod, JniParam, MethodKind};
use crate::validate::type_to_string;

pub fn generate(class: DexClass) -> TokenStream {
    let struct_def = gen_struct(&class);
    let impl_block = gen_impl(&class);
    let bridge_fns = gen_bridges(&class);

    quote! {
        #struct_def
        #impl_block
        #bridge_fns
    }
}

// ─────────────────────────── struct ──────────────────────────────────────

fn gen_struct(class: &DexClass) -> TokenStream {
    let vis = &class.vis;
    let name = &class.name;
    let fields = class.fields.iter().map(|f| {
        let fname = &f.name;
        let fty = &f.ty;
        quote! { pub #fname: #fty, }
    });
    quote! {
        #vis struct #name {
            #(#fields)*
        }
    }
}

// ─────────────────────────── impl block ──────────────────────────────────

fn gen_impl(class: &DexClass) -> TokenStream {
    let name = &class.name;

    let user_methods = class.methods.iter().map(|m| gen_user_method(m));
    let into_java = gen_into_java(class);
    let dex_output = gen_dex_output_fn(class);

    quote! {
        impl #name {
            #(#user_methods)*
            #into_java
            #dex_output
        }
    }
}

/// Emit the user's method body as a plain Rust fn on the struct.
fn gen_user_method(method: &DexMethod) -> TokenStream {
    let name = &method.rust_name;
    let body = &method.body;
    let ret = &method.return_ty;

    let self_param = if matches!(method.kind, MethodKind::Constructor) {
        quote! {}
    } else {
        quote! { &mut self, }
    };

    let env_param = quote! { env: &mut jni::JNIEnv<'_>, };

    let super_param = if matches!(method.kind, MethodKind::Override { .. }) {
        quote! { super_: dexer::SuperCaller, }
    } else {
        quote! {}
    };

    let jni_params = method.jni_params.iter().map(|p| {
        let pname = &p.name;
        let pty = &p.ty;
        quote! { #pname: #pty, }
    });

    quote! {
        fn #name(#self_param #env_param #super_param #(#jni_params)*) #ret
        #body
    }
}

/// `pub fn into_java(self, env, ctor_args...) -> Result<JObject>`
/// Uses the BYO constructor `<init>(args..., long)`.
fn gen_into_java(class: &DexClass) -> TokenStream {
    let java_class_str = &class.java_class;
    let ctor = class
        .methods
        .iter()
        .find(|m| matches!(m.kind, MethodKind::Constructor))
        .expect("validated: exactly one constructor");

    let arg_params = ctor.jni_params.iter().map(|p| {
        let n = &p.name;
        let t = &p.ty;
        quote! { #n: #t, }
    });

    let byo_descriptor = byo_descriptor_for(ctor);

    let arg_jvalues = ctor.jni_params.iter().map(|p| {
        let n = &p.name;
        jvalue_from_param(p, quote! { #n })
    });

    quote! {
        pub fn into_java<'local>(
            self,
            env: &mut jni::JNIEnv<'local>,
            #(#arg_params)*
        ) -> jni::errors::Result<jni::objects::JObject<'local>> {
            let __ptr = Box::into_raw(Box::new(self)) as jni::sys::jlong;
            env.new_object(
                #java_class_str,
                #byo_descriptor,
                &[
                    #(#arg_jvalues,)*
                    jni::objects::JValue::Long(__ptr),
                ],
            )
        }
    }
}

/// `pub fn dex_output() -> &'static dexer::DexOutput`
fn gen_dex_output_fn(class: &DexClass) -> TokenStream {
    let class_def_expr = gen_class_def(class);
    quote! {
        pub fn dex_output() -> &'static dexer::DexOutput {
            static __DEX: ::std::sync::OnceLock<dexer::DexOutput> = ::std::sync::OnceLock::new();
            __DEX.get_or_init(|| {
                #class_def_expr
                    .compile()
                    .expect("dexer: failed to compile DEX class")
            })
        }
    }
}

/// Build the `dexer::ClassDef { ... }` expression.
fn gen_class_def(class: &DexClass) -> TokenStream {
    let java_class = class.java_class.value();
    let extends = class.extends.value();
    let implements = class
        .implements
        .iter()
        .map(|i| i.value())
        .collect::<Vec<_>>();

    // Always-present nativePtr field
    let fields_expr = quote! {
        vec![
            dexer::FieldDef {
                name: "nativePtr".into(),
                descriptor: "J".into(),
                access: dexer::AccessFlags::PRIVATE,
            }
        ]
    };

    let method_entries = gen_method_entries(class);

    quote! {
        dexer::ClassDef {
            class_name:  #java_class.into(),
            superclass:  #extends.into(),
            interfaces:  vec![ #(#implements.into()),* ],
            fields:      #fields_expr,
            methods:     { #method_entries },
        }
    }
}

fn gen_method_entries(class: &DexClass) -> TokenStream {
    let name = &class.name;
    let mut entries: Vec<TokenStream> = Vec::new();

    let ctor = class
        .methods
        .iter()
        .find(|m| matches!(m.kind, MethodKind::Constructor))
        .unwrap();
    let extends = class.extends.value();
    let ctor_descriptor = jni_descriptor_for(ctor);
    let byo_desc = byo_descriptor_for(ctor);

    // Regular constructor (Coded)
    entries.push(quote! {
        dexer::MethodEntry::Coded {
            name: "<init>".into(),
            descriptor: #ctor_descriptor.into(),
            access: dexer::AccessFlags::PUBLIC.with(dexer::AccessFlags::CONSTRUCTOR),
            code: dexer::MethodCode::Constructor {
                superclass: #extends.into(),
                super_descriptor: #ctor_descriptor.into(),
            },
        },
    });

    // BYO constructor (Coded)
    entries.push(quote! {
        dexer::MethodEntry::Coded {
            name: "<init>".into(),
            descriptor: #byo_desc.into(),
            access: dexer::AccessFlags::PUBLIC.with(dexer::AccessFlags::CONSTRUCTOR),
            code: dexer::MethodCode::ByoConstructor {
                superclass: #extends.into(),
                super_descriptor: #ctor_descriptor.into(),
            },
        },
    });

    // $$init native bridge
    let bridge_init_ident = bridge_ident(name, "init");
    entries.push(quote! {
        dexer::MethodEntry::Native {
            name: "$$init".into(),
            descriptor: #ctor_descriptor.into(),
            access: dexer::AccessFlags::PRIVATE,
            fn_ptr: #bridge_init_ident as *mut ::std::ffi::c_void,
        },
    });

    // $$destroy native bridge
    let bridge_destroy_ident = bridge_ident(name, "destroy");
    entries.push(quote! {
        dexer::MethodEntry::Native {
            name: "$$destroy".into(),
            descriptor: "()V".into(),
            access: dexer::AccessFlags::PRIVATE,
            fn_ptr: #bridge_destroy_ident as *mut ::std::ffi::c_void,
        },
    });

    // finalize (Coded)
    entries.push(quote! {
        dexer::MethodEntry::Coded {
            name: "finalize".into(),
            descriptor: "()V".into(),
            access: dexer::AccessFlags::PROTECTED,
            code: dexer::MethodCode::Finalize,
        },
    });

    // User methods
    for method in &class.methods {
        match &method.kind {
            MethodKind::Constructor => {} // handled above
            MethodKind::Override { java_name } => {
                let jname = java_name.value();
                let desc = jni_descriptor_for(method);
                let super_name = format!("{jname}$$super");
                let bridge_fn = bridge_ident(name, &method.rust_name.to_string());

                // native method
                entries.push(quote! {
                    dexer::MethodEntry::Native {
                        name: #jname.into(),
                        descriptor: #desc.into(),
                        access: dexer::AccessFlags::PUBLIC,
                        fn_ptr: #bridge_fn as *mut ::std::ffi::c_void,
                    },
                });
                // super accessor
                entries.push(quote! {
                    dexer::MethodEntry::Coded {
                        name: #super_name.into(),
                        descriptor: #desc.into(),
                        access: dexer::AccessFlags::PRIVATE,
                        code: dexer::MethodCode::SuperAccessor {
                            superclass: #extends.into(),
                            method_name: #jname.into(),
                            descriptor: #desc.into(),
                        },
                    },
                });
            }
            MethodKind::Method { java_name } => {
                let jname = java_name.value();
                let desc = jni_descriptor_for(method);
                let bridge_fn = bridge_ident(name, &method.rust_name.to_string());

                entries.push(quote! {
                    dexer::MethodEntry::Native {
                        name: #jname.into(),
                        descriptor: #desc.into(),
                        access: dexer::AccessFlags::PUBLIC,
                        fn_ptr: #bridge_fn as *mut ::std::ffi::c_void,
                    },
                });
            }
        }
    }

    quote! {
        let mut __methods: Vec<dexer::MethodEntry> = Vec::new();
        #(
            __methods.push(#entries);
        )*
        __methods
    }
}

// ─────────────────────────── bridge functions ─────────────────────────────

fn gen_bridges(class: &DexClass) -> TokenStream {
    let name = &class.name;
    let mut bridges: Vec<TokenStream> = Vec::new();

    // $$init bridge
    bridges.push(gen_init_bridge(class));
    // $$destroy bridge
    bridges.push(gen_destroy_bridge(class));
    // Per-method bridges
    for method in &class.methods {
        match &method.kind {
            MethodKind::Constructor => {}
            MethodKind::Override { java_name } => {
                bridges.push(gen_method_bridge(name, method, Some(java_name)))
            }
            MethodKind::Method { .. } => bridges.push(gen_method_bridge(name, method, None)),
        }
    }

    quote! { #(#bridges)* }
}

fn gen_init_bridge(class: &DexClass) -> TokenStream {
    let name = &class.name;
    let fn_id = bridge_ident(name, "init");
    let ctor = class
        .methods
        .iter()
        .find(|m| matches!(m.kind, MethodKind::Constructor))
        .unwrap();

    let param_decls = ctor.jni_params.iter().map(|p| {
        let pn = &p.name;
        let pt = jni_sys_type(&p.ty);
        quote! { #pn: #pt, }
    });
    let param_conversions = ctor.jni_params.iter().map(|p| {
        let pn = &p.name;
        convert_from_sys(p, quote! { #pn })
    });
    let param_names: Vec<_> = ctor.jni_params.iter().map(|p| &p.name).collect();
    let param_names2 = param_names.clone();

    quote! {
        #[allow(non_snake_case)]
        unsafe extern "C" fn #fn_id(
            __env: *mut jni::sys::JNIEnv,
            __this: jni::sys::jobject,
            #(#param_decls)*
        ) {
            let mut __env = unsafe { jni::JNIEnv::from_raw(__env) }.unwrap();
            let __this_obj = unsafe { jni::objects::JObject::from_raw(__this) };
            let __current_this_guard = dexer::push_current_this(__this);
            #(let #param_names = { #param_conversions };)*
            let __state = #name::init(&mut __env, #(#param_names2),*);
            let __ptr = Box::into_raw(Box::new(__state)) as jni::sys::jlong;
            __env.set_field(&__this_obj, "nativePtr", "J", jni::objects::JValue::Long(__ptr)).unwrap();
            drop(__current_this_guard);
        }
    }
}

fn gen_destroy_bridge(class: &DexClass) -> TokenStream {
    let name = &class.name;
    let fn_id = bridge_ident(name, "destroy");
    quote! {
        #[allow(non_snake_case)]
        unsafe extern "C" fn #fn_id(
            __env: *mut jni::sys::JNIEnv,
            __this: jni::sys::jobject,
        ) {
            let mut __env = unsafe { jni::JNIEnv::from_raw(__env) }.unwrap();
            let __this_obj = unsafe { jni::objects::JObject::from_raw(__this) };
            let __ptr = __env.get_field(&__this_obj, "nativePtr", "J").unwrap().j().unwrap();
            if __ptr != 0 {
                unsafe { drop(Box::from_raw(__ptr as *mut #name)) };
            }
        }
    }
}

fn gen_method_bridge(
    struct_name: &Ident,
    method: &DexMethod,
    super_java_name: Option<&LitStr>,
) -> TokenStream {
    let fn_id = bridge_ident(struct_name, &method.rust_name.to_string());
    let rust_name = &method.rust_name;

    let param_decls = method.jni_params.iter().map(|p| {
        let pn = &p.name;
        let pt = jni_sys_type(&p.ty);
        quote! { #pn: #pt, }
    });
    let param_conversions = method.jni_params.iter().map(|p| {
        let pn = &p.name;
        convert_from_sys(p, quote! { #pn })
    });
    let param_names: Vec<_> = method.jni_params.iter().map(|p| &p.name).collect();

    let super_arg = if let Some(java_name) = super_java_name {
        let super_method = format!("{}$$super", java_name.value());
        let desc = jni_descriptor_for(method);
        quote! {
            let __super_ = dexer::SuperCaller::new(__this, #super_method, #desc);
        }
    } else {
        quote! {}
    };

    let call_super_arg = if super_java_name.is_some() {
        quote! { __super_, }
    } else {
        quote! {}
    };

    let ret_expr = gen_return_expr(&method.return_ty);

    quote! {
        #[allow(non_snake_case)]
        unsafe extern "C" fn #fn_id(
            __env: *mut jni::sys::JNIEnv,
            __this: jni::sys::jobject,
            #(#param_decls)*
        ) #ret_expr {
            let mut __env = unsafe { jni::JNIEnv::from_raw(__env) }.unwrap();
            let __this_obj = unsafe { jni::objects::JObject::from_raw(__this) };
            let __current_this_guard = dexer::push_current_this(__this);
            let __ptr = __env.get_field(&__this_obj, "nativePtr", "J").unwrap().j().unwrap();
            let __state = unsafe { &mut *(__ptr as *mut #struct_name) };
            #super_arg
            #(let #param_names = { #param_conversions };)*
            let __result = #struct_name::#rust_name(__state, &mut __env, #call_super_arg #(#param_names),*);
            drop(__current_this_guard);
            __result
        }
    }
}

// ─────────────────────────── descriptor helpers ───────────────────────────

/// Build the JNI method descriptor for a user method, e.g. "(Landroid/graphics/Canvas;I)V".
pub fn jni_descriptor_for(method: &DexMethod) -> String {
    let mut params = String::from("(");
    for p in &method.jni_params {
        params.push_str(&param_descriptor(p));
    }
    params.push(')');
    params.push_str(&return_descriptor_for(&method.return_ty));
    params
}

fn param_descriptor(p: &JniParam) -> String {
    if let Some(class) = &p.class_attr {
        format!("L{};", class.value())
    } else {
        primitive_descriptor(&p.ty)
    }
}

fn primitive_descriptor(ty: &Type) -> String {
    let s = type_to_string(ty);
    let last = s.split("::").last().unwrap_or(&s);
    match last.trim() {
        "jboolean" => "Z".into(),
        "jbyte" => "B".into(),
        "jchar" => "C".into(),
        "jshort" => "S".into(),
        "jint" => "I".into(),
        "jlong" => "J".into(),
        "jfloat" => "F".into(),
        "jdouble" => "D".into(),
        "JObject" => "Ljava/lang/Object;".into(),
        _ => "Ljava/lang/Object;".into(),
    }
}

fn return_descriptor_for(ret: &ReturnType) -> String {
    match ret {
        ReturnType::Default => "V".into(),
        ReturnType::Type(_, ty) => {
            let s = type_to_string(ty);
            if s == "Self" {
                return "V".into();
            }
            primitive_descriptor(ty)
        }
    }
}

/// Build the BYO constructor descriptor: insert `J` before the closing `)`.
fn byo_descriptor_for(ctor: &DexMethod) -> String {
    let base = jni_descriptor_for(ctor);
    // base = "(params)V" — insert J before )V
    let params: String = base
        .trim_start_matches('(')
        .splitn(2, ')')
        .next()
        .unwrap_or("")
        .to_string();
    format!("({params}J)V")
}

// ─────────────────────────── sys type mapping ─────────────────────────────

/// Map a user-facing JNI type to its `jni::sys` equivalent for the extern "C" signature.
fn jni_sys_type(ty: &Type) -> TokenStream {
    let s = type_to_string(ty);
    let last = s.split("::").last().unwrap_or(&s).trim();
    match last {
        "jboolean" => quote! { jni::sys::jboolean },
        "jbyte" => quote! { jni::sys::jbyte },
        "jchar" => quote! { jni::sys::jchar },
        "jshort" => quote! { jni::sys::jshort },
        "jint" => quote! { jni::sys::jint },
        "jlong" => quote! { jni::sys::jlong },
        "jfloat" => quote! { jni::sys::jfloat },
        "jdouble" => quote! { jni::sys::jdouble },
        "JObject" => quote! { jni::sys::jobject },
        _ => quote! { jni::sys::jobject },
    }
}

/// Convert a raw sys value to the user-facing type at the bridge call site.
fn convert_from_sys(p: &JniParam, expr: TokenStream) -> TokenStream {
    let s = type_to_string(&p.ty);
    let last = s.split("::").last().unwrap_or(&s).trim();
    if last == "JObject" {
        quote! { unsafe { jni::objects::JObject::from_raw(#expr) } }
    } else {
        // primitives pass through directly
        expr
    }
}

/// For `into_java`, convert a user param to a `JValue`.
fn jvalue_from_param(p: &JniParam, expr: TokenStream) -> TokenStream {
    let s = type_to_string(&p.ty);
    let last = s.split("::").last().unwrap_or(&s).trim();
    match last {
        "JObject" => quote! { jni::objects::JValue::Object(&#expr) },
        "jboolean" => quote! { jni::objects::JValue::Bool(#expr) },
        "jbyte" => quote! { jni::objects::JValue::Byte(#expr) },
        "jchar" => quote! { jni::objects::JValue::Char(#expr) },
        "jshort" => quote! { jni::objects::JValue::Short(#expr) },
        "jint" => quote! { jni::objects::JValue::Int(#expr) },
        "jlong" => quote! { jni::objects::JValue::Long(#expr) },
        "jfloat" => quote! { jni::objects::JValue::Float(#expr) },
        "jdouble" => quote! { jni::objects::JValue::Double(#expr) },
        _ => quote! { jni::objects::JValue::Object(&#expr) },
    }
}

fn gen_return_expr(ret: &ReturnType) -> TokenStream {
    match ret {
        ReturnType::Default => quote! {},
        ReturnType::Type(_, ty) => {
            let s = type_to_string(ty);
            if s == "Self" {
                return quote! {};
            }
            quote! { -> #ty }
        }
    }
}

// ─────────────────────────── identifier helpers ───────────────────────────

fn bridge_ident(struct_name: &Ident, method: &str) -> Ident {
    format_ident!("__dexer_bridge_{}_{}", struct_name, method)
}
