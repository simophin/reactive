use crate::component_scope::{ComponentId, ComponentScope, Effect};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Weak};
use std::task::{Context, Poll, Wake, Waker};

// ---------------------------------------------------------------------------
// Per-future waker
// ---------------------------------------------------------------------------

struct FutureWaker {
    flag: Weak<AtomicBool>,
    outer: Waker,
}

impl Wake for FutureWaker {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        // If the Arc is gone the EffectState was dropped (component disposed) — do nothing.
        if let Some(flag) = self.flag.upgrade() {
            flag.store(true, Ordering::Release);
            self.outer.wake_by_ref();
        }
    }
}

fn make_waker(flag: Weak<AtomicBool>, outer: &Waker) -> Waker {
    Waker::from(Arc::new(FutureWaker {
        flag,
        outer: outer.clone(),
    }))
}

// ---------------------------------------------------------------------------
// Tick
// ---------------------------------------------------------------------------

use super::ReactiveScope;

impl ReactiveScope {
    pub(crate) fn traverse_tree_depth_last(
        &mut self,
        start: ComponentId,
        f: &mut impl FnMut(ComponentId, &mut ComponentScope),
    ) {
        let Some(scope) = self.components.get_mut(start) else {
            return;
        };

        f(start, scope);

        let children = std::mem::take(&mut scope.children);
        for child in &children {
            self.traverse_tree_depth_last(*child, f);
        }

        if let Some(scope) = self.components.get_mut(start) {
            scope.children = children;
        }
    }

    pub fn tick(&mut self, future_ctx: &mut Context) {
        struct EffectUpdate {
            component_id: ComponentId,
            effect: Effect,
            dirty: bool,
        }

        self.dirty_signals.set_waker(future_ctx.waker().clone());

        let mut updates = Vec::new();
        let root = std::mem::take(&mut self.root);
        let dirty_signal_set = self.dirty_signals.take();

        for component in &root {
            self.traverse_tree_depth_last(*component, &mut |component_id, c| {
                for effect in std::mem::take(&mut c.active_effects) {
                    let dirty = effect.signal_accessed.intersects(&dirty_signal_set);

                    let future_needs_poll = effect
                        .in_flight
                        .as_ref()
                        .is_some_and(|f| f.woken.load(Ordering::Acquire));

                    if dirty || future_needs_poll {
                        updates.push(EffectUpdate {
                            component_id,
                            effect,
                            dirty,
                        });
                    } else {
                        c.active_effects.push(effect);
                    }
                }
            });
        }

        self.root = root;

        for mut update in updates {
            if update.dirty {
                // Cancel any in-flight future — the inputs changed, so it's stale.
                let (signal_accessed, in_flight) = (update.effect.effect_fn)(self);
                update.effect.signal_accessed = signal_accessed;
                update.effect.in_flight = in_flight;
            }

            // Single poll path: fresh futures start with woken=true, in-flight futures
            // have woken set by their waker.
            if let Some(ref mut in_flight) = update.effect.in_flight {
                // Reset before polling so any wake() concurrent with the poll is
                // visible on the next tick's Acquire load.
                in_flight.woken.store(false, Ordering::SeqCst);
                let waker = make_waker(Arc::downgrade(&in_flight.woken), future_ctx.waker());
                let mut ctx = Context::from_waker(&waker);
                if let Poll::Ready(()) = in_flight.future.as_mut().poll(&mut ctx) {
                    update.effect.in_flight = None;
                }
            }

            if let Some(c) = self.components.get_mut(update.component_id) {
                c.push_effect(update.effect);
            }
        }
    }
}
