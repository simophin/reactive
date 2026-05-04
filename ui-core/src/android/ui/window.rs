use jni::objects::{JObject, JValue};
use reactive_core::{BoxedComponent, Component, SetupContext, Signal};
use ui_core::widgets::Window;

use crate::android::bindings;
use crate::android::ui::flex_layout::AndroidChildrenHost;
use crate::android::ui::view_component::{AndroidView, AndroidViewBuilder, CHILDREN_VIEWS};

const ANDROID_CONTENT_VIEW_ID: i32 = 0x0102_0002;

pub struct AndroidWindow {
    child: BoxedComponent,
    title: Box<dyn Signal<Value = String>>,
    _initial_width: f64,
    _initial_height: f64,
}

impl Window for AndroidWindow {
    fn new(
        title: impl Signal<Value = String> + 'static,
        child: impl Component + 'static,
        width: f64,
        height: f64,
    ) -> Self {
        Self {
            child: Box::new(child),
            title: Box::new(title),
            _initial_width: width,
            _initial_height: height,
        }
    }
}

impl Component for AndroidWindow {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let Self { child, title, .. } = *self;

        let activity = AndroidView::activity();
        let activity_for_view = activity.clone();
        let activity_for_title = activity.clone();
        let container = AndroidViewBuilder::create_with_child(
            move |_ctx| {
                let mut env = activity_for_view.env();
                let content =
                    bindings::call_object::<bindings::activity::findViewById, (jni::sys::jint,)>(
                        &mut env,
                        activity_for_view.as_obj(),
                        &[JValue::Int(ANDROID_CONTENT_VIEW_ID)],
                    )
                    .expect("find activity content view");
                AndroidView::new(&mut env, &content)
            },
            |v| v,
            child,
        )
        .setup(ctx);

        ctx.create_effect(move |_, _| {
            let mut env = activity_for_title.env();
            let java_title = bindings::new_java_string(&mut env, &title.read()).expect("title");
            let java_title_obj = JObject::from(java_title);
            bindings::call_void::<bindings::activity::setTitle, (jni::sys::jobject,)>(
                &mut env,
                activity_for_title.as_obj(),
                &[JValue::Object(&java_title_obj)],
            )
            .expect("set activity title");
        });

        if let Some(children_views) = ctx.use_context(&CHILDREN_VIEWS) {
            ctx.create_effect(
                move |_, prev: Option<Vec<ui_core::ChildEntry<AndroidView>>>| {
                    let next = children_views
                        .read()
                        .iter()
                        .filter_map(|slot| slot.read())
                        .take(1)
                        .collect::<Vec<_>>();
                    let mut current = prev.unwrap_or_default();
                    let host = AndroidChildrenHost {
                        parent: container.clone(),
                    };
                    ui_core::sync_children(&host, &mut current, next);
                    current
                },
            );
        }
    }
}
