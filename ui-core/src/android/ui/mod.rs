use super::node::Node;
use crate::widgets::{Modifier, NativeViewRegistry};
use reactive_core::{ComponentId, ContextKey, ReactiveScope};
use std::rc::Rc;

pub mod button;
pub mod flex;
pub mod image;
pub mod label;
pub mod platform;
pub mod progress_indicator;
pub mod slider;
pub mod stack;
pub mod text_input;
pub mod window;

pub(crate) static VIEW_REGISTRY_KEY: ContextKey<Rc<dyn NativeViewRegistry<Node>>> =
    ContextKey::new();

/// Puts the top-level view into `RootNode.child`.
pub(crate) struct RootRegistry {
    pub root: Node,
    pub current: std::cell::RefCell<Option<Node>>,
}

impl NativeViewRegistry<Node> for RootRegistry {
    fn update_view(&self, _component_id: ComponentId, view: Node, _modifier: Modifier) {
        self.root.set("setChild", Some(&view));
        self.current.replace(Some(view));
    }

    fn clear_view(&self, _component_id: ComponentId, view: Node) {
        if self.current.borrow().as_ref() == Some(&view) {
            self.root.set("setChild", None::<&Node>);
            self.current.replace(None);
        }
    }
}

/// Mirrors views registered by child components into a Kotlin `children` list
/// (`insertChild(index, node)` / `removeChildAt(index)`), kept in component order.
pub(crate) struct ChildListRegistry {
    pub scope: ReactiveScope,
    pub parent: Node,
    pub children: std::cell::RefCell<Vec<(ComponentId, Node)>>,
}

impl NativeViewRegistry<Node> for ChildListRegistry {
    fn update_view(&self, component_id: ComponentId, view: Node, _modifier: Modifier) {
        let mut children = self.children.borrow_mut();
        match children.binary_search_by(|(id, _)| self.scope.compare_components(*id, component_id))
        {
            Ok(index) if children[index].1 == view => {}
            Ok(index) => {
                children[index].1 = view.clone();
                self.parent.remove_child_at(index);
                self.parent.insert_child(index, &view);
            }
            Err(index) => {
                children.insert(index, (component_id, view.clone()));
                self.parent.insert_child(index, &view);
            }
        }
    }

    fn clear_view(&self, component_id: ComponentId, _view: Node) {
        let mut children = self.children.borrow_mut();
        if let Some(index) = children.iter().position(|(id, _)| *id == component_id) {
            children.remove(index);
            self.parent.remove_child_at(index);
        }
    }
}

impl Node {
    pub(crate) fn insert_child(&self, index: usize, child: &Node) {
        self.call(
            "insertChild",
            "(ILcom/reactive/Node;)V",
            &[
                jni::objects::JValue::Int(index as i32),
                jni::objects::JValue::Object(child.as_obj()),
            ],
        );
    }

    pub(crate) fn remove_child_at(&self, index: usize) {
        self.call(
            "removeChildAt",
            "(I)V",
            &[jni::objects::JValue::Int(index as i32)],
        );
    }
}

/// A leaf widget backed by the Kotlin node `class`, created by `create`.
pub(crate) fn leaf(
    create: impl FnOnce(&mut reactive_core::SetupContext) -> Node + 'static,
) -> crate::widgets::NativeView<Node, Node> {
    crate::widgets::NativeView::new(
        create,
        |node| node,
        |_, _| {},
        Default::default(),
        &VIEW_REGISTRY_KEY,
    )
}
