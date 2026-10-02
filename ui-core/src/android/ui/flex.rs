use super::VIEW_REGISTRY_KEY;
use crate::android::Node;
use crate::widgets::taffy::{Axis, FlexTaffyContainer, measure_leaf};
use crate::widgets::{CommonFlex, Modifier, NativeView, NativeViewRegistry};
use jni::JNIEnv;
use jni::objects::{JObject, JValue};
use jni::sys::{jboolean, jfloat, jfloatArray, jlong};
use reactive_core::{Component, ComponentId, FunctionTracker, SetupContext, Signal};
use std::cell::RefCell;
use std::rc::Rc;
use taffy::{AvailableSpace, RequestedAxis, RunMode, Size};

pub type Flex = CommonFlex<Node>;

type ViewTree = FlexTaffyContainer<Node>;

/// The Rust half of a Kotlin `FlexNode`, which holds its address in `handle` until it drops.
struct FlexHandle {
    node: Node,
    tree: RefCell<ViewTree>,
    tracker: FunctionTracker,
}

impl Drop for FlexHandle {
    fn drop(&mut self) {
        self.node.set("setHandle", 0i64);
    }
}

impl Node {
    fn invalidate_layout(&self) {
        self.call("invalidate", "()V", &[]);
    }
}

struct FlexRegistry(Rc<FlexHandle>);

impl NativeViewRegistry<Node> for FlexRegistry {
    fn update_view(&self, component_id: ComponentId, view: Node, modifier: Modifier) {
        let FlexHandle { node, tree, .. } = &*self.0;
        let (index, previous) =
            tree.borrow_mut()
                .insert_child(view.clone(), modifier, component_id);
        match previous {
            None => node.insert_child(index, &view),
            Some(previous) if previous != view => {
                node.remove_child_at(index);
                node.insert_child(index, &view);
            }
            Some(_) => {}
        }
        node.invalidate_layout();
    }

    fn clear_view(&self, _component_id: ComponentId, view: Node) {
        let FlexHandle { node, tree, .. } = &*self.0;
        let removed = tree.borrow_mut().remove_child(&view);
        if let Some(index) = removed {
            node.remove_child_at(index);
            node.invalidate_layout();
        }
    }
}

/// Sizes a child through `FlexNode.intrinsic`, i.e. Compose intrinsic measurements, which
/// unlike `measure()` may be queried any number of times per layout pass.
fn measure_child(
    flex: &Node,
    child: &Node,
    known_dimensions: Size<Option<f32>>,
    available_space: Size<AvailableSpace>,
) -> Size<f32> {
    let intrinsic = |kind: i32, cross: f32| {
        flex.call(
            "intrinsic",
            "(Lcom/reactive/Node;IF)F",
            &[
                JValue::Object(child.as_obj()),
                JValue::Int(kind),
                JValue::Float(cross),
            ],
        )
        .f()
        .expect("FlexNode.intrinsic")
    };

    measure_leaf(known_dimensions, available_space, |axis, cross| {
        let cross = cross.unwrap_or(-1.0);
        let (min, max) = match axis {
            Axis::Horizontal => (0, 1),
            Axis::Vertical => (2, 3),
        };
        (intrinsic(min, cross), intrinsic(max, cross))
    })
}

impl Component for Flex {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let Self {
            props,
            children,
            modifier,
            ..
        } = *self;

        let node = Node::new("com/reactive/FlexNode");
        let mut tree = ViewTree::new(ctx.scope(), props.read(), {
            let node = node.clone();
            move |child, known, available| measure_child(&node, child, known, available)
        });
        tree.set_root(node.clone(), modifier.clone(), ctx.component_id());

        let tracker = ctx.create_fn_tracking({
            let node = node.clone();
            move || node.invalidate_layout()
        });

        let handle = Rc::new(FlexHandle {
            node: node.clone(),
            tree: RefCell::new(tree),
            tracker,
        });
        node.set("setHandle", Rc::as_ptr(&handle) as i64);

        NativeView::new(
            move |_| node,
            |node| node,
            |_, _| {},
            modifier,
            &VIEW_REGISTRY_KEY,
        )
        .setup_in_component(ctx);

        ctx.create_effect({
            let handle = handle.clone();
            move |_, _| {
                handle.tree.borrow_mut().set_props(props.read());
                handle.node.invalidate_layout();
            }
        });

        let registry: Rc<dyn NativeViewRegistry<Node>> = Rc::new(FlexRegistry(handle));
        ctx.set_static_context(&VIEW_REGISTRY_KEY, registry);

        for child in children {
            ctx.boxed_child(child);
        }
    }
}

const MIN_CONTENT: f32 = -1.0;
const MAX_CONTENT: f32 = -2.0;

/// Runs layout for `FlexNode.runLayout`. Sizes are dp. Returns `[width, height]`, then
/// `x, y, width, height` per child (in `FlexNode.children` order) when `layout` is set.
#[unsafe(no_mangle)]
extern "system" fn Java_com_reactive_FlexNode_nativeMeasure<'local>(
    env: JNIEnv<'local>,
    _this: JObject<'local>,
    handle: jlong,
    known_width: jfloat,
    known_height: jfloat,
    available_width: jfloat,
    available_height: jfloat,
    layout: jboolean,
) -> jfloatArray {
    // SAFETY: Kotlin holds a non-zero handle only while the `FlexHandle` is alive, and both
    // run on the main thread.
    let handle = unsafe { &*(handle as *const FlexHandle) };
    let layout = layout != 0;

    let known = |v: f32| (!v.is_nan()).then_some(v);
    let available = |v: f32| match v {
        MIN_CONTENT => AvailableSpace::MinContent,
        MAX_CONTENT => AvailableSpace::MaxContent,
        v => AvailableSpace::Definite(v),
    };

    let mut out = Vec::new();
    if let Ok(mut tree) = handle.tree.try_borrow_mut() {
        let output = handle.tracker.run_tracking(|| {
            tree.compute_layout(
                if layout {
                    RunMode::PerformLayout
                } else {
                    RunMode::ComputeSize
                },
                Size {
                    width: known(known_width),
                    height: known(known_height),
                },
                Size {
                    width: available(available_width),
                    height: available(available_height),
                },
                RequestedAxis::Both,
            )
        });

        out.extend([output.size.width, output.size.height]);
        if layout {
            for (_, child) in tree.iter() {
                match child {
                    Some(l) => {
                        out.extend([l.location.x, l.location.y, l.size.width, l.size.height])
                    }
                    None => out.extend([0.0; 4]),
                }
            }
        }
    } else {
        tracing::warn!("FlexNode measured re-entrantly; reporting zero size");
        out.extend([0.0, 0.0]);
    }

    let array = env
        .new_float_array(out.len() as i32)
        .expect("allocate layout array");
    env.set_float_array_region(&array, 0, &out)
        .expect("fill layout array");
    array.into_raw()
}
