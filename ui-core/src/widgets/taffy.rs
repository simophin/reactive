use crate::widgets::taffy_modifier::ModifierAndFlexProps;
use crate::widgets::{FlexProps, Modifier};
use reactive_core::{ComponentId, ReactiveScope};
use taffy::{
    AvailableSpace, Cache, CacheTree, CoreStyle, Layout, LayoutFlexboxContainer, LayoutInput,
    LayoutOutput, LayoutPartialTree, Line, NodeId, RequestedAxis, RunMode, Size, SizingMode,
    TraversePartialTree, compute_cached_layout, compute_flexbox_layout, compute_leaf_layout,
};

struct NativeViewData<N> {
    view: N,
    modifier: Modifier,
    component_id: ComponentId,
    node_id: NodeId,
    layout: Option<Layout>,
    cache: Cache,
}

pub struct NativeViewIter<'a, N>(std::slice::Iter<'a, NativeViewData<N>>);

impl<'a, N> Iterator for NativeViewIter<'a, N> {
    type Item = (&'a N, Option<&'a Layout>);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|data| (&data.view, data.layout.as_ref()))
    }
}

pub struct FlexTaffyContainer<N> {
    scope: ReactiveScope,
    node_id_seq: u64,
    children: Vec<NativeViewData<N>>,
    root: Option<NativeViewData<N>>,
    props: FlexProps,
    child_measurer: Box<dyn Fn(&N, Size<Option<f32>>, Size<AvailableSpace>) -> Size<f32>>,
}

pub const ROOT_ID: NodeId = NodeId::new(0);

impl<N: 'static> FlexTaffyContainer<N> {
    fn get_node_by_id(&self, node_id: NodeId) -> Option<&NativeViewData<N>> {
        match self.root.as_ref() {
            Some(root) if root.node_id == node_id => Some(root),
            _ => self.children.iter().find(|child| child.node_id == node_id),
        }
    }

    fn get_mut_node_by_id(&mut self, node_id: NodeId) -> Option<&mut NativeViewData<N>> {
        match self.root.as_mut() {
            Some(root) if root.node_id == node_id => Some(root),
            _ => self
                .children
                .iter_mut()
                .find(|child| child.node_id == node_id),
        }
    }

    fn new_node_id(&mut self) -> NodeId {
        let id = self.node_id_seq;
        self.node_id_seq += 1;
        NodeId::new(id)
    }

    pub fn insert_child(&mut self, view: N, modifier: Modifier, component_id: ComponentId) {
        match self.children.binary_search_by(|child| {
            self.scope
                .compare_components(child.component_id, component_id)
        }) {
            Ok(index) => {
                let child = &mut self.children[index];
                child.view = view;
                child.modifier = modifier;
                child.cache.clear();
            }

            Err(index) => {
                let node_id = self.new_node_id();
                self.children.insert(
                    index,
                    NativeViewData {
                        view,
                        modifier,
                        component_id,
                        cache: Default::default(),
                        layout: None,
                        node_id,
                    },
                )
            }
        }
    }

    pub fn remove_child(&mut self, view: &N) -> bool
    where
        N: PartialEq,
    {
        self.children
            .iter()
            .position(|child| &child.view == view)
            .map(|index| self.children.remove(index))
            .is_some()
    }

    pub fn set_root(&mut self, view: N, modifier: Modifier, component_id: ComponentId) {
        self.root.replace(NativeViewData {
            view,
            modifier,
            node_id: ROOT_ID,
            layout: None,
            cache: Default::default(),
            component_id,
        });
    }

    pub fn set_props(&mut self, props: FlexProps) {
        if self.props == props {
            return;
        }

        self.props = props;
        self.clear_cache();
    }

    pub fn clear_cache(&mut self) {
        if let Some(root) = self.root.as_mut() {
            root.cache.clear();
        }
        for child in &mut self.children {
            child.cache.clear();
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (&N, Option<&Layout>)> {
        NativeViewIter(self.children.iter())
    }

    pub fn root_layout(&self) -> Option<&Layout> {
        self.root.as_ref().and_then(|root| root.layout.as_ref())
    }

    pub fn root_view(&self) -> Option<&N> {
        self.root.as_ref().map(|root| &root.view)
    }

    pub fn root_modifier(&self) -> Option<&Modifier> {
        self.root.as_ref().map(|root| &root.modifier)
    }

    pub fn compute_layout(
        &mut self,
        run_mode: RunMode,
        known_dimensions: Size<Option<f32>>,
        available_space: Size<AvailableSpace>,
        axis: RequestedAxis,
    ) -> LayoutOutput {
        self.clear_cache();
        compute_flexbox_layout(
            self,
            ROOT_ID,
            LayoutInput {
                run_mode,
                sizing_mode: SizingMode::InherentSize,
                axis,
                known_dimensions,
                parent_size: Default::default(),
                available_space,
                vertical_margins_are_collapsible: Line::FALSE,
            },
        )
    }

    pub fn new(
        reactive_scope: ReactiveScope,
        props: FlexProps,
        child_measurer: impl Fn(&N, Size<Option<f32>>, Size<AvailableSpace>) -> Size<f32> + 'static,
    ) -> Self {
        Self {
            scope: reactive_scope,
            children: Default::default(),
            root: None,
            node_id_seq: 1,
            props,
            child_measurer: Box::new(child_measurer),
        }
    }
}

pub struct TreeChildIter<'a, N>(std::slice::Iter<'a, NativeViewData<N>>);

