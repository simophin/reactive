pub mod com {
    pub mod kogan {
        pub mod data {
            pub mod local {
                pub mod prefs {
                    pub mod kogan_preferences {
                        pub struct KoganPreference {
                            java_class: ::jni::objects::GlobalRef,
                            r#new_instance_method_id_cache: std::cell::OnceCell<
                                ::jni::errors::Result<::jni::sys::jmethodID>,
                            >,
                            r#get_app_config_last_fetch_timestamp_method_id_cache: std::cell::OnceCell<
                                ::jni::errors::Result<::jni::sys::jmethodID>,
                            >,
                            r#set_app_config_last_fetch_timestamp_method_id_cache: std::cell::OnceCell<
                                ::jni::errors::Result<::jni::sys::jmethodID>,
                            >,
                            r#get_app_install_version_code_method_id_cache: std::cell::OnceCell<
                                ::jni::errors::Result<::jni::sys::jmethodID>,
                            >,
                            r#set_app_install_version_code_method_id_cache: std::cell::OnceCell<
                                ::jni::errors::Result<::jni::sys::jmethodID>,
                            >,
                            r#get_previously_should_show_request_permission_rationale_method_id_cache: std::cell::OnceCell<
                                ::jni::errors::Result<::jni::sys::jmethodID>,
                            >,
                            r#set_previously_should_show_request_permission_rationale_method_id_cache: std::cell::OnceCell<
                                ::jni::errors::Result<::jni::sys::jmethodID>,
                            >,
                            r#get_feature_flag_value_method_id_cache: std::cell::OnceCell<
                                ::jni::errors::Result<::jni::sys::jmethodID>,
                            >,
                            r#set_feature_flag_value_method_id_cache: std::cell::OnceCell<
                                ::jni::errors::Result<::jni::sys::jmethodID>,
                            >,
                            r#get_viewed_arrival_message_i_ds_method_id_cache: std::cell::OnceCell<
                                ::jni::errors::Result<::jni::sys::jmethodID>,
                            >,
                            r#add_arrival_message_viewed_id_method_id_cache: std::cell::OnceCell<
                                ::jni::errors::Result<::jni::sys::jmethodID>,
                            >,
                            r#get_grid_view_enabled_method_id_cache: std::cell::OnceCell<
                                ::jni::errors::Result<::jni::sys::jmethodID>,
                            >,
                            r#set_grid_view_enabled_method_id_cache: std::cell::OnceCell<
                                ::jni::errors::Result<::jni::sys::jmethodID>,
                            >,
                        }
                        impl KoganPreference {
                            pub fn new<'local>(
                                env: &mut ::jni::JNIEnv<'local>,
                            ) -> ::jni::errors::Result<Self> {
                                let java_class = env
                                    .new_global_ref(
                                        env
                                            .find_class("com/kogan/data/local/prefs/KoganPreferences")?,
                                    );
                                Ok(Self {
                                    java_class,
                                    r#new_instance_method_id_cache: Default::defult(),
                                    r#get_app_config_last_fetch_timestamp_method_id_cache: Default::defult(),
                                    r#set_app_config_last_fetch_timestamp_method_id_cache: Default::defult(),
                                    r#get_app_install_version_code_method_id_cache: Default::defult(),
                                    r#set_app_install_version_code_method_id_cache: Default::defult(),
                                    r#get_previously_should_show_request_permission_rationale_method_id_cache: Default::defult(),
                                    r#set_previously_should_show_request_permission_rationale_method_id_cache: Default::defult(),
                                    r#get_feature_flag_value_method_id_cache: Default::defult(),
                                    r#set_feature_flag_value_method_id_cache: Default::defult(),
                                    r#get_viewed_arrival_message_i_ds_method_id_cache: Default::defult(),
                                    r#add_arrival_message_viewed_id_method_id_cache: Default::defult(),
                                    r#get_grid_view_enabled_method_id_cache: Default::defult(),
                                    r#set_grid_view_enabled_method_id_cache: Default::defult(),
                                })
                            }
                            pub fn r#new_instance<'local>(
                                &self,
                                env: &mut ::jni::JNIEnv<'local>,
                                obj: &::jni::objects::JObject<'_>,
                                arg0: &::jni::objects::JObject<'_>,
                                arg1: &::jni::objects::JObject<'_>,
                            ) -> ::jni::errors::Result<()> {
                                let method_id = self
                                    .r#new_instance_method_id_cache
                                    .get_or_init(|| {
                                        env.get_method_id(
                                            obj,
                                            "<init>",
                                            "(Landroid/content/Context;Lcom/koganconnect/api/ApiClient;)V",
                                        )
                                    })?
                                    .clone();
                                todo!()
                            }
                            pub fn r#get_app_config_last_fetch_timestamp<'local>(
                                &self,
                                env: &mut ::jni::JNIEnv<'local>,
                                obj: &::jni::objects::JObject<'_>,
                                arg0: &::jni::objects::JObject<'_>,
                            ) -> ::jni::errors::Result<
                                jni::objects::AutoLocal<
                                    'local,
                                    ::jni::objects::JObject<'local>,
                                >,
                            > {
                                let method_id = self
                                    .r#get_app_config_last_fetch_timestamp_method_id_cache
                                    .get_or_init(|| {
                                        env.get_method_id(
                                            obj,
                                            "getAppConfigLastFetchTimestamp",
                                            "(Lkotlin/coroutines/Continuation;)Ljava/lang/Object;",
                                        )
                                    })?
                                    .clone();
                                todo!()
                            }
                            pub fn r#set_app_config_last_fetch_timestamp<'local>(
                                &self,
                                env: &mut ::jni::JNIEnv<'local>,
                                obj: &::jni::objects::JObject<'_>,
                                arg0: ::jni::sys::jlong,
                            ) -> ::jni::errors::Result<()> {
                                let method_id = self
                                    .r#set_app_config_last_fetch_timestamp_method_id_cache
                                    .get_or_init(|| {
                                        env.get_method_id(
                                            obj,
                                            "setAppConfigLastFetchTimestamp",
                                            "(J)V",
                                        )
                                    })?
                                    .clone();
                                todo!()
                            }
                            pub fn r#get_app_install_version_code<'local>(
                                &self,
                                env: &mut ::jni::JNIEnv<'local>,
                                obj: &::jni::objects::JObject<'_>,
                                arg0: &::jni::objects::JObject<'_>,
                            ) -> ::jni::errors::Result<
                                jni::objects::AutoLocal<
                                    'local,
                                    ::jni::objects::JObject<'local>,
                                >,
                            > {
                                let method_id = self
                                    .r#get_app_install_version_code_method_id_cache
                                    .get_or_init(|| {
                                        env.get_method_id(
                                            obj,
                                            "getAppInstallVersionCode",
                                            "(Lkotlin/coroutines/Continuation;)Ljava/lang/Object;",
                                        )
                                    })?
                                    .clone();
                                todo!()
                            }
                            pub fn r#set_app_install_version_code<'local>(
                                &self,
                                env: &mut ::jni::JNIEnv<'local>,
                                obj: &::jni::objects::JObject<'_>,
                                arg0: ::jni::sys::jint,
                            ) -> ::jni::errors::Result<()> {
                                let method_id = self
                                    .r#set_app_install_version_code_method_id_cache
                                    .get_or_init(|| {
                                        env.get_method_id(obj, "setAppInstallVersionCode", "(I)V")
                                    })?
                                    .clone();
                                todo!()
                            }
                            pub fn r#get_previously_should_show_request_permission_rationale<
                                'local,
                            >(
                                &self,
                                env: &mut ::jni::JNIEnv<'local>,
                                obj: &::jni::objects::JObject<'_>,
                                arg0: &::jni::objects::JObject<'_>,
                            ) -> ::jni::errors::Result<
                                jni::objects::AutoLocal<
                                    'local,
                                    ::jni::objects::JObject<'local>,
                                >,
                            > {
                                let method_id = self
                                    .r#get_previously_should_show_request_permission_rationale_method_id_cache
                                    .get_or_init(|| {
                                        env.get_method_id(
                                            obj,
                                            "getPreviouslyShouldShowRequestPermissionRationale",
                                            "(Lkotlin/coroutines/Continuation;)Ljava/lang/Object;",
                                        )
                                    })?
                                    .clone();
                                todo!()
                            }
                            pub fn r#set_previously_should_show_request_permission_rationale<
                                'local,
                            >(
                                &self,
                                env: &mut ::jni::JNIEnv<'local>,
                                obj: &::jni::objects::JObject<'_>,
                                arg0: &::jni::objects::JObject<'_>,
                            ) -> ::jni::errors::Result<
                                jni::objects::AutoLocal<
                                    'local,
                                    ::jni::objects::JObject<'local>,
                                >,
                            > {
                                let method_id = self
                                    .r#set_previously_should_show_request_permission_rationale_method_id_cache
                                    .get_or_init(|| {
                                        env.get_method_id(
                                            obj,
                                            "setPreviouslyShouldShowRequestPermissionRationale",
                                            "(Lkotlin/coroutines/Continuation;)Ljava/lang/Object;",
                                        )
                                    })?
                                    .clone();
                                todo!()
                            }
                            pub fn r#get_feature_flag_value<'local>(
                                &self,
                                env: &mut ::jni::JNIEnv<'local>,
                                obj: &::jni::objects::JObject<'_>,
                                arg0: &::jni::objects::JObject<'_>,
                            ) -> ::jni::errors::Result<
                                jni::objects::AutoLocal<
                                    'local,
                                    ::jni::objects::JObject<'local>,
                                >,
                            > {
                                let method_id = self
                                    .r#get_feature_flag_value_method_id_cache
                                    .get_or_init(|| {
                                        env.get_method_id(
                                            obj,
                                            "getFeatureFlagValue",
                                            "(Lcom/kogan/data/local/prefs/FeatureFlag;)Lkotlinx/coroutines/flow/Flow;",
                                        )
                                    })?
                                    .clone();
                                todo!()
                            }
                            pub fn r#set_feature_flag_value<'local>(
                                &self,
                                env: &mut ::jni::JNIEnv<'local>,
                                obj: &::jni::objects::JObject<'_>,
                                arg0: &::jni::objects::JObject<'_>,
                                arg1: ::jni::sys::jboolean,
                                arg2: &::jni::objects::JObject<'_>,
                            ) -> ::jni::errors::Result<
                                jni::objects::AutoLocal<
                                    'local,
                                    ::jni::objects::JObject<'local>,
                                >,
                            > {
                                let method_id = self
                                    .r#set_feature_flag_value_method_id_cache
                                    .get_or_init(|| {
                                        env.get_method_id(
                                            obj,
                                            "setFeatureFlagValue",
                                            "(Landroidx/datastore/preferences/core/Preferences$Key;ZLkotlin/coroutines/Continuation;)Ljava/lang/Object;",
                                        )
                                    })?
                                    .clone();
                                todo!()
                            }
                            pub fn r#get_viewed_arrival_message_i_ds<'local>(
                                &self,
                                env: &mut ::jni::JNIEnv<'local>,
                                obj: &::jni::objects::JObject<'_>,
                                arg0: &::jni::objects::JObject<'_>,
                            ) -> ::jni::errors::Result<
                                jni::objects::AutoLocal<
                                    'local,
                                    ::jni::objects::JObject<'local>,
                                >,
                            > {
                                let method_id = self
                                    .r#get_viewed_arrival_message_i_ds_method_id_cache
                                    .get_or_init(|| {
                                        env.get_method_id(
                                            obj,
                                            "getViewedArrivalMessageIDs",
                                            "(Lkotlin/coroutines/Continuation;)Ljava/lang/Object;",
                                        )
                                    })?
                                    .clone();
                                todo!()
                            }
                            pub fn r#add_arrival_message_viewed_id<'local>(
                                &self,
                                env: &mut ::jni::JNIEnv<'local>,
                                obj: &::jni::objects::JObject<'_>,
                                arg0: &::jni::objects::JString<'_>,
                                arg1: &::jni::objects::JObject<'_>,
                            ) -> ::jni::errors::Result<
                                jni::objects::AutoLocal<
                                    'local,
                                    ::jni::objects::JObject<'local>,
                                >,
                            > {
                                let method_id = self
                                    .r#add_arrival_message_viewed_id_method_id_cache
                                    .get_or_init(|| {
                                        env.get_method_id(
                                            obj,
                                            "addArrivalMessageViewedID",
                                            "(Ljava/lang/String;Lkotlin/coroutines/Continuation;)Ljava/lang/Object;",
                                        )
                                    })?
                                    .clone();
                                todo!()
                            }
                            pub fn r#get_grid_view_enabled<'local>(
                                &self,
                                env: &mut ::jni::JNIEnv<'local>,
                                obj: &::jni::objects::JObject<'_>,
                                arg0: ::jni::sys::jboolean,
                                arg1: &::jni::objects::JObject<'_>,
                            ) -> ::jni::errors::Result<
                                jni::objects::AutoLocal<
                                    'local,
                                    ::jni::objects::JObject<'local>,
                                >,
                            > {
                                let method_id = self
                                    .r#get_grid_view_enabled_method_id_cache
                                    .get_or_init(|| {
                                        env.get_method_id(
                                            obj,
                                            "getGridViewEnabled",
                                            "(ZLkotlin/coroutines/Continuation;)Ljava/lang/Object;",
                                        )
                                    })?
                                    .clone();
                                todo!()
                            }
                            pub fn r#set_grid_view_enabled<'local>(
                                &self,
                                env: &mut ::jni::JNIEnv<'local>,
                                obj: &::jni::objects::JObject<'_>,
                                arg0: ::jni::sys::jboolean,
                                arg1: &::jni::objects::JObject<'_>,
                            ) -> ::jni::errors::Result<
                                jni::objects::AutoLocal<
                                    'local,
                                    ::jni::objects::JObject<'local>,
                                >,
                            > {
                                let method_id = self
                                    .r#set_grid_view_enabled_method_id_cache
                                    .get_or_init(|| {
                                        env.get_method_id(
                                            obj,
                                            "setGridViewEnabled",
                                            "(ZLkotlin/coroutines/Continuation;)Ljava/lang/Object;",
                                        )
                                    })?
                                    .clone();
                                todo!()
                            }
                        }
                    }
                }
            }
        }
    }
}
