//! Diff algorithm for comparing two [`ListData`] instances using a [`ListComparator`].
//!
//! Inspired by Android's [DiffUtil](https://developer.android.com/reference/androidx/recyclerview/widget/DiffUtil).
//!
//! The algorithm has two phases:
//!
//! 1. **Myers diff** — finds the longest common subsequence using [`ListComparator::is_same_item`]
//!    as the identity predicate. Items matched in the LCS are *keeps*; the rest become raw
//!    removes / inserts.
//! 2. **Move detection** — pairs each raw remove with a raw insert that shares identity (greedy,
//!    first-match). A matched pair becomes [`DiffOp::Move`]; the remainder are grouped into
//!    batched [`DiffOp::Remove`] / [`DiffOp::Insert`] operations.
//!
//! [`ListComparator::are_content_the_same`] is used on every matched pair (keep *or* move) to
//! decide whether to emit a [`DiffOp::Change`] or set [`DiffOp::Move::changed`].

use super::list::{ListComparator, ListData};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// A single operation produced by [`diff`].
#[derive(Debug, Clone, PartialEq)]
pub enum DiffOp {
    /// `count` new items were inserted starting at `index` in the **new** list.
    Insert { index: usize, count: usize },
    /// `count` old items were removed starting at `index` in the **old** list.
    Remove { index: usize, count: usize },
    /// An item moved from `old_index` to `new_index`.
    ///
    /// `changed` is `true` when [`ListComparator::are_content_the_same`] returns `false`
    /// for the pair, meaning the item also needs a content update.
    Move {
        old_index: usize,
        new_index: usize,
        changed: bool,
    },
    /// An item stayed at the same relative position but its content changed.
    ///
    /// [`ListComparator::is_same_item`] returned `true` for the pair while
    /// [`ListComparator::are_content_the_same`] returned `false`.
    Change { old_index: usize, new_index: usize },
}

/// The result of [`diff`].
pub struct DiffResult {
    pub ops: Vec<DiffOp>,
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Compute the diff between `old` and `new` using the provided `comparator`.
///
/// The returned [`DiffResult`] describes the minimum set of operations needed to
/// transform `old` into `new`.
pub fn diff<T, L, C>(old: &L, new: &L, comparator: &C) -> DiffResult
where
    L: ListData<T> + ?Sized,
    C: ListComparator<T>,
{
    let n = old.count();
    let m = new.count();

    // Phase 1 — Myers diff (identity via is_same_item).
    let raw = myers_edits(n, m, |x, y| {
        comparator.is_same_item(
            old.get_item(x).expect("old index in bounds"),
            new.get_item(y).expect("new index in bounds"),
        )
    });

    let mut removes: Vec<usize> = Vec::new();
    let mut inserts: Vec<usize> = Vec::new();
    let mut changes: Vec<(usize, usize)> = Vec::new();

    for edit in raw {
        match edit {
            RawEdit::Keep(oi, ni) => {
                let a = old.get_item(oi).expect("old index in bounds");
                let b = new.get_item(ni).expect("new index in bounds");
                if !comparator.are_content_the_same(a, b) {
                    changes.push((oi, ni));
                }
            }
            RawEdit::Remove(oi) => removes.push(oi),
            RawEdit::Insert(ni) => inserts.push(ni),
        }
    }

    // Phase 2 — Move detection: pair removes with inserts sharing identity.
    let mut matched_inserts = vec![false; inserts.len()];
    let mut move_ops: Vec<DiffOp> = Vec::new();
    let mut unmatched_removes: Vec<usize> = Vec::new();

    'removes: for &old_i in &removes {
        let old_item = old.get_item(old_i).expect("old index in bounds");
        for (j, &new_i) in inserts.iter().enumerate() {
            if !matched_inserts[j] {
                let new_item = new.get_item(new_i).expect("new index in bounds");
                if comparator.is_same_item(old_item, new_item) {
                    let changed = !comparator.are_content_the_same(old_item, new_item);
                    move_ops.push(DiffOp::Move {
                        old_index: old_i,
                        new_index: new_i,
                        changed,
                    });
                    matched_inserts[j] = true;
                    continue 'removes;
                }
            }
        }
        unmatched_removes.push(old_i);
    }

    let unmatched_inserts: Vec<usize> = inserts
        .iter()
        .enumerate()
        .filter(|(j, _)| !matched_inserts[*j])
        .map(|(_, &i)| i)
        .collect();

    // Assemble final ops.
    let mut ops: Vec<DiffOp> = Vec::new();

