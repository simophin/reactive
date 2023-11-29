pub mod com {
    pub mod kogan {
        pub mod databinding {
            pub struct FragmentProductReviewsBinding {
                java_class: ::jni::objects::GlobalRef,
                r#get_root_method_id_cache: std::cell::OnceCell<
                    ::jni::errors::Result<::jni::objects::JMethodID>,
                >,
                r#inflate_method_id_cache: std::cell::OnceCell<
                    ::jni::errors::Result<::jni::objects::JStaticMethodID>,
                >,
                r#inflate_1_method_id_cache: std::cell::OnceCell<
                    ::jni::errors::Result<::jni::objects::JStaticMethodID>,
                >,
                r#bind_method_id_cache: std::cell::OnceCell<
                    ::jni::errors::Result<::jni::objects::JStaticMethodID>,
                >,
                r#get_root_1_method_id_cache: std::cell::OnceCell<
                    ::jni::errors::Result<::jni::objects::JMethodID>,
                >,
            }
            impl FragmentProductReviewsBinding {
                pub fn new<'local>(
                    env: &mut ::jni::JNIEnv<'local>,
                ) -> ::jni::errors::Result<Self> {
                    let java_class = env
                        .new_global_ref(
                            env
                                .find_class(
                                    "com/kogan/databinding/FragmentProductReviewsBinding",
                                )?,
                        )?;
                    Ok(Self {
                        java_class,
                        r#get_root_method_id_cache: Default::default(),
                        r#inflate_method_id_cache: Default::default(),
                        r#inflate_1_method_id_cache: Default::default(),
                        r#bind_method_id_cache: Default::default(),
                        r#get_root_1_method_id_cache: Default::default(),
                    })
                }
                pub fn r#get_root<'local>(
                    &self,
                    env: &mut ::jni::JNIEnv<'local>,
                    obj: &::jni::objects::JObject<'_>,
                ) -> ::jni::errors::Result<
                    jni::objects::AutoLocal<'local, ::jni::objects::JObject<'local>>,
                > {
                    let method_id = match self
                        .r#get_root_method_id_cache
                        .get_or_init(|| {
                            env.get_method_id(
                                self.java_class.as_ref(),
                                "getRoot",
                                "()Landroidx/constraintlayout/widget/ConstraintLayout;",
                            )
                        })
                    {
                        Ok(v) => *v,
                        Err(e) => return Err(e.clone()),
                    };
                    todo!()
                }
                pub fn r#inflate<'local>(
                    &self,
                    env: &mut ::jni::JNIEnv<'local>,
                    obj: &::jni::objects::JObject<'_>,
                    arg0: &::jni::objects::JObject<'_>,
                ) -> ::jni::errors::Result<
                    jni::objects::AutoLocal<'local, ::jni::objects::JObject<'local>>,
                > {
                    let method_id = match self
                        .r#inflate_method_id_cache
                        .get_or_init(|| {
                            env.get_static_method_id(
                                self.java_class.as_ref(),
                                "inflate",
                                "(Landroid/view/LayoutInflater;)Lcom/kogan/databinding/FragmentProductReviewsBinding;",
                            )
                        })
                    {
                        Ok(v) => *v,
                        Err(e) => return Err(e.clone()),
                    };
                    todo!()
                }
                pub fn r#inflate_1<'local>(
                    &self,
                    env: &mut ::jni::JNIEnv<'local>,
                    obj: &::jni::objects::JObject<'_>,
                    arg0: &::jni::objects::JObject<'_>,
                    arg1: &::jni::objects::JObject<'_>,
                    arg2: ::jni::sys::jboolean,
                ) -> ::jni::errors::Result<
                    jni::objects::AutoLocal<'local, ::jni::objects::JObject<'local>>,
                > {
                    let method_id = match self
                        .r#inflate_1_method_id_cache
                        .get_or_init(|| {
                            env.get_static_method_id(
                                self.java_class.as_ref(),
                                "inflate",
                                "(Landroid/view/LayoutInflater;Landroid/view/ViewGroup;Z)Lcom/kogan/databinding/FragmentProductReviewsBinding;",
                            )
                        })
                    {
                        Ok(v) => *v,
                        Err(e) => return Err(e.clone()),
                    };
                    todo!()
                }
                pub fn r#bind<'local>(
                    &self,
                    env: &mut ::jni::JNIEnv<'local>,
                    obj: &::jni::objects::JObject<'_>,
                    arg0: &::jni::objects::JObject<'_>,
                ) -> ::jni::errors::Result<
                    jni::objects::AutoLocal<'local, ::jni::objects::JObject<'local>>,
                > {
                    let method_id = match self
                        .r#bind_method_id_cache
                        .get_or_init(|| {
                            env.get_static_method_id(
                                self.java_class.as_ref(),
                                "bind",
                                "(Landroid/view/View;)Lcom/kogan/databinding/FragmentProductReviewsBinding;",
                            )
                        })
                    {
                        Ok(v) => *v,
                        Err(e) => return Err(e.clone()),
                    };
                    todo!()
                }
                pub fn r#get_root_1<'local>(
                    &self,
                    env: &mut ::jni::JNIEnv<'local>,
                    obj: &::jni::objects::JObject<'_>,
                ) -> ::jni::errors::Result<
                    jni::objects::AutoLocal<'local, ::jni::objects::JObject<'local>>,
                > {
                    let method_id = match self
                        .r#get_root_1_method_id_cache
                        .get_or_init(|| {
                            env.get_method_id(
                                self.java_class.as_ref(),
                                "getRoot",
                                "()Landroid/view/View;",
                            )
                        })
                    {
                        Ok(v) => *v,
                        Err(e) => return Err(e.clone()),
                    };
                    todo!()
                }
            }
        }
    }
}
