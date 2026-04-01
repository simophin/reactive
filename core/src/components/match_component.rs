use crate::ComponentId;
use crate::component::{Component, SetupContext};

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
            $pattern => ::std::result::Result::Ok($expr),
            v => ::std::result::Result::Err(v),
        }
    };
}
use crate::ReactiveScope;
use crate::signal::stored::ReadStoredSignal;
use crate::signal::{Signal, StoredSignal};

/// A reactive `match`-like component.
///
/// Each case supplies an *extractor* `FnMut(&mut T) -> Option<E>` and a *factory*
/// `FnMut(StoredSignal<E>) -> BoxedComponent`.
///
/// When a case first becomes active the factory is called once with a freshly created signal
/// holding the extracted value `E`. While the same case remains active, subsequent changes to
/// `E` are pushed into that signal — the child component reacts through its own effects without
/// being rebuilt. The child is only rebuilt when the active case index changes.
pub struct Match<S: Signal> {
    signal: S,
    cases: Vec<Box<dyn FnMut(&ReactiveScope, ComponentId, S::Value, bool) -> Result<(), S::Value>>>,
    fallback: Box<dyn FnMut(&ReactiveScope, ComponentId)>,
}

impl<S: Signal> Match<S> {
    pub fn new<C: Component + 'static>(
        signal: S,
        mut fallback: impl FnMut() -> C + 'static,
    ) -> Self {
        Self {
            signal,
            cases: Vec::new(),
            fallback: Box::new(move |scope, component| {
                scope.dispose_all_children(component);
                scope.setup_child(component, |ctx| Box::new(fallback()).setup(ctx));
            }),
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
    pub fn case<E: PartialEq + 'static, C: Component + 'static>(
        mut self,
        mut extractor: impl FnMut(S::Value) -> Result<E, S::Value> + 'static,
        mut factory: impl FnMut(ReadStoredSignal<E>) -> C + 'static,
    ) -> Self {
        let mut case_signal: Option<StoredSignal<E>> = None;

        self.cases.push(Box::new(
            move |scope, component_id, value, is_active| -> Result<(), S::Value> {
                let e = match extractor(value) {
                    Ok(extracted) => extracted,
                    Err(value) => return Err(value),
                };

                match (&case_signal, is_active) {
                    (Some(signal), true) => {
                        signal.update_if_changes(e);
                    }
                    (Some(signal), false) => {
                        signal.update_if_changes(e);
                        scope.dispose_all_children(component_id);

                        let comp = Box::new(factory(signal.clone().read_only()));
                        scope.setup_child(component_id, |ctx| comp.setup(ctx));
                    }
                    (None, _) => {
                        let signal = scope.create_signal(e);
                        case_signal = Some(signal.clone());
                        scope.dispose_all_children(component_id);

                        let comp = Box::new(factory(signal.clone().read_only()));
                        scope.setup_child(component_id, |ctx| comp.setup(ctx));
                    }
                }

                Ok(())
            },
        ));

        self
    }
}

enum ActiveBranch {
    Case(usize),
    Fallback,
}

impl<S> Component for Match<S>
where
    S: Signal + 'static,
    S::Value: 'static,
{
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let signal = self.signal;
        let mut cases = self.cases;
        let mut fallback = self.fallback;
        let my_id = ctx.component_id();

        // State: None = fallback active, Some(i) = case i active.
        ctx.create_effect(
            move |scope: &ReactiveScope, active_branch: Option<ActiveBranch>| {
                let mut value = signal.read();

                for (index, case) in cases.iter_mut().enumerate() {
                    let is_active = matches!(active_branch, Some(ActiveBranch::Case(active)) if active == index);
                    match case(scope, my_id, value, is_active) {
                        Ok(()) => return ActiveBranch::Case(index),
                        Err(e) => value = e,
                    }
                }

                match active_branch {
                    Some(ActiveBranch::Case(_)) | None => {
                        fallback(scope, my_id);
                    }
                    Some(ActiveBranch::Fallback) => {}
                }

                ActiveBranch::Fallback
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
    fn make_match<S: Signal<Value = ResourceState<i32>> + 'static>(
        signal: S,
        log: Arc<Mutex<Vec<String>>>,
    ) -> Box<Match<S>> {
        let log1 = Arc::clone(&log);
        let log2 = Arc::clone(&log);
        Box::new(
            Match::new(signal, || ())
                .case(
                    |s| match s {
                        ResourceState::Loading(last) => Ok(last),
                        other => Err(other),
                    },
                    move |sig: ReadStoredSignal<Option<i32>>| {
                        let log = Arc::clone(&log1);
                        // Log the signal value at setup time (activation snapshot).
                        move |ctx: &mut SetupContext| {
                            let _ = ctx;
                            log.lock()
                                .unwrap()
                                .push(format!("loading({:?})", sig.read()));
                        }
                    },
                )
                .case(
                    |s| match s {
                        ResourceState::Ready(v) => Ok(v),
                        other => Err(other),
                    },
                    move |sig: ReadStoredSignal<i32>| {
                        let log = Arc::clone(&log2);
                        move |ctx: &mut SetupContext| {
                            let _ = ctx;
                            log.lock().unwrap().push(format!("ready({})", sig.read()));
                        }
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

        make_match(signal.clone(), Arc::clone(&log)).setup(&mut SetupContext {
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

        make_match(signal.clone(), Arc::clone(&log)).setup(&mut SetupContext {
            scope: scope.clone(),
            component_id: root,
        });
        assert_eq!(*log.lock().unwrap(), vec!["loading(None)"]);

        // E changes within the same branch: signal is updated in-place, no factory call.
        signal.update(ResourceState::Loading(Some(42)));
        scope.tick(&mut Context::from_waker(noop_waker_ref()));
        assert_eq!(*log.lock().unwrap(), vec!["loading(None)"]);
    }

    #[test]
    fn test_match_branch_change_rebuilds() {
        let scope = ReactiveScope::default();
        let root = scope.create_child_component(None);
        let signal = scope.create_signal(ResourceState::<i32>::Loading(None));
        let log = Arc::new(Mutex::new(Vec::<String>::new()));

        make_match(signal.clone(), Arc::clone(&log)).setup(&mut SetupContext {
            scope: scope.clone(),
            component_id: root,
        });
        assert_eq!(*log.lock().unwrap(), vec!["loading(None)"]);

        // Update E within Loading — no rebuild.
        signal.update(ResourceState::Loading(Some(42)));
        scope.tick(&mut Context::from_waker(noop_waker_ref()));
        assert_eq!(*log.lock().unwrap(), vec!["loading(None)"]);

        // Branch changes to Ready — factory called once with new signal.
        signal.update(ResourceState::Ready(99));
        scope.tick(&mut Context::from_waker(noop_waker_ref()));
        assert_eq!(*log.lock().unwrap(), vec!["loading(None)", "ready(99)"]);
    }

    #[test]
    fn test_match_rebuilds_when_returning_to_a_previous_case() {
        let scope = ReactiveScope::default();
        let root = scope.create_child_component(None);
        let signal = scope.create_signal(ResourceState::<i32>::Loading(None));
        let log = Arc::new(Mutex::new(Vec::<String>::new()));

        make_match(signal.clone(), Arc::clone(&log)).setup(&mut SetupContext {
            scope: scope.clone(),
            component_id: root,
        });
        assert_eq!(*log.lock().unwrap(), vec!["loading(None)"]);

        signal.update(ResourceState::Ready(1));
        scope.tick(&mut Context::from_waker(noop_waker_ref()));

        signal.update(ResourceState::Loading(Some(1)));
        scope.tick(&mut Context::from_waker(noop_waker_ref()));

        signal.update(ResourceState::Ready(2));
        scope.tick(&mut Context::from_waker(noop_waker_ref()));

        assert_eq!(
            *log.lock().unwrap(),
            vec!["loading(None)", "ready(1)", "loading(Some(1))", "ready(2)"]
        );
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
        Box::new(Match::new(profile.clone(), || ()).case(
            |s| match s {
                ResourceState::Loading(last) => Ok(last),
                other => Err(other),
            },
            move |sig: ReadStoredSignal<Option<i32>>| {
                let log = Arc::clone(&log_clone);
                move |ctx: &mut SetupContext| {
                    // Inner effect reads the signal reactively.
                    ctx.create_effect(move |_, _: Option<()>| {
                        log.lock()
                            .unwrap()
                            .push(format!("loading({:?})", sig.read()));
                    });
                }
            },
        ))
        .setup(&mut SetupContext {
            scope: scope.clone(),
            component_id: root,
        });

        // Effect ran at setup.
        assert_eq!(*log.lock().unwrap(), vec!["loading(None)"]);

        // E changes: Match effect updates the signal; inner effect re-runs next tick.
        profile.update(ResourceState::Loading(Some(42)));
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
        Box::new(Match::new(signal.clone(), move || {
            let log = Arc::clone(&log_clone);
            move |ctx: &mut SetupContext| {
                let _ = ctx;
                log.lock().unwrap().push("fallback".into())
            }
        }))
        .setup(&mut SetupContext {
            scope: scope.clone(),
            component_id: root,
        });

        assert_eq!(*log.lock().unwrap(), vec!["fallback"]);
    }
}
