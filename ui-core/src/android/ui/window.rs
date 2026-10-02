use super::{ContentFrame, VIEW_REGISTRY_KEY};
use crate::android::JavaObject;
use crate::widgets::{CommonWindow, Modifier, NativeViewRegistry};
use jni::objects::JValue;
use reactive_core::{Component, ComponentId, SetupContext, Signal};
use std::rc::Rc;

/// On Android the window is the hosting activity: its content view and title.
/// The initial size is ignored.
pub type Window = CommonWindow<JavaObject>;

struct ContentRegistry(ContentFrame);

impl NativeViewRegistry<JavaObject> for ContentRegistry {
    fn update_view(&self, _component_id: ComponentId, view: JavaObject, _modifier: Modifier) {
        let parent = view.call_object("getParent", "()Landroid/view/ViewParent;", &[]);
        if parent.is_none_or(|parent| !self.0.is_same(parent.as_obj())) {
            self.0.call_void(
                "addView",
                "(Landroid/view/View;)V",
                &[JValue::Object(view.as_obj())],
            );
        }
    }

    fn clear_view(&self, _component_id: ComponentId, view: JavaObject) {
        self.0.call_void(
            "removeView",
            "(Landroid/view/View;)V",
            &[JValue::Object(view.as_obj())],
        );
    }
}

impl Component for Window {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let Self { title, child, .. } = *self;
        let activity = super::activity(ctx);

        // A FrameLayout gives the child the full content area (its default
        // layout params are MATCH_PARENT) and, with fitsSystemWindows, keeps it
        // clear of the system bars under the enforced edge-to-edge mode.
        let frame = ContentFrame(super::new_view(ctx, "android/widget/FrameLayout"));
        frame.call_void("setFitsSystemWindows", "(Z)V", &[JValue::Bool(1)]);
        activity.call_void(
            "setContentView",
            "(Landroid/view/View;)V",
            &[JValue::Object(frame.as_obj())],
        );

        ctx.create_effect(move |_, _| activity.set_text("setTitle", &title.read()));

        ctx.set_static_context(
            &VIEW_REGISTRY_KEY,
            Rc::new(ContentRegistry(frame)) as Rc<dyn NativeViewRegistry<_>>,
        );
        ctx.boxed_child(child);
    }
}
