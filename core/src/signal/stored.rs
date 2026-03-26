use std::cell::RefCell;
use std::rc::Rc;

use crate::SignalWrapper;
use crate::reactive_scope::WeakReactiveScope;
use crate::signal::Signal;
// ---------------------------------------------------------------------------
// SignalInner — the single heap allocation shared by all clones of a signal
// ---------------------------------------------------------------------------

pub(crate) struct SignalInner<T> {
    pub value: RefCell<T>,
    scope: WeakReactiveScope,
}

/// The identity of a signal, derived from the pointer address of its heap allocation.
/// Stable for the lifetime of the signal; used in sorted dependency sets.
pub(crate) type SignalId = usize;

// ---------------------------------------------------------------------------
// StoredSignal
// ---------------------------------------------------------------------------

pub struct StoredSignal<T>(Rc<SignalInner<T>>);

pub type ReadStoredSignal<T> = SignalWrapper<StoredSignal<T>, ()>;

impl<T> Clone for StoredSignal<T> {
    fn clone(&self) -> Self {
        Self(Rc::clone(&self.0))
    }
}

impl<T> StoredSignal<T> {
    pub(crate) fn new(init: T, scope: WeakReactiveScope) -> Self
    where
        T: 'static,
    {
        Self(Rc::new(SignalInner {
            value: RefCell::new(init),
            scope,
        }))
    }

    /// The signal's identity — stable pointer address of the shared allocation.
    pub(crate) fn id(&self) -> SignalId {
        Rc::as_ptr(&self.0) as usize
    }

    pub fn read_only(self) -> ReadStoredSignal<T> {
        SignalWrapper::new(self, ())
    }

    /// Update the value in-place. The closure returns `true` if the value changed
    /// and the signal should be marked dirty.
    pub fn update_with(&self, f: impl FnOnce(&mut T) -> bool)
    where
        T: 'static,
    {
        let changed = f(&mut self.0.value.borrow_mut());
        if changed {
            if let Some(scope) = self.0.scope.upgrade() {
                scope.0.borrow().dirty_signals.mark_dirty(self.id());
            }
        }
    }

    pub fn update<S: Into<T>>(&self, value: S)
    where
        T: 'static,
    {
        self.update_with(|v| {
            *v = value.into();
            true
        });
    }

    pub fn update_if_changes<S: Into<T>>(&self, value: S)
    where
        S: PartialEq<T>,
        T: 'static,
    {
        self.update_with(move |v| {
            if &value != v {
                *v = value.into();
                true
            } else {
                false
            }
        });
    }
}

impl<T: Clone + 'static> Signal for StoredSignal<T> {
    type Value = T;

    fn read(&self) -> T {
        if let Some(scope) = self.0.scope.upgrade() {
            scope
                .0
                .borrow()
                .active_signal_tracker
                .on_accessed(self.id());
        }
        self.0.value.borrow().clone()
    }
}
