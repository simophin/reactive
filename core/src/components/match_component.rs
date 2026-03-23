use crate::ReadSignal;
use crate::component::{BoxedComponent, Component, SetupContext};

/// Builds an extractor closure for use with [`Match::case`].
///
/// Matches a single pattern against `&mut T` and returns `Some(expr)` on match,
/// `None` otherwise. Bindings in the pattern are mutable references, so
/// [`std::mem::take`] works for [`Default`] types and `.clone()` for the rest.
///
/// # Examples
///
/// ```ignore
/// Match::new(signal, fallback)
///     .case(extract!(MyEnum::Variant(x) => x.clone()), |sig| { ... })
///     .case(extract!(MyEnum::Unit => ()), |sig| { ... })
/// ```
#[macro_export]
macro_rules! extract {
    ($pattern:pat => $expr:expr) => {
        |v| match v {
            $pattern => ::std::option::Option::Some($expr),
            _ => ::std::option::Option::None,
        }
    };
}
use crate::ReactiveScope;
use crate::signal::{BoxedSignal, Signal, StoredSignal, remove_signal};
use std::cell::RefCell;
use std::rc::Rc;

/// A case inside a [`Match`] component.
struct Case<T> {
    /// Run the extractor against the current signal value.
    ///
    /// Returns:
    /// - `None` — the extractor did not match.
    /// - `Some(None)` — matched; the extracted value was pushed into the existing signal
    ///   (the child component does not need to be rebuilt).
    /// - `Some(Some(component))` — newly activated; build the subtree with this component.
    try_match: Box<dyn FnMut(&mut T, &ReactiveScope) -> Option<Option<BoxedComponent>>>,

    /// Called when this case transitions from active to inactive. Removes the case's signal
    /// from the global store so it can be garbage-collected once the child is disposed.
    deactivate: Box<dyn FnMut()>,
}

/// A reactive `match`-like component.
///
/// Each case supplies an *extractor* `FnMut(&mut T) -> Option<E>` and a *factory*
/// `FnMut(StoredSignal<E>) -> BoxedComponent`.
///
/// When a case first becomes active the factory is called once with a freshly created signal
/// holding the extracted value `E`. While the same case remains active, subsequent changes to
/// `E` are pushed into that signal — the child component reacts through its own effects without
/// being rebuilt. The child is only rebuilt when the active case index changes.
pub struct Match<T> {
    signal: BoxedSignal<T>,
    cases: Vec<Case<T>>,
    fallback: Box<dyn FnMut() -> BoxedComponent>,
}

impl<T: Clone + 'static> Match<T> {
    pub fn new(
        signal: impl Signal<Value = T> + 'static,
        fallback: impl FnMut() -> BoxedComponent + 'static,
    ) -> Self {
        Self {
            signal: Box::new(signal),
            cases: Vec::new(),
            fallback: Box::new(fallback),
        }
    }

    /// Add a case.
    ///
    /// `extractor` receives `&mut T` and returns `Some(e)` if the value matches. The mutable
    /// reference lets you move fields out via [`std::mem::take`]. Extractors that do not match
    /// must leave `T` unmodified so subsequent cases see the original value.
    ///
    /// `factory` is called once when the case activates. It receives a signal whose value
    /// tracks the extracted `E` for as long as the case stays active. The signal type is
    /// intentionally abstract — write `|sig| { ... }` and let type inference handle it, or
    /// annotate with [`ReadSignal<E>`] if an explicit type is needed.
    pub fn case<E: Clone + 'static>(
        mut self,
        mut extractor: impl FnMut(&mut T) -> Option<E> + 'static,
        mut factory: impl FnMut(ReadSignal<E>) -> BoxedComponent + 'static,
    ) -> Self {
        let active: Rc<RefCell<Option<StoredSignal<E>>>> = Rc::new(RefCell::new(None));

        let active1 = Rc::clone(&active);
        let try_match = Box::new(
            move |value: &mut T, scope: &ReactiveScope| -> Option<Option<BoxedComponent>> {
                let e = extractor(value)?;
                let mut slot = active1.borrow_mut();
                if let Some(sig) = *slot {
                    // Already active: update the signal in-place; no rebuild needed.
                    sig.set_and_notify_changes(e);
                    Some(None)
                } else {
                    // Newly activated: create the signal and call the factory once.
                    let sig = scope.create_signal(e);
                    *slot = Some(sig);
                    Some(Some(factory(ReadSignal(sig))))
                }
            },
        );

        let active2 = Rc::clone(&active);
        let deactivate = Box::new(move || {
            if let Some(sig) = active2.borrow_mut().take() {
                remove_signal(sig.id());
            }
        });

        self.cases.push(Case {
            try_match,
            deactivate,
        });
        self
    }
}

