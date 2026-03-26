use super::Signal;
use std::num::*;

macro_rules! impl_constant_signal {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl Signal for $ty {
                type Value = $ty;
                #[inline]
                fn read(&self) -> Self::Value {
                    *self
                }
            }
        )+
    };
}

impl_constant_signal!(
    f32,
    f64,
    i8,
    i16,
    i32,
    i64,
    i128,
    u8,
    u16,
    u32,
    u64,
    u128,
    usize,
    isize,
    bool,
    char,
    NonZeroUsize,
    NonZeroI8,
    NonZeroI16,
    NonZeroI32,
    NonZeroI64,
    NonZeroIsize,
    NonZeroU8,
    NonZeroU16,
    NonZeroU32,
    NonZeroU64,
);

impl Signal for String {
    type Value = String;
    #[inline]
    fn read(&self) -> String {
        self.clone()
    }
}

/// Allows string literals to be used directly where `Signal<Value = String>` is expected.
impl Signal for &str {
    type Value = String;
    #[inline]
    fn read(&self) -> String {
        (*self).to_string()
    }
}
