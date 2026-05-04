use crate::widgets::Modifier;
use reactive_core::{ComponentId, ReactiveScope};
use std::cell::RefCell;
use std::rc::Rc;
use taffy::{AvailableSpace, Layout, NodeId, PrintTree, Size, TaffyTree};

struct NativeNodeData<N> {
    view: N,
    modifier: Modifier,
    component_id: ComponentId,
}

#[derive(Clone)]
pub struct TaffyTreeManager<N> {
    scope: ReactiveScope,
    tree: Rc<RefCell<TaffyTree<NativeNodeData<N>>>>,
    root_node: NodeId,
}

impl<N> TaffyTreeManager<N> {
    pub fn new(scope: ReactiveScope) -> Self {
        let mut tree = TaffyTree::new();
        let root_node = tree.new_leaf(Default::default()).unwrap();
        Self {
            scope,
            tree: Rc::new(RefCell::new(tree)),
            root_node,
        }
    }
}

impl<N: PartialEq> TaffyTreeManager<N> {
    pub fn upsert_node(
        &self,
        component_id: ComponentId,
        view: N,
        modifier: Modifier,
        style: taffy::Style,
    ) {
        let mut tree = self.tree.borrow_mut();
        let children = tree.children(self.root_node).unwrap();
        match children.binary_search_by(|id| {
            let data = tree.get_node_context(*id).unwrap();
            self.scope
                .compare_components(data.component_id, component_id)
        }) {
            Ok(index) => {
                let data = tree.get_node_context_mut(children[index]).unwrap();
                data.view = view;
                data.modifier = modifier;
                tree.set_style(children[index], style).unwrap();
            }

            Err(index) => {
                let new_child = tree
                    .new_leaf_with_context(
                        style,
                        NativeNodeData {
                            view,
                            modifier,
                            component_id,
                        },
                    )
                    .unwrap();
                let _ = tree.insert_child_at_index(self.root_node, index, new_child);
            }
        }
    }

    pub fn remove_node(&self, component_id: ComponentId, view: N) {
        let mut tree = self.tree.borrow_mut();
        for child in tree.children(self.root_node).unwrap() {
            match tree.get_node_context_mut(child) {
                Some(data) if data.component_id == component_id && data.view == view => {
                    let _ = tree.remove_child(self.root_node, child);
                    return;
                }

                Some(data) if data.component_id == component_id => {
                    // The component is the same but the view is different, so it must have been
                    // updated after the view was added to the tree.
                    // Don't remove it since it's not the same view.
                    return;
                }

                _ => {}
            }
        }
    }

    pub fn compute_layout(
        &self,
        available_space: Size<AvailableSpace>,
        mut measurer: impl FnMut(Size<AvailableSpace>, &N) -> Size<f32>,
    ) {
        let mut tree = self.tree.borrow_mut();

        tree.compute_layout_with_measure(
            self.root_node,
            available_space,
            move |_known_dimensions, available_space, _node, ctx, _style| {
                measurer(available_space, &ctx.unwrap().view)
            },
        )
        .unwrap();
    }

    pub fn get_root_node_size(&self) -> Layout {
        self.tree.borrow().get_final_layout(self.root_node)
    }

    pub fn children_layouts(&self) -> impl Iterator<Item = (N, Layout)>
    where
        N: Clone,
    {
        let children = self.tree.borrow().children(self.root_node).unwrap();

        children.into_iter().map(move |child_id| {
            let tree = self.tree.borrow();
            let data = tree.get_node_context(child_id).unwrap();
            let layout = tree.get_final_layout(child_id);
            (data.view.clone(), layout)
        })
    }

    pub fn set_root_style(&self, style: taffy::Style) {
        let mut tree = self.tree.borrow_mut();
        tree.set_style(self.root_node, style).unwrap();
    }
}
