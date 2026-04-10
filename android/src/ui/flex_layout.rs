use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use jni::objects::{JObject, JValue};
use jni::sys::{jint, jlong};
use ui_core::layout::algorithm::{
    AxisConstraint, LayoutHost, Measurement, Rect, Size, SizeConstraint,
};
use ui_core::layout::{
    compute_flex_layout, measure_flex_container_constrained, ChildLayoutInfo, CrossAxisAlignment,
};
use ui_core::{sync_children, ChildEntry, ChildrenHost};

use crate::bindings;
use crate::ui::view_component::AndroidView;

pub(crate) type ChildViewEntry = ChildEntry<AndroidView>;

#[derive(Clone)]
struct FlexData {
    children: Vec<ChildViewEntry>,
    vertical: bool,
    spacing: f32,
    cross_axis: CrossAxisAlignment,
}

impl Default for FlexData {
    fn default() -> Self {
        Self {
            children: Vec::new(),
            vertical: false,
            spacing: 0.0,
            cross_axis: CrossAxisAlignment::Start,
        }
    }
}

thread_local! {
    static FLEX_DATA: RefCell<HashMap<u64, FlexData>> = RefCell::new(HashMap::new());
}

static NEXT_LAYOUT_ID: AtomicU64 = AtomicU64::new(1);

dexer::dex_class! {
    #[java_class("com.reactive.ReactiveFlexLayout")]
    pub struct ReactiveFlexLayout {
        id: u64,
    }
    extends "android/widget/FrameLayout";

    #[constructor]
    pub fn init(
        _env: &mut jni::JNIEnv,
        #[class("android/content/Context")] _context: JObject,
        id: jlong,
    ) -> Self {
        Self { id: id as u64 }
    }

    #[method_override(name = "onMeasure")]
    pub fn on_measure(
        &mut self,
        env: &mut jni::JNIEnv,
        super_: dexer::SuperCaller,
        width_spec: jint,
        height_spec: jint,
    ) {
        let this = dexer::current_this(env).expect("current this");
        let data = data_snapshot(self.id);
        let child_infos: Vec<ChildLayoutInfo> =
            data.children.iter().map(|entry| entry.layout.clone()).collect();
        let host = AndroidFlexHost {
            children: &data.children,
        };

        let cross_size = if data.vertical {
            axis_size_from_measure_spec(width_spec).map(|size| (size, data.cross_axis))
        } else {
            axis_size_from_measure_spec(height_spec).map(|size| (size, data.cross_axis))
        };

        let measurement = measure_flex_container_constrained(
            &host,
            &child_infos,
            data.vertical,
            data.spacing,
            cross_size,
        );

        let measured_width = resolve_dimension(width_spec, measurement.natural.width, measurement.min.width);
        let measured_height =
            resolve_dimension(height_spec, measurement.natural.height, measurement.min.height);

        bindings::call_void::<
            bindings::view::setMeasuredDimension,
            (jni::sys::jint, jni::sys::jint),
        >(
            env,
            &this,
            &[JValue::Int(measured_width), JValue::Int(measured_height)],
        )
        .expect("set measured dimension");

        let _ = super_;
    }

    #[method_override(name = "onLayout")]
    pub fn on_layout(
        &mut self,
        _env: &mut jni::JNIEnv,
        super_: dexer::SuperCaller,
        _changed: jni::sys::jboolean,
        left: jint,
        top: jint,
        right: jint,
        bottom: jint,
    ) {
        let data = data_snapshot(self.id);
        let child_infos: Vec<ChildLayoutInfo> =
            data.children.iter().map(|entry| entry.layout.clone()).collect();
        let host = AndroidFlexHost {
            children: &data.children,
        };
        compute_flex_layout(
            &host,
            &child_infos,
            data.vertical,
            data.spacing,
            data.cross_axis,
            Size {
                width: (right - left) as f32,
                height: (bottom - top) as f32,
            },
        );
        let _ = super_;
    }
}

#[derive(Clone)]
pub struct AndroidFlexLayout {
    view: AndroidView,
    id: u64,
}

impl PartialEq for AndroidFlexLayout {
    fn eq(&self, other: &Self) -> bool {
        self.view == other.view
    }
}

impl Eq for AndroidFlexLayout {}

impl AndroidFlexLayout {
    pub fn new(ctx: &mut reactive_core::SetupContext) -> Self {
        let id = NEXT_LAYOUT_ID.fetch_add(1, Ordering::Relaxed);
        FLEX_DATA.with(|data| {
            data.borrow_mut().insert(id, FlexData::default());
        });
        ctx.on_cleanup(move || {
            FLEX_DATA.with(|data| {
                data.borrow_mut().remove(&id);
            });
        });

        let java_vm = AndroidView::java_vm();
        let mut env = java_vm
            .attach_current_thread_permanently()
            .expect("attach thread");
        let activity = AndroidView::activity();
        let activity_obj = env
            .new_local_ref(activity.as_obj())
            .expect("clone activity ref");
        let java_obj = ReactiveFlexLayout { id }
            .into_java(&mut env, activity_obj, id as jlong)
            .expect("create ReactiveFlexLayout");

        Self {
            view: AndroidView::new(&mut env, &java_obj),
            id,
        }
    }

    pub fn as_view(&self) -> AndroidView {
        self.view.clone()
    }

