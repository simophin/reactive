use dexer::dex_class;
use jni::{JNIEnv, objects::{JObject, GlobalRef}};
use ui_core::layout::algorithm::{LayoutHost, Measurement, Rect, Size, SizeConstraint};
use ui_core::layout::{ChildLayoutInfo, compute_flex_layout, measure_flex_container_constrained};
use ui_core::ChildrenHost;
use crate::ui::view_component::AndroidView;
use crate::ui::context::ChildViewEntry;
use std::cell::RefCell;

#[dex_class]
#[java_class = "com.reactive.ReactiveFlexLayout"]
pub struct AndroidFlexLayout {
    pub(crate) children: RefCell<Vec<ChildViewEntry>>,
    pub(crate) vertical: RefCell<bool>,
    pub(crate) spacing: RefCell<f32>,
    pub(crate) cross_axis: RefCell<ui_core::layout::CrossAxisAlignment>,
}

impl AndroidFlexLayout {
    #[constructor]
    pub fn init(_env: &mut JNIEnv, context: JObject) {
        // Context used for ViewGroup initialization
    }

    #[override(name = "onMeasure")]
    pub fn on_measure(&mut self, env: &mut JNIEnv, super_: dexer::SuperCaller, width_spec: i32, height_spec: i32) {
        let children = self.children.borrow();
        let vertical = *self.vertical.borrow();
        let spacing = *self.spacing.borrow();
        let cross_axis = *self.cross_axis.borrow();

        let child_infos: Vec<ChildLayoutInfo> = children.iter().map(|e| e.layout.clone()).collect();

        let host = AndroidFlexHost { children: &children };

        // SimplifiedAndroid MeasureSpec conversion
        let cross_size = if vertical { Some((width_spec as f32, cross_axis)) } else { Some((height_spec as f32, cross_axis)) };

        let m = measure_flex_container_constrained(
            &host,
            &child_infos,
            vertical,
            spacing,
            cross_size,
        );

        // Call setMeasuredDimension via JNI
        env.call_method(
            env.as_obj(),
            "setMeasuredDimension",
            "(II)V",
            &[m.natural.width as i32, m.natural.height as i32]
        ).unwrap();
    }

    #[override(name = "onLayout")]
    pub fn on_layout(&mut self, env: &mut JNIEnv, super_: dexer::SuperCaller, changed: bool, l: i32, t: i32, r: i32, b: i32) {
        let children = self.children.borrow();
        let vertical = *self.vertical.borrow();
        let spacing = *self.spacing.borrow();
        let cross_axis = *self.cross_axis.borrow();

        let available = Size {
            width: (r - l) as f32,
            height: (b - t) as f32,
        };

        let child_infos: Vec<ChildLayoutInfo> = children.iter().map(|e| e.layout.clone()).collect();
        let host = AndroidFlexHost { children: &children };

        compute_flex_layout(
            &host,
            &child_infos,
            vertical,
            spacing,
            cross_axis,
            available,
        );
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

        // Call view.measure(widthSpec, heightSpec) via JNI
        // This requires a helper in Kotlin or a complex JNI call to build MeasureSpec
        Measurement {
            min: Size { width: 0.0, height: 0.0 },
            natural: Size { width: 100.0, height: 100.0 }, // Stub
        }
    }

    fn place_child(&self, index: usize, frame: Rect) {
        let view = &self.children[index].native;
        let mut env = view.env();
        env.call_method(
            view.as_obj(),
            "layout",
            "(IIII)V",
            &[frame.x as i32, frame.y as i32, (frame.x + frame.width) as i32, (frame.y + frame.height) as i32],
        ).unwrap();
    }
}

impl ChildrenHost<AndroidView> for AndroidFlexLayout {
    fn remove_child(&self, child: &AndroidView) {
        let mut env = child.env();
        // Logic to find parent and removeView
    }

    fn add_child(&self, child: &AndroidView, _after: Option<&AndroidView>) {
        let mut env = child.env();
        // Logic to addView
    }

    fn invalidate_layout(&self) {
        // call requestLayout() via JNI
    }
}
