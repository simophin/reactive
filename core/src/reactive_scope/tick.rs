use crate::component_scope::{ComponentId, Effect};
use crate::reactive_scope::ReactiveScopeData;
use crate::signal::SignalId;
use crate::sorted_vec::SortedVec;
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
// Traversal + effect collection (operates on ReactiveScopeData directly,
// no user closures called — safe to hold borrow throughout)
// ---------------------------------------------------------------------------

struct EffectUpdate {
    component_id: ComponentId,
    effect: Effect,
    dirty: bool,
}

fn traverse_collect(
    data: &mut ReactiveScopeData,
    start: ComponentId,
    dirty: &SortedVec<SignalId>,
    updates: &mut Vec<EffectUpdate>,
) {
    let Some(scope) = data.components.get_mut(start) else {
        return;
    };

    let effects = std::mem::take(&mut scope.active_effects);
    let children = std::mem::take(&mut scope.children);

    let mut keep = Vec::new();
    for effect in effects {
        let is_dirty = effect.signal_accessed.intersects(dirty);
        let future_needs_poll = effect
            .in_flight
            .as_ref()
            .is_some_and(|f| f.woken.load(Ordering::Acquire));

        if is_dirty || future_needs_poll {
            updates.push(EffectUpdate {
                component_id: start,
                effect,
                dirty: is_dirty,
            });
        } else {
            keep.push(effect);
        }
    }

    if let Some(scope) = data.components.get_mut(start) {
        scope.active_effects = keep;
    }

    for child in &children {
        traverse_collect(data, *child, dirty, updates);
    }

    if let Some(scope) = data.components.get_mut(start) {
        scope.children = children;
    }
}

// ---------------------------------------------------------------------------
// Tick
// ---------------------------------------------------------------------------

use super::ReactiveScope;

impl ReactiveScope {
    pub fn tick(&self, future_ctx: &mut Context) {
        // Phase 1: collect dirty effects while holding the borrow.
        // No user closures are called here, so it is safe to hold borrow_mut.
        let mut updates: Vec<EffectUpdate> = Vec::new();
        {
            let mut data = self.0.borrow_mut();
            data.dirty_signals.set_waker(future_ctx.waker().clone());
            let dirty_signal_set = data.dirty_signals.take();
            let root = std::mem::take(&mut data.root);

            for &component in &root {
                traverse_collect(&mut data, component, &dirty_signal_set, &mut updates);
            }

            data.root = root;
        } // borrow released

        // Phase 2: execute effects with no borrow held.
        // Effects receive &ReactiveScope and may freely call any scope method.
        for mut update in updates {
            if update.dirty {
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

            // Phase 3: push the effect back — brief borrow.
            if let Some(c) = self.0.borrow_mut().components.get_mut(update.component_id) {
                c.push_effect(update.effect);
            }
        }
    }
}