    pub fn set_flex_params(&self, vertical: bool, spacing: f32, cross_axis: CrossAxisAlignment) {
        FLEX_DATA.with(|data| {
            let mut data = data.borrow_mut();
            let entry = data
                .get_mut(&self.id)
                .expect("flex layout state should exist");
            entry.vertical = vertical;
            entry.spacing = spacing;
            entry.cross_axis = cross_axis;
        });
    }

    pub fn update_children(&self, entries: Vec<ChildViewEntry>) {
        FLEX_DATA.with(|data| {
            let mut data = data.borrow_mut();
            let entry = data
                .get_mut(&self.id)
                .expect("flex layout state should exist");
            sync_children(self, &mut entry.children, entries);
        });
    }
}

impl ChildrenHost<AndroidView> for AndroidFlexLayout {
    fn remove_child(&self, child: &AndroidView) {
        let mut env = self.view.env();
        bindings::call_void::<bindings::view_group::removeView, (jni::sys::jobject,)>(
            &mut env,
            self.view.as_obj(),
            &[JValue::Object(child.as_obj())],
        )
        .expect("remove child");
    }

    fn add_child(&self, child: &AndroidView, _after: Option<&AndroidView>) {
        let mut env = self.view.env();
        bindings::call_void::<bindings::view_group::addView, (jni::sys::jobject,)>(
            &mut env,
            self.view.as_obj(),
            &[JValue::Object(child.as_obj())],
        )
        .expect("add child");
    }

    fn invalidate_layout(&self) {
        let mut env = self.view.env();
        bindings::call_void::<bindings::view::requestLayout, ()>(&mut env, self.view.as_obj(), &[])
            .expect("request layout");
    }
}

#[derive(Clone)]
pub(crate) struct AndroidChildrenHost {
    pub(crate) parent: AndroidView,
}

impl ChildrenHost<AndroidView> for AndroidChildrenHost {
    fn remove_child(&self, child: &AndroidView) {
        let mut env = self.parent.env();
        bindings::call_void::<bindings::view_group::removeView, (jni::sys::jobject,)>(
            &mut env,
            self.parent.as_obj(),
            &[JValue::Object(child.as_obj())],
        )
        .expect("remove child");
    }

    fn add_child(&self, child: &AndroidView, _after: Option<&AndroidView>) {
        let mut env = self.parent.env();
        bindings::call_void::<bindings::view_group::addView, (jni::sys::jobject,)>(
            &mut env,
            self.parent.as_obj(),
            &[JValue::Object(child.as_obj())],
        )
        .expect("add child");
    }

    fn invalidate_layout(&self) {
        let mut env = self.parent.env();
        bindings::call_void::<bindings::view::requestLayout, ()>(
            &mut env,
            self.parent.as_obj(),
            &[],
        )
        .expect("request layout");
    }
}

struct AndroidFlexHost<'a> {
    children: &'a [ChildViewEntry],
}

impl LayoutHost for AndroidFlexHost<'_> {
    fn child_count(&self) -> usize {
        self.children.len()
    }

    fn measure_child(&self, index: usize, constraint: SizeConstraint) -> Measurement {
        let view = &self.children[index].native;
        let mut env = view.env();

        bindings::call_void::<bindings::view::measure, (jni::sys::jint, jni::sys::jint)>(
            &mut env,
            view.as_obj(),
            &[
                JValue::Int(android_measure_spec(constraint.width)),
                JValue::Int(android_measure_spec(constraint.height)),
            ],
        )
        .expect("measure child");

        let width = bindings::call_int::<bindings::view::getMeasuredWidth, ()>(
            &mut env,
            view.as_obj(),
            &[],
        )
        .expect("child measured width") as f32;
        let height = bindings::call_int::<bindings::view::getMeasuredHeight, ()>(
            &mut env,
            view.as_obj(),
            &[],
        )
        .expect("child measured height") as f32;

        Measurement {
            min: Size { width, height },
            natural: Size { width, height },
        }
    }

    fn place_child(&self, index: usize, frame: Rect) {
        let view = &self.children[index].native;
        let mut env = view.env();
        bindings::call_void::<
            bindings::view::layout,
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
                JValue::Int(frame.x as jint),
                JValue::Int(frame.y as jint),
                JValue::Int((frame.x + frame.width) as jint),
                JValue::Int((frame.y + frame.height) as jint),
            ],
        )
        .expect("layout child");
    }
}

fn data_snapshot(id: u64) -> FlexData {
    FLEX_DATA.with(|data| data.borrow().get(&id).cloned().unwrap_or_default())
}

fn axis_size_from_measure_spec(spec: jint) -> Option<f32> {
    match measure_spec_mode(spec) {
        0 => None,
        _ => Some(measure_spec_size(spec) as f32),
    }
}

fn resolve_dimension(spec: jint, natural: f32, min: f32) -> jint {
    match measure_spec_mode(spec) {
        0x4000_0000 => measure_spec_size(spec),
        mode if mode == 0x8000_0000u32 as jint => {
            natural.min(measure_spec_size(spec) as f32).max(min) as jint
        }
        _ => natural.max(min) as jint,
    }
}

fn android_measure_spec(constraint: AxisConstraint) -> jint {
    match constraint {
        AxisConstraint::Exact(size) => 0x4000_0000 | (size as jint & 0x3fff_ffff),
        AxisConstraint::AtMost(size) => 0x8000_0000u32 as jint | (size as jint & 0x3fff_ffff),
        AxisConstraint::Unconstrained => 0,
    }
}

fn measure_spec_mode(spec: jint) -> jint {
    spec & 0xc000_0000u32 as jint
}

fn measure_spec_size(spec: jint) -> jint {
    spec & 0x3fff_ffff
}
