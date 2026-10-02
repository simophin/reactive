//! `Flex` backed by `com.reactive.ReactiveFlexLayout`, a `ViewGroup` whose
//! `onMeasure`/`onLayout` call into Rust and run Taffy over its children.
//!
//! Taffy works in dp so modifier values mean the same on every platform; the
//! conversion to and from pixels happens only at this boundary.

use super::{FlexLayout, VIEW_REGISTRY_KEY};
use crate::android::JavaObject;
use crate::android::app::register;
use crate::widgets::taffy::FlexTaffyContainer;
use crate::widgets::{
    CommonFlex, CommonModifiers, Modifier, NativeView, NativeViewRegistry, SizeSpec,
};
use jni::JNIEnv;
use jni::objects::{JClass, JValue};
use jni::sys::{jint, jlong};
use reactive_core::{Component, ComponentId, SetupContext, Signal};
use std::cell::RefCell;
use std::ffi::c_void;
use std::rc::Rc;
use taffy::{AvailableSpace, RequestedAxis, RunMode, Size};

pub type Flex = CommonFlex<JavaObject>;

type ViewTree = FlexTaffyContainer<JavaObject>;

const CLASS: &str = "com/reactive/ReactiveFlexLayout";

struct FlexState {
    tree: RefCell<ViewTree>,
    /// Pixels per dp.
    density: f32,
}

pub(crate) fn register_natives(env: &mut JNIEnv) {
    register(
        env,
        CLASS,
        &[
            ("nativeMeasure", "(JII)J", native_measure as *mut c_void),
            ("nativeLayout", "(JII)V", native_layout as *mut c_void),
        ],
    );
}

// --- View.MeasureSpec ------------------------------------------------------

const MODE_MASK: i32 = 0b11 << 30;
const UNSPECIFIED: i32 = 0;
const EXACTLY: i32 = 1 << 30;
const AT_MOST: i32 = 2 << 30;

fn measure_spec(size: i32, mode: i32) -> i32 {
    (size.max(0) & !MODE_MASK) | mode
}

fn spec_mode(spec: i32) -> i32 {
    spec & MODE_MASK
}

fn spec_size(spec: i32) -> i32 {
    spec & !MODE_MASK
}

/// Equivalent of `View.resolveSize`.
fn resolve_size(desired: i32, spec: i32) -> i32 {
    match spec_mode(spec) {
        EXACTLY => spec_size(spec),
        AT_MOST => desired.min(spec_size(spec)),
        _ => desired,
    }
}

impl FlexState {
    fn to_px(&self, dp: f32) -> i32 {
        (dp * self.density).round() as i32
    }

    fn to_dp(&self, px: i32) -> f32 {
        px as f32 / self.density
    }
}

// --- Measuring children ------------------------------------------------------

/// Measures a child view through `View.measure`, all values in dp.
///
/// The View system has no min-content query, so min-content is approximated
/// by offering zero width (`AT_MOST 0`): views then report their minimum,
/// e.g. a button its `minWidth`, a text view zero.
fn measure_child(
    view: &JavaObject,
    density: f32,
    known: Size<Option<f32>>,
    available: Size<AvailableSpace>,
) -> Size<f32> {
    let spec = |known: Option<f32>, available: AvailableSpace| match (known, available) {
        (Some(size), _) => measure_spec((size * density).round() as i32, EXACTLY),
        (None, AvailableSpace::Definite(size)) => {
            measure_spec((size * density).floor() as i32, AT_MOST)
        }
        (None, AvailableSpace::MinContent) => measure_spec(0, AT_MOST),
        (None, AvailableSpace::MaxContent) => measure_spec(0, UNSPECIFIED),
    };

    view.call_void(
        "measure",
        "(II)V",
        &[
            JValue::Int(spec(known.width, available.width)),
            JValue::Int(spec(known.height, available.height)),
        ],
    );

    Size {
        width: known
            .width
            .unwrap_or_else(|| view.call_int("getMeasuredWidth") as f32 / density),
        height: known
            .height
            .unwrap_or_else(|| view.call_int("getMeasuredHeight") as f32 / density),
    }
}

// --- ReactiveFlexLayout natives ------------------------------------------------

extern "system" fn native_measure(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
    width_spec: jint,
    height_spec: jint,
) -> jlong {
    let state = unsafe { &*(handle as *const FlexState) };
    let mut tree = state.tree.borrow_mut();

    let (fixed_width, fixed_height) = tree.root_modifier().unwrap().get_size().read();
    let axis = |spec: i32, fixed: SizeSpec| {
        let known = match (spec_mode(spec), fixed) {
            (EXACTLY, _) => Some(state.to_dp(spec_size(spec))),
            (_, SizeSpec::Fixed(size)) => Some(size as f32),
            _ => None,
        };
        let available = match spec_mode(spec) {
            UNSPECIFIED => AvailableSpace::MaxContent,
            _ => AvailableSpace::Definite(state.to_dp(spec_size(spec))),
        };
        (known, available)
    };

    let (known_width, available_width) = axis(width_spec, fixed_width);
    let (known_height, available_height) = axis(height_spec, fixed_height);

    let output = tree.compute_layout(
        RunMode::ComputeSize,
        Size {
            width: known_width,
            height: known_height,
        },
        Size {
            width: available_width,
            height: available_height,
        },
        RequestedAxis::Both,
    );

    let width = resolve_size(
        (output.size.width * state.density).ceil() as i32,
        width_spec,
    );
    let height = resolve_size(
        (output.size.height * state.density).ceil() as i32,
        height_spec,
    );
    ((width as jlong) << 32) | (height as u32 as jlong)
}

