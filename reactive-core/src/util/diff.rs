pub fn diff_sorted<T>(old: impl Iterator<Item = T>, new: impl Iterator<Item = T>) -> DiffResult<T>
where
    T: Ord + Eq,
{
    let mut old = old.peekable();
    let mut new = new.peekable();

    let mut added: Vec<_> = Default::default();
    let mut removed: Vec<_> = Default::default();

    loop {
        match (old.peek(), new.peek()) {
            (Some(old_item), Some(new_item)) => {
                if old_item < new_item {
                    removed.push(old.next().unwrap());
                } else if old_item > new_item {
                    added.push(new.next().unwrap());
                } else {
                    old.next();
                    new.next();
                }
            }
            (Some(_), None) => {
                removed.push(old.next().unwrap());
            }
            (None, Some(_)) => {
                added.push(new.next().unwrap());
            }
            (None, None) => {
                break;
            }
        }
    }

    DiffResult { added, removed }
}

#[derive(Default)]
pub struct DiffResult<T> {
    pub added: Vec<T>,
    pub removed: Vec<T>,
}

impl<T> DiffResult<T> {
    pub fn clear(&mut self) {
        self.added.clear();
        self.removed.clear();
    }
}