    group_consecutive(&unmatched_removes, |index, count| {
        ops.push(DiffOp::Remove { index, count });
    });
    group_consecutive(&unmatched_inserts, |index, count| {
        ops.push(DiffOp::Insert { index, count });
    });
    ops.extend(move_ops);
    for (oi, ni) in changes {
        ops.push(DiffOp::Change {
            old_index: oi,
            new_index: ni,
        });
    }

    DiffResult { ops }
}

// ---------------------------------------------------------------------------
// Myers diff — internal
// ---------------------------------------------------------------------------

#[derive(Debug)]
enum RawEdit {
    /// Matched pair: `old[oi]` corresponds to `new[ni]` via `is_same_item`.
    Keep(usize, usize),
    /// `new[ni]` has no counterpart in `old`.
    Insert(usize),
    /// `old[oi]` has no counterpart in `new`.
    Remove(usize),
}

/// Myers' diff algorithm.
///
/// Returns the shortest edit script as an ordered list of [`RawEdit`] values.
/// `equals(x, y)` returns `true` when `old[x]` and `new[y]` should be considered
/// the same item (i.e. kept rather than removed + inserted).
///
/// Time:  O((n + m) · d)  where d is the edit distance.
/// Space: O((n + m)²)     for the trace used during backtracking.
fn myers_edits(n: usize, m: usize, equals: impl Fn(usize, usize) -> bool) -> Vec<RawEdit> {
    if n == 0 && m == 0 {
        return vec![];
    }
    if n == 0 {
        return (0..m).map(RawEdit::Insert).collect();
    }
    if m == 0 {
        return (0..n).map(RawEdit::Remove).collect();
    }

    let max_d = n + m;
    // `v[k + offset]` holds the furthest x-coordinate reached on diagonal k.
    let offset = max_d as i64;
    let mut v: Vec<i64> = vec![0; 2 * max_d + 2];
    // `trace[d]` is a snapshot of `v` taken *before* the d-th forward pass, so it
    // reflects the frontier after d-1 edits.  Used during backtracking.
    let mut trace: Vec<Vec<i64>> = Vec::with_capacity(max_d + 1);

    'outer: for d in 0..=(max_d as i64) {
        trace.push(v.clone());
        let mut k = -d;
        while k <= d {
            let k_idx = (k + offset) as usize;
            // Choose the move that maximises x on diagonal k.
            let x: i64 = if k == -d
                || (k != d && v[(k - 1 + offset) as usize] < v[(k + 1 + offset) as usize])
            {
                // Insert: arrived from diagonal k+1 (y increases, x unchanged).
                v[(k + 1 + offset) as usize]
            } else {
                // Delete: arrived from diagonal k-1 (x increases).
                v[(k - 1 + offset) as usize] + 1
            };
            let mut x = x;
            let mut y = x - k;
            // Extend the snake along matching items.
            while x < n as i64 && y < m as i64 && equals(x as usize, y as usize) {
                x += 1;
                y += 1;
            }
            v[k_idx] = x;
            if x >= n as i64 && y >= m as i64 {
                break 'outer;
            }
            k += 2;
        }
    }

    // Backtrack through the saved trace to reconstruct the edit sequence in
    // reverse, then flip it at the end.
    let mut edits: Vec<RawEdit> = Vec::new();
    let mut x = n as i64;
    let mut y = m as i64;

    for (d, v_snap) in trace.iter().enumerate().rev() {
        if x == 0 && y == 0 {
            break;
        }
        let d = d as i64;
        if d == 0 {
            // Everything remaining is the initial snake (pure keeps from (0,0)).
            while x > 0 {
                x -= 1;
                y -= 1;
                edits.push(RawEdit::Keep(x as usize, y as usize));
            }
            break;
        }

        let k = x - y;

        // Determine whether the d-th edit was an insert (prev_k = k+1) or a delete
        // (prev_k = k-1) by replaying the forward-pass decision with v_snap.
        let prev_k: i64 = if k == -d
            || (k != d && v_snap[(k - 1 + offset) as usize] < v_snap[(k + 1 + offset) as usize])
        {
            k + 1 // insert
        } else {
            k - 1 // delete
        };

        let prev_x = v_snap[(prev_k + offset) as usize];
        let prev_y = prev_x - prev_k;

        // (mid_x, mid_y) is the point immediately after the edit, before the snake.
        let (mid_x, _mid_y) = if prev_k == k + 1 {
            (prev_x, prev_y + 1) // after insert: y advanced by 1
        } else {
            (prev_x + 1, prev_y) // after delete: x advanced by 1
        };

        // Walk back along the snake from (x, y) to (mid_x, mid_y).
        while x > mid_x {
            x -= 1;
            y -= 1;
            edits.push(RawEdit::Keep(x as usize, y as usize));
        }

        // Emit the single edit that preceded the snake.
        if prev_k == k + 1 {
            edits.push(RawEdit::Insert(prev_y as usize));
        } else {
            edits.push(RawEdit::Remove(prev_x as usize));
        }

        x = prev_x;
        y = prev_y;
    }

    edits.reverse();
    edits
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Calls `on_group(start, count)` for each run of consecutive indices.
///
/// Expects `indices` to be in ascending order (guaranteed by Myers output).
fn group_consecutive(indices: &[usize], mut on_group: impl FnMut(usize, usize)) {
    let mut i = 0;
    while i < indices.len() {
        let start = indices[i];
        let mut count = 1;
        while i + count < indices.len() && indices[i + count] == start + count {
            count += 1;
        }
        on_group(start, count);
        i += count;
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- test helpers -------------------------------------------------------

    /// Item with separate identity (`id`) and content (`val`) fields so we can
    /// exercise all four diff operations independently.
    #[derive(Debug, Clone, PartialEq)]
    struct Item {
        id: u32,
        val: u32,
    }

    fn item(id: u32, val: u32) -> Item {
        Item { id, val }
    }

    struct IdComparator;

    impl ListComparator<Item> for IdComparator {
        fn is_same_item(&self, a: &Item, b: &Item) -> bool {
            a.id == b.id
        }
        fn are_content_the_same(&self, a: &Item, b: &Item) -> bool {
            a.val == b.val
        }
    }

    fn ops(old: &[Item], new: &[Item]) -> Vec<DiffOp> {
        // Pass &old so L = &[Item] (Sized), which satisfies the AsRef<[Item]> bound.
        diff(&old, &new, &IdComparator).ops
    }

    // Collect ops of a single kind for assertions that don't care about order.
    fn removes(ops: &[DiffOp]) -> Vec<(usize, usize)> {
        ops.iter()
            .filter_map(|o| {
                if let DiffOp::Remove { index, count } = o {
                    Some((*index, *count))
                } else {
                    None
                }
            })
            .collect()
    }

    fn inserts(ops: &[DiffOp]) -> Vec<(usize, usize)> {
        ops.iter()
            .filter_map(|o| {
                if let DiffOp::Insert { index, count } = o {
                    Some((*index, *count))
                } else {
                    None
                }
            })
            .collect()
    }

    fn moves(ops: &[DiffOp]) -> Vec<(usize, usize, bool)> {
        ops.iter()
            .filter_map(|o| {
                if let DiffOp::Move {
                    old_index,
                    new_index,
                    changed,
                } = o
                {
                    Some((*old_index, *new_index, *changed))
                } else {
                    None
                }
            })
            .collect()
    }

    fn changes(ops: &[DiffOp]) -> Vec<(usize, usize)> {
        ops.iter()
            .filter_map(|o| {
                if let DiffOp::Change {
                    old_index,
                    new_index,
                } = o
                {
                    Some((*old_index, *new_index))
                } else {
                    None
                }
            })
            .collect()
    }

    // --- tests --------------------------------------------------------------

    #[test]
    fn empty_to_empty() {
        let result: Vec<Item> = vec![];
        assert!(ops(&result, &result).is_empty());
    }

    #[test]
    fn identical_lists_produce_no_ops() {
        let list = vec![item(1, 10), item(2, 20), item(3, 30)];
        assert!(ops(&list, &list).is_empty());
    }

    #[test]
    fn insert_into_empty() {
        let new = vec![item(1, 1), item(2, 2), item(3, 3)];
        let result = ops(&[], &new);
        assert_eq!(inserts(&result), [(0, 3)]);
        assert!(removes(&result).is_empty());
    }

    #[test]
    fn remove_all() {
        let old = vec![item(1, 1), item(2, 2), item(3, 3)];
        let result = ops(&old, &[]);
        assert_eq!(removes(&result), [(0, 3)]);
        assert!(inserts(&result).is_empty());
    }

    #[test]
    fn append_items() {
        let old = vec![item(1, 1)];
        let new = vec![item(1, 1), item(2, 2), item(3, 3)];
        let result = ops(&old, &new);
        assert!(removes(&result).is_empty());
        assert_eq!(inserts(&result), [(1, 2)]);
    }

    #[test]
    fn prepend_items() {
        let old = vec![item(3, 3)];
        let new = vec![item(1, 1), item(2, 2), item(3, 3)];
        let result = ops(&old, &new);
        assert!(removes(&result).is_empty());
        assert_eq!(inserts(&result), [(0, 2)]);
    }

    #[test]
    fn remove_from_middle() {
        let old = vec![item(1, 1), item(2, 2), item(3, 3)];
        let new = vec![item(1, 1), item(3, 3)];
        let result = ops(&old, &new);
        assert_eq!(removes(&result), [(1, 1)]);
        assert!(inserts(&result).is_empty());
    }

    #[test]
    fn insert_in_middle() {
        let old = vec![item(1, 1), item(3, 3)];
        let new = vec![item(1, 1), item(2, 2), item(3, 3)];
        let result = ops(&old, &new);
        assert!(removes(&result).is_empty());
        assert_eq!(inserts(&result), [(1, 1)]);
    }

    #[test]
    fn replace_no_identity_match() {
        // Old and new share no ids, so everything is remove + insert (no moves).
        let old = vec![item(1, 1), item(2, 2)];
        let new = vec![item(3, 3), item(4, 4)];
        let result = ops(&old, &new);
        assert_eq!(removes(&result), [(0, 2)]);
        assert_eq!(inserts(&result), [(0, 2)]);
        assert!(moves(&result).is_empty());
    }

    #[test]
    fn content_change_same_position() {
        let old = vec![item(1, 10), item(2, 20)];
        let new = vec![item(1, 10), item(2, 99)]; // id=2 content changed
        let result = ops(&old, &new);
        assert!(removes(&result).is_empty());
        assert!(inserts(&result).is_empty());
        assert!(moves(&result).is_empty());
        assert_eq!(changes(&result), [(1, 1)]);
    }

    #[test]
    fn move_without_content_change() {
        // Item id=2 moves from index 1 to index 0.
        let old = vec![item(1, 1), item(2, 2)];
        let new = vec![item(2, 2), item(1, 1)];
        let result = ops(&old, &new);
        // One of the two items must be detected as a move; the other is kept.
        // (Exactly which one depends on which side Myers happens to keep.)
        let m = moves(&result);
        assert_eq!(m.len(), 1);
        assert!(!m[0].2, "content did not change");
    }

    #[test]
    fn move_with_content_change() {
        let old = vec![item(1, 1), item(2, 2), item(3, 3)];
        // id=3 moves to front and its value changes.
        let new = vec![item(3, 99), item(1, 1), item(2, 2)];
        let result = ops(&old, &new);
        let m = moves(&result);
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].0, 2); // old_index of id=3
        assert_eq!(m[0].1, 0); // new_index of id=3
        assert!(m[0].2, "content changed");
    }

    #[test]
    fn interleaved_inserts_and_removes() {
        // [A, B, C, D] → [B, D, E]  (remove A and C, insert E at end)
        let old = vec![item(1, 1), item(2, 2), item(3, 3), item(4, 4)];
        let new = vec![item(2, 2), item(4, 4), item(5, 5)];
        let result = ops(&old, &new);
        // Removes: indices 0 (A) and 2 (C) — not consecutive so two ops.
        let r = removes(&result);
        assert!(r.contains(&(0, 1)));
        assert!(r.contains(&(2, 1)));
        // Insert: index 2 in new list (E).
        assert_eq!(inserts(&result), [(2, 1)]);
    }

    #[test]
    fn consecutive_removes_are_batched() {
        // Remove B and C together.
        let old = vec![item(1, 1), item(2, 2), item(3, 3), item(4, 4)];
        let new = vec![item(1, 1), item(4, 4)];
        let result = ops(&old, &new);
        assert_eq!(removes(&result), [(1, 2)]);
    }

    #[test]
    fn consecutive_inserts_are_batched() {
        let old = vec![item(1, 1), item(4, 4)];
        let new = vec![item(1, 1), item(2, 2), item(3, 3), item(4, 4)];
        let result = ops(&old, &new);
        assert_eq!(inserts(&result), [(1, 2)]);
    }

    #[test]
    fn myers_edits_simple() {
        // Internal unit test: [A, B] → [B, C] should yield Remove(0), Keep, Insert.
        let edits = myers_edits(2, 2, |x, y| x == 1 && y == 0); // old[1]==new[0] (B==B)
        let removes: Vec<_> = edits
            .iter()
            .filter(|e| matches!(e, RawEdit::Remove(_)))
            .collect();
        let inserts: Vec<_> = edits
            .iter()
            .filter(|e| matches!(e, RawEdit::Insert(_)))
            .collect();
        let keeps: Vec<_> = edits
            .iter()
            .filter(|e| matches!(e, RawEdit::Keep(_, _)))
            .collect();
        assert_eq!(removes.len(), 1);
        assert_eq!(inserts.len(), 1);
        assert_eq!(keeps.len(), 1);
    }
}