impl<'a, N> Iterator for TreeChildIter<'a, N> {
    type Item = NodeId;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|d| d.node_id)
    }
}

impl<N: 'static> TraversePartialTree for FlexTaffyContainer<N> {
    type ChildIter<'a> = TreeChildIter<'a, N>;

    fn child_ids(&self, parent_node_id: NodeId) -> Self::ChildIter<'_> {
        if ROOT_ID == parent_node_id {
            TreeChildIter(self.children.iter())
        } else {
            TreeChildIter([].iter())
        }
    }

    fn child_count(&self, parent_node_id: NodeId) -> usize {
        if ROOT_ID == parent_node_id {
            self.children.len()
        } else {
            0
        }
    }

    fn get_child_id(&self, parent_node_id: NodeId, child_index: usize) -> NodeId {
        if ROOT_ID == parent_node_id {
            self.children[child_index].node_id
        } else {
            panic!("Invalid parent node id")
        }
    }
}

impl<N: 'static> CacheTree for FlexTaffyContainer<N> {
    fn cache_get(&self, node_id: NodeId, input: &LayoutInput) -> Option<LayoutOutput> {
        self.get_node_by_id(node_id)?.cache.get(input)
    }

    fn cache_store(&mut self, node_id: NodeId, input: &LayoutInput, layout_output: LayoutOutput) {
        self.get_mut_node_by_id(node_id)
            .expect("Invalid node id")
            .cache
            .store(input, layout_output);
    }

    fn cache_clear(&mut self, node_id: NodeId) {
        self.get_mut_node_by_id(node_id)
            .expect("Invalid node id")
            .cache
            .clear();
    }
}

impl<N: 'static> LayoutPartialTree for FlexTaffyContainer<N> {
    type CoreContainerStyle<'a> = &'a Modifier;
    type CustomIdent = <Modifier as CoreStyle>::CustomIdent;

    fn get_core_container_style(&self, node_id: NodeId) -> Self::CoreContainerStyle<'_> {
        if ROOT_ID == node_id {
            &self.root.as_ref().unwrap().modifier
        } else {
            panic!("Invalid node id")
        }
    }

    fn set_unrounded_layout(&mut self, node_id: NodeId, layout: &Layout) {
        self.get_mut_node_by_id(node_id)
            .expect("Invalid node id")
            .layout
            .replace(layout.clone());
    }

    fn compute_child_layout(&mut self, node_id: NodeId, inputs: LayoutInput) -> LayoutOutput {
        compute_cached_layout(self, node_id, inputs, |tree, node_id, inputs| {
            let node = tree
                .children
                .iter()
                .find(|c| c.node_id == node_id)
                .expect("Invalid node id");

            compute_leaf_layout(
                inputs,
                &node.modifier,
                |_, value| value,
                |known_dimensions, available_space| {
                    (tree.child_measurer)(&node.view, known_dimensions, available_space)
                },
            )
        })
    }
}

impl<N: 'static> LayoutFlexboxContainer for FlexTaffyContainer<N> {
    type FlexboxContainerStyle<'a> = ModifierAndFlexProps<'a>;

    type FlexboxItemStyle<'a> = &'a Modifier;

    fn get_flexbox_container_style(&self, node_id: NodeId) -> Self::FlexboxContainerStyle<'_> {
        if ROOT_ID == node_id {
            ModifierAndFlexProps(&self.root.as_ref().unwrap().modifier, &self.props)
        } else {
            panic!("Invalid node id")
        }
    }

    fn get_flexbox_child_style(&self, child_node_id: NodeId) -> Self::FlexboxItemStyle<'_> {
        assert_ne!(child_node_id, ROOT_ID, "Root node cannot be a child");

        &self
            .get_node_by_id(child_node_id)
            .expect("Invalid child node id")
            .modifier
    }
}
