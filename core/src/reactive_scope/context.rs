use super::ReactiveScope;
use crate::component_scope::{ComponentId, ContextKey};
use crate::signal::StoredSignal;
use crate::signal::stored::ReadStoredSignal;

impl ReactiveScope {
    pub(crate) fn provide_context<T: Clone + 'static>(
        &self,
        component_id: ComponentId,
        key: &'static ContextKey<T>,
        initial_value: T,
    ) -> StoredSignal<T> {
        let signal = self.create_signal(initial_value);
        if let Some(component) = self.0.borrow_mut().components.get_mut(component_id) {
            component.context.insert(key.id(), signal.clone().into());
        }
        signal
    }

    pub(crate) fn use_context<T: Clone + 'static>(
        &self,
        component_id: ComponentId,
        key: &'static ContextKey<T>,
    ) -> Option<ReadStoredSignal<T>> {
        let data = self.0.borrow();
        let mut scope = data.components.get(component_id);

        while let Some(component) = scope {
            if let Some(signal) = component.context.get(&key.id()) {
                return signal.downcast_ref().cloned().map(|r| r.read_only());
            }
            scope = component.parent.and_then(|id| data.components.get(id));
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use crate::component_scope::ContextKey;
    use crate::signal::Signal;

    use super::ReactiveScope;

    #[test]
    fn test_context() {
        static THEME: ContextKey<&str> = ContextKey::new();

        let scope = ReactiveScope::default();
        let root = scope.create_child_component(None);

        scope.provide_context(root, &THEME, "dark");

        let child = scope.create_child_component(Some(root));
        let theme_signal = scope.use_context::<&str>(child, &THEME).unwrap();

        assert_eq!(theme_signal.read(), "dark");
    }

    #[test]
    fn test_context_override() {
        static THEME: ContextKey<&str> = ContextKey::new();

        let scope = ReactiveScope::default();
        let root = scope.create_child_component(None);
        scope.provide_context(root, &THEME, "dark");

        let child = scope.create_child_component(Some(root));
        scope.provide_context(child, &THEME, "light");

        let grandchild = scope.create_child_component(Some(child));
        let gc_theme = scope.use_context::<&str>(grandchild, &THEME).unwrap();
        assert_eq!(gc_theme.read(), "light");

        let sibling = scope.create_child_component(Some(root));
        let sibling_theme = scope.use_context::<&str>(sibling, &THEME).unwrap();
        assert_eq!(sibling_theme.read(), "dark");

        let root_theme = scope.use_context::<&str>(root, &THEME).unwrap();
        assert_eq!(root_theme.read(), "dark");
    }
}
