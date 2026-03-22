use crate::component_scope::{ComponentId, ComponentScope};
use crate::vec_utils::extract_if;

use super::ReactiveScope;

impl ReactiveScope {
    pub fn create_child_component(&mut self, parent: Option<ComponentId>) -> ComponentId {
        let mut component = ComponentScope::default();
        component.parent = parent;
        let component_id = self.components.insert(component);
        match parent.and_then(|p| self.components.get_mut(p)) {
            Some(parent) => parent.children.push(component_id),
            None => self.root.push(component_id),
        }
        component_id
    }

    pub fn dispose_component(&mut self, id: ComponentId) {
        let Some(mut component) = self.components.remove(id) else {
            return;
        };

        for child_id in std::mem::take(&mut component.children) {
            self.dispose_component(child_id);
        }

        for cleanup_fn in component.cleanup {
            cleanup_fn();
        }
    }

    pub fn dispose_all_children(&mut self, id: ComponentId) {
        if let Some(component) = self.components.get_mut(id) {
            for child_id in std::mem::take(&mut component.children) {
                self.dispose_component(child_id);
            }
        }
    }

    pub fn dispose_children(&mut self, id: ComponentId, f: impl FnMut(&ComponentId) -> bool) {
        let to_dispose = self
            .components
            .get_mut(id)
            .map(move |scope| extract_if(&mut scope.children, f));

        if let Some(to_dispose) = to_dispose {
            for child in to_dispose {
                self.dispose_component(child);
            }
        }
    }

    pub fn on_cleanup(&mut self, component_id: ComponentId, cleanup_fn: impl FnOnce() + 'static) {
        if let Some(component) = self.components.get_mut(component_id) {
            component.cleanup.push(Box::new(cleanup_fn));
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::signal::Signal;
    use futures::task::noop_waker_ref;
    use std::sync::{Arc, Mutex};
    use std::task::Context;

    use super::ReactiveScope;

    #[test]
    fn test_component_dispose() {
        let mut scope = ReactiveScope::default();
        let root = scope.create_child_component(None);
        let child = scope.create_child_component(Some(root));

        let count = scope.create_signal(0);
        let result = Arc::new(Mutex::new(0));

        scope.create_effect(child, {
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
}
