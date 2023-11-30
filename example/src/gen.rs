pub mod com {
    pub mod kogan {
        pub mod databinding {
            pub struct FragmentProductReviewsBinding {
                java_class: ::jni::objects::GlobalRef,
                r#get_root_method_id_cache: std::sync::OnceLock<
                    Result<::jni::objects::JMethodID, std::borrow::Cow<'static, str>>,
                >,
                r#inflate_method_id_cache: std::sync::OnceLock<
                    Result<
                        ::jni::objects::JStaticMethodID,
                        std::borrow::Cow<'static, str>,
                    >,
                >,
                r#inflate_1_method_id_cache: std::sync::OnceLock<
                    Result<
                        ::jni::objects::JStaticMethodID,
                        std::borrow::Cow<'static, str>,
                    >,
                >,
                r#bind_method_id_cache: std::sync::OnceLock<
                    Result<
                        ::jni::objects::JStaticMethodID,
                        std::borrow::Cow<'static, str>,
                    >,
                >,
                r#get_root_1_method_id_cache: std::sync::OnceLock<
                    Result<::jni::objects::JMethodID, std::borrow::Cow<'static, str>>,
                >,
            }
            impl FragmentProductReviewsBinding {
                pub fn new<'local>(
                    env: &mut ::jni::JNIEnv<'local>,
                ) -> ::jni::errors::Result<Self> {
                    let java_class = env
                        .find_class(
                            "com/kogan/databinding/FragmentProductReviewsBinding",
                        )?;
                    let java_class = env.new_global_ref(java_class)?;
                    Ok(Self {
                        java_class,
                        r#get_root_method_id_cache: Default::default(),
                        r#inflate_method_id_cache: Default::default(),
                        r#inflate_1_method_id_cache: Default::default(),
                        r#bind_method_id_cache: Default::default(),
                        r#get_root_1_method_id_cache: Default::default(),
                    })
                }
                pub fn get_java_class<'local>(&self) -> ::jni::objects::JClass<'local> {
                    let raw = self.java_class.as_raw() as ::jni::sys::jclass;
                    unsafe { ::jni::objects::JClass::from_raw(raw) }
                }
                pub fn r#get_root<'local>(
                    &self,
                    env: &mut ::jni::JNIEnv<'local>,
                    obj: &::jni::objects::JObject<'_>,
                ) -> ::jni::errors::Result<::jni::objects::JObject<'local>> {
                    let method_id = match self
                        .r#get_root_method_id_cache
                        .get_or_init(|| {
                            env.get_method_id(
                                    self.get_java_class(),
                                    "getRoot",
                                    "()Landroidx/constraintlayout/widget/ConstraintLayout;",
                                )
                                .map_err(|e| std::borrow::Cow::Owned(
                                    format!("Unable to find method '{}': {}", "getRoot", e),
                                ))
                        })
                    {
                        Ok(v) => *v,
                        Err(_e) => {
                            return Err(::jni::errors::Error::MethodNotFound {
                                name: "getRoot".to_string(),
                                sig: "()Landroidx/constraintlayout/widget/ConstraintLayout;"
                                    .to_string(),
                            });
                        }
                    };
                    unsafe {
                        env.call_method_unchecked(
                            obj,
                            method_id,
                            ::jni::signature::ReturnType::Object,
                            &[],
                        )
                    }?
                        .try_into()
                }
                pub fn r#inflate<'local>(
                    &self,
                    env: &mut ::jni::JNIEnv<'local>,
                    arg0: &::jni::objects::JObject<'_>,
                ) -> ::jni::errors::Result<::jni::objects::JObject<'local>> {
                    let method_id = match self
                        .r#inflate_method_id_cache
                        .get_or_init(|| {
                            env.get_static_method_id(
                                    self.get_java_class(),
                                    "inflate",
                                    "(Landroid/view/LayoutInflater;)Lcom/kogan/databinding/FragmentProductReviewsBinding;",
                                )
                                .map_err(|e| std::borrow::Cow::Owned(
                                    format!("Unable to find method '{}': {}", "inflate", e),
                                ))
                        })
                    {
                        Ok(v) => *v,
                        Err(_e) => {
                            return Err(::jni::errors::Error::MethodNotFound {
                                name: "inflate".to_string(),
                                sig: "(Landroid/view/LayoutInflater;)Lcom/kogan/databinding/FragmentProductReviewsBinding;"
                                    .to_string(),
                            });
                        }
                    };
                    unsafe {
                        env.call_static_method_unchecked(
                            self.get_java_class(),
                            method_id,
                            ::jni::signature::ReturnType::Object,
                            &[::jni::objects::JValueGen::Object(arg0).as_jni()],
                        )
                    }?
                        .try_into()
                }
                pub fn r#inflate_1<'local>(
                    &self,
                    env: &mut ::jni::JNIEnv<'local>,
                    arg0: &::jni::objects::JObject<'_>,
                    arg1: &::jni::objects::JObject<'_>,
                    arg2: ::jni::sys::jboolean,
                ) -> ::jni::errors::Result<::jni::objects::JObject<'local>> {
                    let method_id = match self
                        .r#inflate_1_method_id_cache
                        .get_or_init(|| {
                            env.get_static_method_id(
                                    self.get_java_class(),
                                    "inflate",
                                    "(Landroid/view/LayoutInflater;Landroid/view/ViewGroup;Z)Lcom/kogan/databinding/FragmentProductReviewsBinding;",
                                )
                                .map_err(|e| std::borrow::Cow::Owned(
                                    format!("Unable to find method '{}': {}", "inflate", e),
                                ))
                        })
                    {
                        Ok(v) => *v,
                        Err(_e) => {
                            return Err(::jni::errors::Error::MethodNotFound {
                                name: "inflate".to_string(),
                                sig: "(Landroid/view/LayoutInflater;Landroid/view/ViewGroup;Z)Lcom/kogan/databinding/FragmentProductReviewsBinding;"
                                    .to_string(),
                            });
                        }
                    };
                    unsafe {
                        env.call_static_method_unchecked(
                            self.get_java_class(),
                            method_id,
                            ::jni::signature::ReturnType::Object,
                            &[
                                ::jni::objects::JValueGen::Object(arg0).as_jni(),
                                ::jni::objects::JValueGen::Object(arg1).as_jni(),
                                ::jni::objects::JValueGen::<
                                    ::jni::objects::JObject<'_>,
                                >::from(arg2)
                                    .as_jni(),
                            ],
                        )
                    }?
                        .try_into()
                }
                pub fn r#bind<'local>(
                    &self,
                    env: &mut ::jni::JNIEnv<'local>,
                    arg0: &::jni::objects::JObject<'_>,
                ) -> ::jni::errors::Result<::jni::objects::JObject<'local>> {
                    let method_id = match self
                        .r#bind_method_id_cache
                        .get_or_init(|| {
                            env.get_static_method_id(
                                    self.get_java_class(),
                                    "bind",
                                    "(Landroid/view/View;)Lcom/kogan/databinding/FragmentProductReviewsBinding;",
                                )
                                .map_err(|e| std::borrow::Cow::Owned(
                                    format!("Unable to find method '{}': {}", "bind", e),
                                ))
                        })
                    {
                        Ok(v) => *v,
                        Err(_e) => {
                            return Err(::jni::errors::Error::MethodNotFound {
                                name: "bind".to_string(),
                                sig: "(Landroid/view/View;)Lcom/kogan/databinding/FragmentProductReviewsBinding;"
                                    .to_string(),
                            });
                        }
                    };
                    unsafe {
                        env.call_static_method_unchecked(
                            self.get_java_class(),
                            method_id,
                            ::jni::signature::ReturnType::Object,
                            &[::jni::objects::JValueGen::Object(arg0).as_jni()],
                        )
                    }?
                        .try_into()
                }
                pub fn r#get_root_1<'local>(
                    &self,
                    env: &mut ::jni::JNIEnv<'local>,
                    obj: &::jni::objects::JObject<'_>,
                ) -> ::jni::errors::Result<::jni::objects::JObject<'local>> {
                    let method_id = match self
                        .r#get_root_1_method_id_cache
                        .get_or_init(|| {
                            env.get_method_id(
                                    self.get_java_class(),
                                    "getRoot",
                                    "()Landroid/view/View;",
                                )
                                .map_err(|e| std::borrow::Cow::Owned(
                                    format!("Unable to find method '{}': {}", "getRoot", e),
                                ))
                        })
                    {
                        Ok(v) => *v,
                        Err(_e) => {
                            return Err(::jni::errors::Error::MethodNotFound {
                                name: "getRoot".to_string(),
                                sig: "()Landroid/view/View;".to_string(),
                            });
                        }
                    };
                    unsafe {
                        env.call_method_unchecked(
                            obj,
                            method_id,
                            ::jni::signature::ReturnType::Object,
                            &[],
                        )
                    }?
                        .try_into()
                }
            }
        }
    }
}