impl<T: Clone + 'static> Component for Match<T> {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let signal = self.signal;
        let mut cases = self.cases;
        let mut fallback = self.fallback;
        let my_id = ctx.component_id();

        // State: None = fallback active, Some(i) = case i active.
        ctx.create_effect(
            move |scope: &ReactiveScope, prev_branch: Option<Option<usize>>| {
                let mut value = signal.read();

                // Find the first matching case.
                let mut matched: Option<(usize, Option<BoxedComponent>)> = None;
                for (i, case) in cases.iter_mut().enumerate() {
                    if matched.is_none() {
                        if let Some(component_opt) = (case.try_match)(&mut value, scope) {
                            matched = Some((i, component_opt));
                        }
                    }
                }

                let new_branch: Option<usize> = matched.as_ref().map(|(i, _)| *i);

                let should_rebuild = match prev_branch {
                    None => true,
                    Some(prev) => {
                        prev != new_branch                           // active case changed
                        || matches!(matched, Some((_, Some(_)))) // same case, just activated
                    }
                };

                if should_rebuild {
                    // Dispose the old child first (removes its effects), then deactivate the
                    // old case's signal so it can be freed without risking a dangling read.
                    scope.dispose_all_children(my_id);

                    if let Some(prev) = prev_branch {
                        if prev != new_branch {
                            if let Some(old_idx) = prev {
                                (cases[old_idx].deactivate)();
                            }
                        }
                    }

                    let component = matched.and_then(|(_, c)| c).unwrap_or_else(|| fallback());
                    let child_id = scope.create_child_component(Some(my_id));
                    component.setup(&mut SetupContext {
                        component_id: child_id,
                        scope: scope.clone(),
                    });
                }

                new_branch
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ReactiveScope;
    use crate::ResourceState;
    use futures::task::noop_waker_ref;
    use std::sync::{Arc, Mutex};
    use std::task::Context;

    /// Helper: build a Match over ResourceState<i32> that logs factory activations.
    fn make_match(
        signal: StoredSignal<ResourceState<i32>>,
        log: Arc<Mutex<Vec<String>>>,
    ) -> Box<Match<ResourceState<i32>>> {
        let log1 = Arc::clone(&log);
        let log2 = Arc::clone(&log);
        Box::new(
            Match::new(signal, || Box::new(()))
                .case(
                    |s| match s {
                        ResourceState::Loading(last) => Some(std::mem::take(last)),
                        _ => None,
                    },
                    move |sig: ReadSignal<Option<i32>>| -> BoxedComponent {
                        let log = Arc::clone(&log1);
                        // Log the signal value at setup time (activation snapshot).
                        Box::new(move |ctx: &mut SetupContext| {
                            let _ = ctx;
                            log.lock()
                                .unwrap()
                                .push(format!("loading({:?})", sig.read()));
                        })
                    },
                )
                .case(
                    |s| match s {
                        ResourceState::Ready(v) => Some(std::mem::take(v)),
                        _ => None,
                    },
                    move |sig: ReadSignal<i32>| -> BoxedComponent {
                        let log = Arc::clone(&log2);
                        Box::new(move |ctx: &mut SetupContext| {
                            let _ = ctx;
                            log.lock().unwrap().push(format!("ready({})", sig.read()));
                        })
                    },
                ),
        )
    }

    #[test]
    fn test_match_initial_render() {
        let scope = ReactiveScope::default();
        let root = scope.create_child_component(None);
        let signal = scope.create_signal(ResourceState::<i32>::Loading(None));
        let log = Arc::new(Mutex::new(Vec::<String>::new()));

        make_match(signal, Arc::clone(&log)).setup(&mut SetupContext {
            scope: scope.clone(),
            component_id: root,
        });

        assert_eq!(*log.lock().unwrap(), vec!["loading(None)"]);
    }

    #[test]
    fn test_match_same_branch_no_rebuild() {
        let scope = ReactiveScope::default();
        let root = scope.create_child_component(None);
        let signal = scope.create_signal(ResourceState::<i32>::Loading(None));
        let log = Arc::new(Mutex::new(Vec::<String>::new()));

        make_match(signal, Arc::clone(&log)).setup(&mut SetupContext {
            scope: scope.clone(),
            component_id: root,
        });
        assert_eq!(*log.lock().unwrap(), vec!["loading(None)"]);

        // E changes within the same branch: signal is updated in-place, no factory call.
        signal.set_and_notify_changes(ResourceState::Loading(Some(42)));
        scope.tick(&mut Context::from_waker(noop_waker_ref()));
        assert_eq!(*log.lock().unwrap(), vec!["loading(None)"]);
    }

    #[test]
    fn test_match_branch_change_rebuilds() {
        let scope = ReactiveScope::default();
        let root = scope.create_child_component(None);
        let signal = scope.create_signal(ResourceState::<i32>::Loading(None));
        let log = Arc::new(Mutex::new(Vec::<String>::new()));

        make_match(signal, Arc::clone(&log)).setup(&mut SetupContext {
            scope: scope.clone(),
            component_id: root,
        });
        assert_eq!(*log.lock().unwrap(), vec!["loading(None)"]);

        // Update E within Loading — no rebuild.
        signal.set_and_notify_changes(ResourceState::Loading(Some(42)));
        scope.tick(&mut Context::from_waker(noop_waker_ref()));
        assert_eq!(*log.lock().unwrap(), vec!["loading(None)"]);

        // Branch changes to Ready — factory called once with new signal.
        signal.set_and_notify_changes(ResourceState::Ready(99));
        scope.tick(&mut Context::from_waker(noop_waker_ref()));
        assert_eq!(*log.lock().unwrap(), vec!["loading(None)", "ready(99)"]);
    }

    #[test]
    fn test_match_signal_updates_reactively() {
        // Verify that the signal passed to the factory reflects E changes, so child
        // effects that read it will observe the latest value.
        let scope = ReactiveScope::default();
        let root = scope.create_child_component(None);
        let profile = scope.create_signal(ResourceState::<i32>::Loading(None));
        let log = Arc::new(Mutex::new(Vec::<String>::new()));

        let log_clone = Arc::clone(&log);
        Box::new(Match::new(profile, || Box::new(())).case(
            |s| match s {
                ResourceState::Loading(last) => Some(std::mem::take(last)),
                _ => None,
            },
            move |sig: ReadSignal<Option<i32>>| -> BoxedComponent {
                let log = Arc::clone(&log_clone);
                Box::new(move |ctx: &mut SetupContext| {
                    // Inner effect reads the signal reactively.
                    ctx.create_effect(move |_, _: Option<()>| {
                        log.lock()
                            .unwrap()
                            .push(format!("loading({:?})", sig.read()));
                    });
                })
            },
        ))
        .setup(&mut SetupContext {
            scope: scope.clone(),
            component_id: root,
        });

        // Effect ran at setup.
        assert_eq!(*log.lock().unwrap(), vec!["loading(None)"]);

        // E changes: Match effect updates the signal; inner effect re-runs next tick.
        profile.set_and_notify_changes(ResourceState::Loading(Some(42)));
        scope.tick(&mut Context::from_waker(noop_waker_ref())); // Match updates signal
        scope.tick(&mut Context::from_waker(noop_waker_ref())); // inner effect sees new value
        assert_eq!(
            *log.lock().unwrap(),
            vec!["loading(None)", "loading(Some(42))"]
        );
    }

    #[test]
    fn test_match_fallback() {
        let scope = ReactiveScope::default();
        let root = scope.create_child_component(None);
        let signal = scope.create_signal(0i32);
        let log = Arc::new(Mutex::new(Vec::<String>::new()));

        let log_clone = Arc::clone(&log);
        Box::new(Match::new(signal, move || -> BoxedComponent {
            let log = Arc::clone(&log_clone);
            Box::new(move |ctx: &mut SetupContext| {
                let _ = ctx;
                log.lock().unwrap().push("fallback".into())
            })
        }))
        .setup(&mut SetupContext {
            scope: scope.clone(),
            component_id: root,
        });

        assert_eq!(*log.lock().unwrap(), vec!["fallback"]);
    }
}
