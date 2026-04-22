use super::{Modifier, NativeViewRegistry};
use reactive_core::{
    BoxedComponent, Component, ContextKey, IntoSignal, SetupContext, StoredSignal,
};
use std::rc::Rc;

type ChildrenStore<N> = Rc<[Option<(N, Modifier)>]>;

struct IndexedChild<N: 'static> {
    index: usize,
    view_registry_key: &'static ContextKey<Rc<dyn NativeViewRegistry<N>>>,
    children_store: StoredSignal<ChildrenStore<N>>,
    child: BoxedComponent,
}
struct IndexedChildViewRegistry<N>(usize, StoredSignal<ChildrenStore<N>>);

impl<N: Clone + PartialEq + 'static> NativeViewRegistry<N> for IndexedChildViewRegistry<N> {
    fn update_view(&self, view: &N, modifier: Modifier) {
        self.1.update_with(|vec| {
            Rc::make_mut(vec)[self.0].replace((view.clone(), modifier));
            true
        })
    }

    fn clear_view(&self, view: &N) {
        self.1.update_with(|vec| match &vec[self.0] {
            Some((stored_view, _)) if stored_view == view => {
                Rc::make_mut(vec)[self.0].take();
                true
            }
            _ => false,
        })
    }
}

impl<N: Clone + PartialEq + 'static> Component for IndexedChild<N> {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        ctx.set_context(
            self.view_registry_key,
            (Rc::new(IndexedChildViewRegistry(self.index, self.children_store))
                as Rc<dyn NativeViewRegistry<N>>)
                .into_signal(),
        );

        ctx.boxed_child(self.child);
    }
}

pub fn setup_indexed_native_view_manager<N: Clone + PartialEq + 'static>(
    ctx: &mut SetupContext,
    view_registry_key: &'static ContextKey<Rc<dyn NativeViewRegistry<N>>>,
    children: Vec<BoxedComponent>,
) -> StoredSignal<ChildrenStore<N>> {
    let children_views = ctx.create_signal(vec![None; children.len()].into());
    children.into_iter().enumerate().for_each(|(index, child)| {
        ctx.child(IndexedChild {
            index,
            view_registry_key,
            children_store: children_views.clone(),
            child,
        });
    });

    children_views
}
