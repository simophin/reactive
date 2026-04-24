use super::{ReactiveScope, ReactiveScopeData};
use crate::component_scope::{ComponentId, ComponentScope};
use smallvec::SmallVec;
use std::cmp::Ordering;
use std::iter::once;

impl ReactiveScope {
    pub(crate) fn create_child_component(&self, parent: Option<ComponentId>) -> ComponentId {
        let mut data = self.0.borrow_mut();
        let mut component = ComponentScope::default();
        component.parent = parent;
        let component_id = data.components.insert(component);
        match parent.and_then(|p| data.components.get_mut(p)) {
            Some(parent_scope) => parent_scope.children.push(component_id),
            None => data.root.push(component_id),
        }
        component_id
    }

    pub fn dispose_component(&self, id: ComponentId) {
        // Remove the component in a short borrow, then drop the borrow before
        // recursing or calling cleanup fns (which are user code and may call
        // back into the scope).
        let component = { self.0.borrow_mut().components.remove(id) };
        let Some(component) = component else {
            return;
        };

        for child_id in component.children {
            self.dispose_component(child_id);
        }

        for cleanup_fn in component.cleanup {
            cleanup_fn(); // no borrow held — user code may call scope methods
        }
    }

    pub(crate) fn dispose_all_children(&self, id: ComponentId) {
        let children = {
            let mut data = self.0.borrow_mut();
            data.components
                .get_mut(id)
                .map(|c| std::mem::take(&mut c.children))
                .unwrap_or_default()
        }; // borrow released

        for child_id in children {
            self.dispose_component(child_id);
        }
    }

    pub(crate) fn on_cleanup(
        &self,
        component_id: ComponentId,
        cleanup_fn: impl FnOnce() + 'static,
    ) {
        if let Some(component) = self.0.borrow_mut().components.get_mut(component_id) {
            component.cleanup.push(Box::new(cleanup_fn));
        }
    }

    pub fn compare_components(&self, lhs: ComponentId, rhs: ComponentId) -> Ordering {
        if lhs == rhs {
            return Ordering::Equal;
        }

        let data = self.0.borrow();

        // Build paths into common ancestor for both components
        let lhs_path: SmallVec<[_; 8]> = once(lhs).chain(data.ancestors_of(lhs)).collect();
        let rhs_path: SmallVec<[_; 8]> = once(rhs).chain(data.ancestors_of(rhs)).collect();

        // Finding the split point
        let mut split_ancestor = None;
        for (a, b) in lhs_path.iter().rev().zip(rhs_path.iter().rev()) {
            if a == b {
                split_ancestor.replace(*a);
                continue;
            }

            // The first time we see different values, the tree splits...now we'll know who's left
            // and who's right.
            let children = split_ancestor
                .map(|c| &data.components.get(c).unwrap().children)
                .unwrap_or(&data.root);

            return children
                .iter()
                .position(|c| c == a)
                .unwrap()
                .cmp(&children.iter().position(|c| c == b).unwrap());
        }

        // If one path is a full prefix of the other, the shorter path is the
        // ancestor and should sort before its descendant.
        lhs_path.len().cmp(&rhs_path.len())
    }
}

struct AncestorIter<'a> {
    current: Option<ComponentId>,
    data: &'a ReactiveScopeData,
}

impl<'a> Iterator for AncestorIter<'a> {
    type Item = ComponentId;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.current?;
        let next = self.data.components.get(current).and_then(|p| p.parent);
        self.current = next;
        next
    }
}

impl ReactiveScopeData {
    fn ancestors_of(&self, component_id: ComponentId) -> AncestorIter<'_> {
        AncestorIter {
            current: Some(component_id),
            data: self,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::signal::Signal;
    use futures::task::noop_waker_ref;
    use std::cmp::Ordering;
    use std::sync::{Arc, Mutex};
    use std::task::Context;

    use super::ReactiveScope;

    #[test]
    fn test_component_dispose() {
        let scope = ReactiveScope::default();
        let root = scope.create_child_component(None);
        let child = scope.create_child_component(Some(root));

        let count = scope.create_signal(0);
        let result = Arc::new(Mutex::new(0));

        scope.create_effect(child, {
            let count = count.clone();
            let result = Arc::clone(&result);
            move |_, _: Option<()>| {
                *result.lock().unwrap() = count.read();
            }
        });

        assert_eq!(*result.lock().unwrap(), 0);

        count.update_if_changes(5);
        scope.tick(&mut Context::from_waker(noop_waker_ref()));
        assert_eq!(*result.lock().unwrap(), 5);

        scope.dispose_component(child);

        count.update_if_changes(10);
        scope.tick(&mut Context::from_waker(noop_waker_ref()));
        assert_eq!(*result.lock().unwrap(), 5); // unchanged
    }

    #[test]
    fn test_compare_components_orders_siblings() {
        let scope = ReactiveScope::default();
        let parent = scope.create_child_component(None);
        let first = scope.create_child_component(Some(parent));
        let second = scope.create_child_component(Some(parent));

        assert_eq!(scope.compare_components(first, second), Ordering::Less);
        assert_eq!(scope.compare_components(second, first), Ordering::Greater);
        assert_eq!(scope.compare_components(first, first), Ordering::Equal);
    }

    #[test]
    fn test_compare_components_orders_across_branches() {
        let scope = ReactiveScope::default();
        let root = scope.create_child_component(None);
        let left = scope.create_child_component(Some(root));
        let right = scope.create_child_component(Some(root));
        let left_leaf = scope.create_child_component(Some(left));
        let right_leaf = scope.create_child_component(Some(right));

        assert_eq!(
            scope.compare_components(left_leaf, right_leaf),
            Ordering::Less
        );
        assert_eq!(
            scope.compare_components(right_leaf, left_leaf),
            Ordering::Greater
        );
    }

    #[test]
    fn test_compare_components_orders_ancestor_before_descendant() {
        let scope = ReactiveScope::default();
        let root = scope.create_child_component(None);
        let child = scope.create_child_component(Some(root));
        let grandchild = scope.create_child_component(Some(child));

        assert_eq!(scope.compare_components(root, grandchild), Ordering::Less);
        assert_eq!(
            scope.compare_components(grandchild, root),
            Ordering::Greater
        );
        assert_eq!(scope.compare_components(child, grandchild), Ordering::Less);
    }

    #[test]
    fn test_compare_components_orders_across_root_components() {
        let scope = ReactiveScope::default();
        let first_root = scope.create_child_component(None);
        let second_root = scope.create_child_component(None);
        let first_leaf = scope.create_child_component(Some(first_root));
        let second_leaf = scope.create_child_component(Some(second_root));

        assert_eq!(
            scope.compare_components(first_leaf, second_leaf),
            Ordering::Less
        );
        assert_eq!(
            scope.compare_components(second_leaf, first_leaf),
            Ordering::Greater
        );
    }
}
