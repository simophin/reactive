pub trait VecExt<T> {
    fn condense(&mut self);
}

impl<T> VecExt<Option<T>> for Vec<Option<T>> {
    fn condense(&mut self) {
        self.retain(Option::is_some);
    }
}
