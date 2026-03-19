use crate::EffectContext;
use crate::component::{BoxedComponent, Component, SetupContext};
use crate::component_scope::ComponentId;
use std::hash::Hash;

type ItemsFn<T> = Box<dyn Fn(&mut EffectContext) -> Vec<T>>;
type KeyFn<T, K> = Box<dyn Fn(&T) -> K>;
type ChildFactory<T> = Box<dyn Fn(&T) -> BoxedComponent>;

pub struct For<T, K> {
    items_fn: ItemsFn<T>,
    key_fn: KeyFn<T, K>,
    child_fn: ChildFactory<T>,
}

impl<T: 'static, K: Eq + Hash + Clone + 'static> For<T, K> {
    pub fn new(
        items_fn: impl Fn(&mut EffectContext) -> Vec<T> + 'static,
        key_fn: impl Fn(&T) -> K + 'static,
        child_fn: impl Fn(&T) -> BoxedComponent + 'static,
    ) -> Self {
        Self {
            items_fn: Box::new(items_fn),
            key_fn: Box::new(key_fn),
            child_fn: Box::new(child_fn),
        }
    }
}

struct ChildEntry<K> {
    key: K,
    component_id: ComponentId,
}

impl<T: 'static, K: Eq + Hash + Clone + 'static> Component for For<T, K> {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let items_fn = self.items_fn;
        let key_fn = self.key_fn;
        let child_fn = self.child_fn;
        let component_id = ctx.component_id();

