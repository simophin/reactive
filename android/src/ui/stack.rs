use jni::objects::JValue;
use reactive_core::{BoxedComponent, Component, SetupContext, Signal};
use ui_core::layout::{Alignment, BoxModifier, ChildLayoutInfo};
use ui_core::widgets::Stack;
use ui_core::{sync_children, ChildEntry};

use crate::bindings;
use crate::ui::flex_layout::AndroidChildrenHost;
use crate::ui::view_component::{AndroidView, AndroidViewBuilder, CHILDREN_VIEWS};

pub struct AndroidStack {
    children: Vec<BoxedComponent>,
    alignment: Option<Box<dyn Signal<Value = Alignment>>>,
}

impl Stack for AndroidStack {
    fn new() -> Self {
        Self {
            children: Vec::new(),
            alignment: None,
        }
    }

    fn alignment(mut self, alignment: impl Signal<Value = Alignment> + 'static) -> Self {
        self.alignment = Some(Box::new(alignment));
        self
    }

    fn child(mut self, child: impl Component + 'static) -> Self {
        self.children.push(Box::new(child));
        self
    }
}

impl Component for AndroidStack {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let Self {
            children,
            alignment,
        } = *self;

        let builder = children.into_iter().fold(
            AndroidViewBuilder::create_multiple_child(
                |_ctx| {
                    let java_vm = AndroidView::java_vm();
                    let mut env = java_vm
                        .attach_current_thread_permanently()
                        .expect("attach thread");
                    let activity = AndroidView::activity();
                    let frame = bindings::new_object::<bindings::frame_layout::FrameLayout>(
                        &mut env,
                        "(Landroid/content/Context;)V",
                        &[JValue::Object(activity.as_obj())],
                    )
                    .expect("create FrameLayout");
                    AndroidView::new(&mut env, &frame)
                },
                |v| v,
            ),
            |builder, child| builder.add_child(child),
        );

        let container = builder.setup(ctx);
        if let Some(children_views) = ctx.use_context(&CHILDREN_VIEWS) {
            ctx.create_effect(move |_, prev: Option<Vec<ChildEntry<AndroidView>>>| {
                let next = children_views
                    .read()
                    .iter()
                    .filter_map(|slot| slot.read())
                    .collect::<Vec<_>>();
                let mut current = prev.unwrap_or_default();
                let host = AndroidChildrenHost {
                    parent: container.clone(),
                };
                sync_children(&host, &mut current, next);

                let active_alignment = alignment
                    .as_ref()
                    .map(|signal| signal.read())
                    .unwrap_or_default();
                for entry in &current {
                    apply_stack_layout(&entry.native, &entry.layout, active_alignment);
                }
                current
            });
        }
    }
}

fn apply_stack_layout(view: &AndroidView, layout: &ChildLayoutInfo, alignment: Alignment) {
    let mut padding_left = 0;
    let mut padding_top = 0;
    let mut padding_right = 0;
    let mut padding_bottom = 0;
    let mut min_width = 0;
    let mut min_height = 0;

    for modifier in &layout.box_modifiers.modifiers {
        match modifier {
            BoxModifier::Padding(insets) => {
                padding_left += insets.left as i32;
                padding_top += insets.top as i32;
                padding_right += insets.right as i32;
                padding_bottom += insets.bottom as i32;
            }
            BoxModifier::SizedBox { width, height } => {
                min_width = width.unwrap_or_default() as i32;
                min_height = height.unwrap_or_default() as i32;
            }
            BoxModifier::Align(_) => {}
        }
    }

    match alignment {
        Alignment::TopLeading | Alignment::Leading | Alignment::BottomLeading => {
            padding_right += 0;
        }
        Alignment::Top | Alignment::Center | Alignment::Bottom => {}
        Alignment::TopTrailing | Alignment::Trailing | Alignment::BottomTrailing => {
            padding_left += 0;
        }
    }

    let mut env = view.env();
    bindings::call_void::<
        bindings::view::setPadding,
        (
            jni::sys::jint,
            jni::sys::jint,
            jni::sys::jint,
            jni::sys::jint,
        ),
    >(
        &mut env,
        view.as_obj(),
        &[
            JValue::Int(padding_left),
            JValue::Int(padding_top),
            JValue::Int(padding_right),
            JValue::Int(padding_bottom),
        ],
    )
    .expect("set stack child padding");
    bindings::call_void::<bindings::view::setMinimumWidth, (jni::sys::jint,)>(
        &mut env,
        view.as_obj(),
        &[JValue::Int(min_width)],
    )
    .expect("set stack child min width");
    bindings::call_void::<bindings::view::setMinimumHeight, (jni::sys::jint,)>(
        &mut env,
        view.as_obj(),
        &[JValue::Int(min_height)],
    )
    .expect("set stack child min height");
}
