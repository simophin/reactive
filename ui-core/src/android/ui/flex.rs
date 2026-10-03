//! `Flex` as a [`ViewGroupLayout`] that runs Taffy over the children of a
//! `ReactiveLayout`.
//!
//! Taffy works in dp so modifier values mean the same on every platform; the
//! conversion to and from pixels happens only at this boundary.

use super::layout::{
    ViewGroupLayout, attach, layout_view, measure_spec, measure_view, new_layout_view, resolve_size,
};
use super::{FlexLayout, VIEW_REGISTRY_KEY};
use crate::android::JavaObject;
use crate::widgets::taffy::FlexTaffyContainer;
use crate::widgets::{
    CommonFlex, CommonModifiers, Modifier, NativeView, NativeViewRegistry, SizeSpec,
};
use jni::objects::JValue;
use reactive_core::{Component, ComponentId, SetupContext, Signal};
use std::cell::RefCell;
use std::rc::Rc;
use taffy::{AvailableSpace, RequestedAxis, RunMode, Size};

pub type Flex = CommonFlex<JavaObject>;

type ViewTree = FlexTaffyContainer<JavaObject>;

struct FlexState {
    tree: RefCell<ViewTree>,
    /// Pixels per dp.
    density: f32,
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
    use measure_spec::{AT_MOST, EXACTLY, UNSPECIFIED};
    let spec = |known: Option<f32>, available: AvailableSpace| match (known, available) {
        (Some(size), _) => measure_spec::make((size * density).round() as i32, EXACTLY),
        (None, AvailableSpace::Definite(size)) => {
            measure_spec::make((size * density).floor() as i32, AT_MOST)
        }
        (None, AvailableSpace::MinContent) => measure_spec::make(0, AT_MOST),
        (None, AvailableSpace::MaxContent) => measure_spec::make(0, UNSPECIFIED),
    };

    let (width, height) = measure_view(
        view,
        spec(known.width, available.width),
        spec(known.height, available.height),
    );

    Size {
        width: known.width.unwrap_or(width as f32 / density),
        height: known.height.unwrap_or(height as f32 / density),
    }
}

// --- Measure and layout passes -------------------------------------------------

impl ViewGroupLayout for FlexState {
    fn measure(&self, width_spec: i32, height_spec: i32) -> (i32, i32) {
        let mut tree = self.tree.borrow_mut();

        let (fixed_width, fixed_height) = tree.root_modifier().unwrap().get_size().read();
        let axis = |spec: i32, fixed: SizeSpec| {
            let known = match (measure_spec::mode(spec), fixed) {
                (measure_spec::EXACTLY, _) => Some(self.to_dp(measure_spec::size(spec))),
                (_, SizeSpec::Fixed(size)) => Some(size as f32),
                _ => None,
            };
            let available = match measure_spec::mode(spec) {
                measure_spec::UNSPECIFIED => AvailableSpace::MaxContent,
                _ => AvailableSpace::Definite(self.to_dp(measure_spec::size(spec))),
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

        (
            resolve_size((output.size.width * self.density).ceil() as i32, width_spec),
            resolve_size(
                (output.size.height * self.density).ceil() as i32,
                height_spec,
            ),
        )
    }

    fn layout(&self, width: i32, height: i32) {
        // Collect placements first so no borrow is held while calling back
        // into the View system.
        let placements: Vec<_> = {
            let mut tree = self.tree.borrow_mut();
            let size = Size {
                width: self.to_dp(width),
                height: self.to_dp(height),
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
                    let left = self.to_px(layout.location.x);
                    let top = self.to_px(layout.location.y);
                    let right = self.to_px(layout.location.x + layout.size.width);
                    let bottom = self.to_px(layout.location.y + layout.size.height);
                    Some((view.clone(), left, top, right, bottom))
                })
                .collect()
        };

        for (view, left, top, right, bottom) in placements {
            // Re-measure at the final size: views such as TextView lay out
            // their content during measure.
            measure_view(
                &view,
                measure_spec::make(right - left, measure_spec::EXACTLY),
                measure_spec::make(bottom - top, measure_spec::EXACTLY),
            );
            layout_view(&view, left, top, right, bottom);
        }
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

        let layout = FlexLayout(new_layout_view(ctx));
        let density = display_density(ctx);
        let mut tree = ViewTree::new(ctx.scope(), props.read(), move |view, known, available| {
            measure_child(view, density, known, available)
        });
        tree.set_root(layout.0.clone(), modifier.clone(), ctx.component_id());

        let state = Rc::new(FlexState {
            tree: RefCell::new(tree),
            density,
        });

        attach(ctx, &layout, state.clone());

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
