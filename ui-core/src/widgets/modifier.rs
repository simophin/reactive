use reactive_core::{Component, ContextKey, SetupContext, Signal};
use std::any::Any;
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::LazyLock;
use std::sync::atomic::{AtomicUsize, Ordering};

type ModifierKeyId = usize;

pub struct ModifierKey<T> {
    id: LazyLock<ModifierKeyId>,
    merger: fn(&dyn Signal<Value = T>, T) -> T,
    _marker: PhantomData<fn() -> T>,
}

impl<T> ModifierKey<T> {
    pub const fn with_merger(merger: fn(&dyn Signal<Value = T>, T) -> T) -> Self {
        Self {
            id: LazyLock::new(|| {
                static COUNTER: AtomicUsize = AtomicUsize::new(0);
                COUNTER.fetch_add(1, Ordering::Relaxed)
            }),
            merger,
            _marker: PhantomData,
        }
    }

    pub const fn new() -> Self {
        Self::with_merger(|_old, new| new)
    }

    pub fn id(&self) -> ModifierKeyId {
        *self.id
    }
}

struct ModifierValue {
    id: ModifierKeyId,
    signal: Rc<dyn Any>, // A type-erased Rc<dyn Signal<Value = T>>.
    merger: Rc<dyn Fn(&Rc<dyn Any>, &Rc<dyn Any>) -> Rc<dyn Any>>,
}

#[derive(Default)]
pub struct Modifier {
    values: Vec<ModifierValue>,
}

impl Clone for Modifier {
    fn clone(&self) -> Self {
        Self {
            values: self.values.clone(),
        }
    }
}

impl Clone for ModifierValue {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            signal: Rc::clone(&self.signal),
            merger: Rc::clone(&self.merger),
        }
    }
}

impl Modifier {
    pub fn new() -> Self {
        Self { values: Vec::new() }
    }

    fn box_signal<T: 'static>(s: impl Signal<Value = T> + 'static) -> Rc<dyn Any> {
        let boxed_signal: Rc<dyn Signal<Value = T>> = Rc::new(s);
        Rc::new(boxed_signal)
    }

    fn unbox_signal<T: 'static>(s: &Rc<dyn Any>) -> Rc<dyn Signal<Value = T>> {
        s.downcast_ref::<Rc<dyn Signal<Value = T>>>()
            .unwrap()
            .clone()
    }

    pub fn with<T: 'static>(
        mut self,
        k: &ModifierKey<T>,
        signal: impl Signal<Value = T> + 'static,
    ) -> Self {
        let merger = k.merger;

        match self.values.binary_search_by_key(&k.id(), |v| v.id) {
            Ok(found) => {
                let old_signal = Self::unbox_signal(&self.values[found].signal);
                self.values[found].signal =
                    Self::box_signal(move || merger(old_signal.as_ref(), signal.read()))
            }
            Err(insert) => self.values.insert(
                insert,
                ModifierValue {
                    id: k.id(),
                    signal: Self::box_signal(signal),
                    merger: Rc::new(move |old, new| {
                        let old_signal = Self::unbox_signal::<T>(old);
                        let new_signal = Self::unbox_signal::<T>(new);
                        Self::box_signal(move || merger(old_signal.as_ref(), new_signal.read()))
                    }),
                },
            ),
        }

        self
    }

    pub fn get<T: 'static>(&self, k: &ModifierKey<T>) -> Option<Rc<dyn Signal<Value = T>>> {
        self.values
            .binary_search_by_key(&k.id(), |v| v.id)
            .ok()
            .and_then(|found| self.values[found].signal.downcast_ref().cloned())
    }

    pub fn then(mut self, another: Self) -> Self {
        for value in another.values {
            match self.values.binary_search_by_key(&value.id, |v| v.id) {
                Ok(found) => {
                    // Merge the two signals using the merger function.
                    let old_value = &mut self.values[found];
                    old_value.signal = (value.merger)(&old_value.signal, &value.signal);
                }
                Err(insert) => self.values.insert(insert, value),
            }
        }

        self
    }
}

pub trait WithModifier {
    fn modifier(self, modifier: Modifier) -> Self;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    static SUM_KEY: ModifierKey<i32> =
        ModifierKey::with_merger(|old_signal, new_value| old_signal.read() + new_value);
    static PRODUCT_KEY: ModifierKey<i32> =
        ModifierKey::with_merger(|old_signal, new_value| old_signal.read() * new_value);

    #[test]
    fn new_modifier_has_no_values() {
        let modifier = Modifier::new();

        assert!(modifier.get(&SUM_KEY).is_none());
    }

    #[test]
    fn with_stores_value_for_key() {
        let modifier = Modifier::new().with(&SUM_KEY, 4);

        assert_eq!(modifier.get(&SUM_KEY).unwrap().read(), 4);
    }

    #[test]
    fn with_merges_repeated_key_in_order() {
        let modifier = Modifier::new().with(&SUM_KEY, 4).with(&SUM_KEY, 7);

        assert_eq!(modifier.get(&SUM_KEY).unwrap().read(), 11);
    }

    #[test]
    fn with_keeps_merged_signals_live() {
        let left = Rc::new(Cell::new(2));
        let right = Rc::new(Cell::new(3));

        let modifier = Modifier::new()
            .with(&SUM_KEY, {
                let left = Rc::clone(&left);
                move || left.get()
            })
            .with(&SUM_KEY, {
                let right = Rc::clone(&right);
                move || right.get()
            });

        let signal = modifier.get(&SUM_KEY).unwrap();
        assert_eq!(signal.read(), 5);

        left.set(10);
        right.set(-4);

        assert_eq!(signal.read(), 6);
    }

    #[test]
    fn then_combines_distinct_keys() {
        let modifier = Modifier::new()
            .with(&SUM_KEY, 4)
            .then(Modifier::new().with(&PRODUCT_KEY, 6));

        assert_eq!(modifier.get(&SUM_KEY).unwrap().read(), 4);
        assert_eq!(modifier.get(&PRODUCT_KEY).unwrap().read(), 6);
    }

    #[test]
    fn then_merges_overlapping_keys() {
        let modifier = Modifier::new()
            .with(&SUM_KEY, 4)
            .then(Modifier::new().with(&SUM_KEY, 7));

        assert_eq!(modifier.get(&SUM_KEY).unwrap().read(), 11);
    }

    #[test]
    fn then_uses_each_keys_own_merger() {
        let modifier = Modifier::new()
            .with(&PRODUCT_KEY, 3)
            .then(Modifier::new().with(&PRODUCT_KEY, 5));

        assert_eq!(modifier.get(&PRODUCT_KEY).unwrap().read(), 15);
    }
}
