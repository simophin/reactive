use crate::component_scope::{ComponentId, ComponentScope};

use super::ReactiveScope;

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
        let scope = ReactiveScope::default();
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
