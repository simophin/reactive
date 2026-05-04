use jni::errors::Result as JniResult;
use jni::objects::{JObject, JString, JValue, JValueOwned};
use jni::JNIEnv;

use crate::android::desc::{declare_jni_binding, JavaClassDescriptor, JavaMethodDescriptor};

pub mod activity {
    use super::*;
    declare_jni_binding! {
        class android.app.Activity {
            android.view.View findViewById(int);
            void setTitle(java.lang.CharSequence);
        }
    }
}

pub mod object {
    use super::*;
    declare_jni_binding! {
        class java.lang.Object {
            String toString();
        }
    }
}

pub mod view {
    use super::*;
    declare_jni_binding! {
        class android.view.View {
            void setEnabled(boolean);
            void setOnClickListener(android.view.View.OnClickListener);
            void setPadding(int, int, int, int);
            void measure(int, int);
            int getMeasuredWidth();
            int getMeasuredHeight();
            void layout(int, int, int, int);
            void setMeasuredDimension(int, int);
            void requestLayout();
            void setMinimumWidth(int);
            void setMinimumHeight(int);
        }
    }
}

pub mod view_group {
    use super::*;
    declare_jni_binding! {
        class android.view.ViewGroup {
            void addView(android.view.View);
            void removeView(android.view.View);
        }
    }
}

pub mod text_view {
    use super::*;
    declare_jni_binding! {
        class android.widget.TextView {
            void setText(java.lang.CharSequence);
            void setTextSize(float);
            void setTextAlignment(int);
            java.lang.CharSequence getText();
        }
    }
}

pub mod button {
    use super::*;
    declare_jni_binding! {
        class android.widget.Button {
        }
    }
}

pub mod progress_bar {
    use super::*;
    declare_jni_binding! {
        class android.widget.ProgressBar {
            void setProgress(int);
            void setMax(int);
            void setIndeterminate(boolean);
        }
    }
}

pub mod seek_bar {
    use super::*;
    declare_jni_binding! {
        class android.widget.SeekBar {
            void setProgress(int);
            int getProgress();
            void setMin(int);
            void setMax(int);
            void setOnSeekBarChangeListener(android.widget.SeekBar.OnSeekBarChangeListener);
        }
    }
}

pub mod linear_layout {
    use super::*;
    declare_jni_binding! {
        class android.widget.LinearLayout {
            void setOrientation(int);
            void setGravity(int);
        }
    }
}

pub mod frame_layout {
    use super::*;
    declare_jni_binding! {
        class android.widget.FrameLayout {
        }
    }
}

pub mod image_view {
    use super::*;
    declare_jni_binding! {
        class android.widget.ImageView {
            void setImageDrawable(android.graphics.drawable.Drawable);
            void setContentDescription(java.lang.CharSequence);
        }
    }
}

pub mod edit_text {
    use super::*;
    declare_jni_binding! {
        class android.widget.EditText {
            void setText(java.lang.CharSequence);
            void setTextSize(float);
            java.lang.CharSequence getText();
            void setSelection(int, int);
            void addTextChangedListener(android.text.TextWatcher);
        }
    }
}

pub fn call_method<'local, M, Args>(
    env: &mut JNIEnv<'local>,
    obj: &JObject<'local>,
    args: &[JValue<'local, 'local>],
) -> JniResult<JValueOwned<'local>>
where
    M: JavaMethodDescriptor<Args>,
{
    env.call_method(obj, M::NAME, M::SIGNATURE, args)
}

pub fn call_void<'local, M, Args>(
    env: &mut JNIEnv<'local>,
    obj: &JObject<'local>,
    args: &[JValue<'local, 'local>],
) -> JniResult<()>
where
    M: JavaMethodDescriptor<Args>,
{
    call_method::<M, Args>(env, obj, args).map(|_| ())
}

pub fn call_int<'local, M, Args>(
    env: &mut JNIEnv<'local>,
    obj: &JObject<'local>,
    args: &[JValue<'local, 'local>],
) -> JniResult<i32>
where
    M: JavaMethodDescriptor<Args>,
{
    call_method::<M, Args>(env, obj, args)?.i()
}

pub fn call_object<'local, M, Args>(
    env: &mut JNIEnv<'local>,
    obj: &JObject<'local>,
    args: &[JValue<'local, 'local>],
) -> JniResult<JObject<'local>>
where
    M: JavaMethodDescriptor<Args>,
{
    call_method::<M, Args>(env, obj, args)?.l()
}

pub fn new_object<'local, C>(
    env: &mut JNIEnv<'local>,
    signature: &str,
    args: &[JValue<'local, 'local>],
) -> JniResult<JObject<'local>>
where
    C: JavaClassDescriptor,
{
    env.new_object(C::FQ_NAME, signature, args)
}

pub fn new_java_string<'local>(
    env: &mut JNIEnv<'local>,
    value: &str,
) -> JniResult<JString<'local>> {
    env.new_string(value)
}
