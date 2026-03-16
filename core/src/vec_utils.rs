/// Removes all elements matching the predicate from `vec` and returns them.
/// Uses an in-place partition (swap to back, then split_off) — one pass, no extra index vec.
pub fn extract_if<T>(vec: &mut Vec<T>, mut predicate: impl FnMut(&T) -> bool) -> Vec<T> {
    let mut split = vec.len();
    let mut i = 0;

    while i < split {
        if predicate(&vec[i]) {
            split -= 1;
            vec.swap(i, split);
        } else {
            i += 1;
        }
    }

    vec.split_off(split)
}
