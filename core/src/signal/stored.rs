use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::reactive_scope::WeakReactiveScope;
use crate::signal::{Signal, SignalId};

static SIGNAL_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

// ---------------------------------------------------------------------------
// Global signal store
// ---------------------------------------------------------------------------

pub(crate) struct SignalData {
    pub value: Box<dyn Any>,
    /// Weak back-reference to the owning scope, used to notify dirty tracking
    /// and dependency tracking without keeping the scope alive.
    pub scope: WeakReactiveScope,
}

thread_local! {
    static SIGNALS: Rc<RefCell<HashMap<SignalId, SignalData>>> =
        Rc::new(RefCell::new(HashMap::new()));
}

pub(crate) fn remove_signal(id: SignalId) {
    SIGNALS.with(|map| {
        map.borrow_mut().remove(&id);
    });
}

// ---------------------------------------------------------------------------
// StoredSignal
// ---------------------------------------------------------------------------

pub struct StoredSignal<T> {
    id: SignalId,
    _marker: PhantomData<T>,
}

impl<T> Copy for StoredSignal<T> {}

impl<T> Clone for StoredSignal<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> StoredSignal<T> {
    pub(crate) fn new(init: T, scope: WeakReactiveScope) -> Self
    where
        T: 'static,
    {
        let id = SIGNAL_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        SIGNALS.with(|map| {
            map.borrow_mut().insert(
                id,
                SignalData {
                    value: Box::new(init),
                    scope,
                },
            );
        });
        Self {
            id,
            _marker: PhantomData,
        }
    }

    pub(crate) fn id(&self) -> SignalId {
        self.id
    }

    pub fn update(&self, f: impl FnOnce(&mut T) -> bool)
    where
        T: 'static,
    {
        SIGNALS.with(|map| {
            let mut map = map.borrow_mut();
            if let Some(data) = map.get_mut(&self.id) {
                let value = data
                    .value
                    .downcast_mut::<T>()
                    .expect("signal type mismatch");
                if f(value) {
                    if let Some(scope) = data.scope.upgrade() {
                        scope.0.borrow().dirty_signals.mark_dirty(self.id);
                    }
                }
            }
        });
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
        SIGNALS.with(|map| {
            let map = map.borrow();
            let data = map.get(&self.id).expect("signal not found");
            if let Some(scope) = data.scope.upgrade() {
                scope.0.borrow().active_signal_tracker.on_accessed(self.id);
            }
            data.value
                .downcast_ref::<T>()
                .expect("signal type mismatch")
                .clone()
        })
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
        Box::new(*self)
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

impl<T> Copy for ReadSignal<T> {}
impl<T> Clone for ReadSignal<T> {
    fn clone(&self) -> Self {
        *self
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
