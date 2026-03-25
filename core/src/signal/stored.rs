use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

use crate::reactive_scope::WeakReactiveScope;
use crate::signal::{Signal, SignalId};

// ---------------------------------------------------------------------------
// SignalInner — the single heap allocation shared by all clones of a signal
// ---------------------------------------------------------------------------

pub(crate) struct SignalInner<T> {
    pub value: RefCell<T>,
    scope: WeakReactiveScope,
}

// ---------------------------------------------------------------------------
// StoredSignal
// ---------------------------------------------------------------------------

pub struct StoredSignal<T>(Rc<SignalInner<T>>);

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

    /// Update the value in-place. The closure returns `true` if the value changed
    /// and the signal should be marked dirty.
    pub fn update(&self, f: impl FnOnce(&mut T) -> bool)
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

    pub fn set_and_notify_changes(&self, value: T)
    where
        T: 'static,
    {
        self.update(|v| {
            *v = value;
            true
        });
    }

    pub fn update_if_changes(&self, value: T)
    where
        T: PartialEq + 'static,
    {
        self.update(|v| {
            if v != &value {
                *v = value;
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

// ---------------------------------------------------------------------------
// BoxedStoredSignal — type-erased handle used by the context system
// ---------------------------------------------------------------------------

trait RawStoreSignal: Any {
    fn clone_to_box(&self) -> Box<dyn RawStoreSignal>;
}

impl<T: 'static> RawStoreSignal for StoredSignal<T> {
    fn clone_to_box(&self) -> Box<dyn RawStoreSignal> {
        Box::new(self.clone())
    }
}

pub(crate) struct BoxedStoredSignal(Box<dyn RawStoreSignal>);

impl Clone for BoxedStoredSignal {
    fn clone(&self) -> Self {
        Self(self.0.clone_to_box())
    }
}

impl BoxedStoredSignal {
    pub fn downcast_ref<T: 'static>(&self) -> Option<&StoredSignal<T>> {
        (self.0.as_ref() as &dyn Any).downcast_ref()
    }
}

impl<T: 'static> From<StoredSignal<T>> for BoxedStoredSignal {
    fn from(value: StoredSignal<T>) -> Self {
        Self(Box::new(value))
    }
}

// ---------------------------------------------------------------------------
// ReadSignal — read-only handle to a StoredSignal
// ---------------------------------------------------------------------------

/// A read-only handle to a [`StoredSignal`].
///
/// Implements [`Signal`] but does not expose any write methods. This is the
/// type returned by [`ReactiveScope::create_resource`] and
/// [`ReactiveScope::create_stream`], and used as the parameter type for
/// [`Match`](crate::components::Match) case factories.
pub struct ReadSignal<T>(pub(crate) StoredSignal<T>);

impl<T> Clone for ReadSignal<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: Clone + 'static> Signal for ReadSignal<T> {
    type Value = T;
    fn read(&self) -> T {
        self.0.read()
    }
}

impl<T> From<StoredSignal<T>> for ReadSignal<T> {
    fn from(s: StoredSignal<T>) -> Self {
        Self(s)
    }
}