        ctx.create_effect(move |ectx, prev: Option<&mut Vec<ChildEntry<K>>>| {
            let items = (items_fn)(ectx);
            let mut prev_entries = prev.map(std::mem::take).unwrap_or_default();

            let new_keys: Vec<K> = items.iter().map(|item| (key_fn)(item)).collect();

            // Remove children whose keys are no longer present
            let mut kept = Vec::with_capacity(prev_entries.len());
            for entry in prev_entries.drain(..) {
                if new_keys.contains(&entry.key) {
                    kept.push(entry);
                } else {
                    ectx.dispose_component(entry.component_id);
                }
            }

            // Build the new list, reusing existing children by key
            let mut new_entries = Vec::with_capacity(items.len());
            for (item, key) in items.iter().zip(new_keys) {
                if let Some(pos) = kept.iter().position(|e| e.key == key) {
                    // Reuse existing child
                    new_entries.push(kept.swap_remove(pos));
                } else {
                    // Create new child
                    let mut child_ctx = ectx.setup_child(component_id);
                    let child_id = child_ctx.component_id();
                    (child_fn)(item).setup(&mut child_ctx);
                    new_entries.push(ChildEntry {
                        key,
                        component_id: child_id,
                    });
                }
            }

            // Dispose any remaining kept entries that weren't matched
            // (shouldn't happen since we filtered by new_keys, but defensive)
            for entry in kept {
                ectx.dispose_component(entry.component_id);
            }

            new_entries
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ReactiveScope;
    use futures::task::noop_waker_ref;
    use std::sync::{Arc, Mutex};
    use std::task::Context;

    #[test]
    fn test_for_initial_render() {
        let mut scope = ReactiveScope::default();
        let root = scope.create_child_component(None);
        let items = scope.create_signal(vec!["a", "b", "c"]);
        let log = Arc::new(Mutex::new(Vec::<String>::new()));

        let for_comp = Box::new(For::new(
            move |ctx| ctx.access(items, |v| v.clone()),
            |item: &&str| item.to_string(),
            {
                let log = Arc::clone(&log);
                move |item: &&str| -> BoxedComponent {
                    let item = item.to_string();
                    let log = Arc::clone(&log);
                    Box::new(move |_: &mut SetupContext| log.lock().unwrap().push(item))
                }
            },
        ));

        for_comp.setup(&mut SetupContext {
            scope: &mut scope,
            component_id: root,
        });
        assert_eq!(*log.lock().unwrap(), vec!["a", "b", "c"]);
    }

    #[test]
    fn test_for_add_item() {
        let mut scope = ReactiveScope::default();
        let root = scope.create_child_component(None);
        let items = scope.create_signal(vec!["a", "b"]);
        let log = Arc::new(Mutex::new(Vec::<String>::new()));

        let for_comp = Box::new(For::new(
            move |ctx| ctx.access(items, |v| v.clone()),
            |item: &&str| item.to_string(),
            {
                let log = Arc::clone(&log);
                move |item: &&str| -> BoxedComponent {
                    let item = item.to_string();
                    let log = Arc::clone(&log);
                    Box::new(move |_: &mut SetupContext| log.lock().unwrap().push(item))
                }
            },
        ));

        for_comp.setup(&mut SetupContext {
            scope: &mut scope,
            component_id: root,
        });
        assert_eq!(*log.lock().unwrap(), vec!["a", "b"]);

        scope.update(items, |v| {
            v.push("c");
            true
        });
        scope.tick(&mut Context::from_waker(noop_waker_ref()));
        assert_eq!(*log.lock().unwrap(), vec!["a", "b", "c"]);
    }

    #[test]
    fn test_for_remove_item() {
        let mut scope = ReactiveScope::default();
        let root = scope.create_child_component(None);
        let items = scope.create_signal(vec!["a", "b", "c"]);
        let log = Arc::new(Mutex::new(Vec::<String>::new()));
        let disposed = Arc::new(Mutex::new(Vec::<String>::new()));

        let for_comp = Box::new(For::new(
            move |ctx| ctx.access(items, |v| v.clone()),
            |item: &&str| item.to_string(),
            {
                let log = Arc::clone(&log);
                let disposed = Arc::clone(&disposed);
                move |item: &&str| -> BoxedComponent {
                    let item = item.to_string();
                    let log = Arc::clone(&log);
                    let disposed = Arc::clone(&disposed);
                    Box::new(move |ctx: &mut SetupContext| {
                        log.lock().unwrap().push(item.clone());
                        let item = item.clone();
                        let disposed = disposed.clone();
                        ctx.on_cleanup(move || disposed.lock().unwrap().push(item));
                    })
                }
            },
        ));

        for_comp.setup(&mut SetupContext {
            scope: &mut scope,
            component_id: root,
        });
        assert_eq!(*log.lock().unwrap(), vec!["a", "b", "c"]);

        scope.update(items, |v| {
            v.retain(|&x| x != "b");
            true
        });
        scope.tick(&mut Context::from_waker(noop_waker_ref()));
        assert_eq!(*disposed.lock().unwrap(), vec!["b"]);
        assert_eq!(*log.lock().unwrap(), vec!["a", "b", "c"]);
    }

    #[test]
    fn test_for_reorder() {
        let mut scope = ReactiveScope::default();
        let root = scope.create_child_component(None);
        let items = scope.create_signal(vec!["a", "b", "c"]);
        let log = Arc::new(Mutex::new(Vec::<String>::new()));

        let for_comp = Box::new(For::new(
            move |ctx| ctx.access(items, |v| v.clone()),
            |item: &&str| item.to_string(),
            {
                let log = Arc::clone(&log);
                move |item: &&str| -> BoxedComponent {
                    let item = item.to_string();
                    let log = Arc::clone(&log);
                    Box::new(move |_: &mut SetupContext| log.lock().unwrap().push(item))
                }
            },
        ));

        for_comp.setup(&mut SetupContext {
            scope: &mut scope,
            component_id: root,
        });
        assert_eq!(*log.lock().unwrap(), vec!["a", "b", "c"]);

        scope.update(items, |v| {
            *v = vec!["c", "a", "b"];
            true
        });
        scope.tick(&mut Context::from_waker(noop_waker_ref()));
        assert_eq!(*log.lock().unwrap(), vec!["a", "b", "c"]);
    }

    #[test]
    fn test_for_empty() {
        let mut scope = ReactiveScope::default();
        let root = scope.create_child_component(None);
        let items = scope.create_signal(Vec::<&str>::new());
        let log = Arc::new(Mutex::new(Vec::<String>::new()));

        let for_comp = Box::new(For::new(
            move |ctx| ctx.access(items, |v| v.clone()),
            |item: &&str| item.to_string(),
            {
                let log = Arc::clone(&log);
                move |item: &&str| -> BoxedComponent {
                    let item = item.to_string();
                    let log = Arc::clone(&log);
                    Box::new(move |_: &mut SetupContext| log.lock().unwrap().push(item))
                }
            },
        ));

        for_comp.setup(&mut SetupContext {
            scope: &mut scope,
            component_id: root,
        });
        assert!(log.lock().unwrap().is_empty());
    }
}
