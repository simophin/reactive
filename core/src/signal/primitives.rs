use super::Signal;

macro_rules! impl_signal_copy {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl Signal for $ty {
                type Value = $ty;
                #[inline]
                fn read(&self) -> $ty {
                    *self
                }
            }
        )+
    };
}

impl_signal_copy!(
    f32, f64, i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, usize, isize, bool, char,
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