extern "system" fn native_layout(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
    width: jint,
    height: jint,
) {
    let state = unsafe { &*(handle as *const FlexState) };

    // Collect placements first so no borrow is held while calling back into
    // the View system.
    let placements: Vec<_> = {
        let mut tree = state.tree.borrow_mut();
        let size = Size {
            width: state.to_dp(width),
            height: state.to_dp(height),
        };
        tree.compute_layout(
            RunMode::PerformLayout,
            size.map(Some),
            size.map(AvailableSpace::Definite),
            RequestedAxis::Both,
        );

        tree.iter()
            .filter_map(|(view, layout)| {
                let layout = layout?;
                // Round edges rather than sizes so adjacent children never
                // leave a gap or overlap by a pixel.
                let left = state.to_px(layout.location.x);
                let top = state.to_px(layout.location.y);
                let right = state.to_px(layout.location.x + layout.size.width);
                let bottom = state.to_px(layout.location.y + layout.size.height);
                Some((view.clone(), left, top, right, bottom))
            })
            .collect()
    };

    for (view, left, top, right, bottom) in placements {
        // Re-measure at the final size: views such as TextView lay out their
        // content during measure.
        view.call_void(
            "measure",
            "(II)V",
            &[
                JValue::Int(measure_spec(right - left, EXACTLY)),
                JValue::Int(measure_spec(bottom - top, EXACTLY)),
            ],
        );
        view.call_void(
            "layout",
            "(IIII)V",
            &[
                JValue::Int(left),
                JValue::Int(top),
                JValue::Int(right),
                JValue::Int(bottom),
            ],
        );
    }
}

// --- Component -------------------------------------------------------------------

struct ViewRegistry {
    state: Rc<FlexState>,
    layout: FlexLayout,
}

impl NativeViewRegistry<JavaObject> for ViewRegistry {
    fn update_view(&self, component_id: ComponentId, view: JavaObject, modifier: Modifier) {
        let parent = view.call_object("getParent", "()Landroid/view/ViewParent;", &[]);
        if parent.is_none_or(|parent| !self.layout.is_same(parent.as_obj())) {
            self.layout.call_void(
                "addView",
                "(Landroid/view/View;)V",
                &[JValue::Object(view.as_obj())],
            );
        }

        self.state
            .tree
            .borrow_mut()
            .insert_child(view, modifier, component_id);
        self.layout.call_void("requestLayout", "()V", &[]);
    }

    fn clear_view(&self, _component_id: ComponentId, view: JavaObject) {
        self.state.tree.borrow_mut().remove_child(&view);
        self.layout.call_void(
            "removeView",
            "(Landroid/view/View;)V",
            &[JValue::Object(view.as_obj())],
        );
    }
}

fn display_density(ctx: &SetupContext) -> f32 {
    super::activity(ctx)
        .call_object("getResources", "()Landroid/content/res/Resources;", &[])
        .and_then(|r| r.call_object("getDisplayMetrics", "()Landroid/util/DisplayMetrics;", &[]))
        .expect("display metrics")
        .get_float_field("density")
}

impl Component for Flex {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let Self {
            props,
            children,
            modifier,
            ..
        } = *self;

        let layout = FlexLayout(super::new_view(ctx, CLASS));
        let density = display_density(ctx);
        let mut tree = ViewTree::new(ctx.scope(), props.read(), move |view, known, available| {
            measure_child(view, density, known, available)
        });
        tree.set_root(layout.0.clone(), modifier.clone(), ctx.component_id());

        let state = Rc::new(FlexState {
            tree: RefCell::new(tree),
            density,
        });

        layout.set_long_field("nativeHandle", Rc::as_ptr(&state) as jlong);
        ctx.on_cleanup({
            let layout = layout.clone();
            let state = state.clone();
            move || {
                layout.set_long_field("nativeHandle", 0);
                drop(state);
            }
        });

        NativeView::new(
            {
                let layout = layout.clone();
                move |_| layout
            },
            Into::into,
            |_, _| {},
            modifier,
            &VIEW_REGISTRY_KEY,
        )
        .setup_in_component(ctx);

        ctx.create_effect({
            let state = state.clone();
            let layout = layout.clone();
            move |_, _| {
                state.tree.borrow_mut().set_props(props.read());
                layout.call_void("requestLayout", "()V", &[]);
            }
        });

        for child in children {
            let registry: Rc<dyn NativeViewRegistry<_>> = Rc::new(ViewRegistry {
                state: state.clone(),
                layout: layout.clone(),
            });
            ctx.child(move |child_ctx: &mut SetupContext| {
                child_ctx.set_static_context(&VIEW_REGISTRY_KEY, registry);
                child_ctx.boxed_child(child);
            });
        }
    }
}
