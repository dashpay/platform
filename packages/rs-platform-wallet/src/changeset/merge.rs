//! The `Merge` trait for composing changeset deltas.
//!
//! Changesets are commutative and associative so that multiple deltas can be
//! batched and reordered without affecting the final result.

use std::collections::{BTreeMap, BTreeSet};

/// Combine two changesets.  Changesets are commutative and associative
/// for safe batching and reordering.
pub trait Merge: Default {
    /// Merge another changeset into `self`.
    fn merge(&mut self, other: Self);

    /// Returns `true` if this changeset contains no changes.
    fn is_empty(&self) -> bool;

    /// Take the changeset if non-empty, leaving `Default` in place.
    fn take(&mut self) -> Option<Self>
    where
        Self: Sized,
    {
        if self.is_empty() {
            None
        } else {
            Some(std::mem::take(self))
        }
    }
}

// ---------------------------------------------------------------------------
// Blanket / stdlib impls
// ---------------------------------------------------------------------------

/// `BTreeMap<K, V>` where `V: Merge` — recursive merge of values sharing a
/// key, plain insert for new keys.
impl<K: Ord, V: Merge + Clone> Merge for BTreeMap<K, V> {
    fn merge(&mut self, other: Self) {
        for (key, value) in other {
            self.entry(key)
                .and_modify(|existing| existing.merge(value.clone()))
                .or_insert(value);
        }
    }

    fn is_empty(&self) -> bool {
        self.is_empty()
    }
}

/// `BTreeSet<T>` — union (add-only).
impl<T: Ord + Clone> Merge for BTreeSet<T> {
    fn merge(&mut self, other: Self) {
        self.extend(other);
    }

    fn is_empty(&self) -> bool {
        self.is_empty()
    }
}

/// `Option<T>` where `T: Merge` — merge inner values or take `other`.
impl<T: Merge> Merge for Option<T> {
    fn merge(&mut self, other: Self) {
        match (self.as_mut(), other) {
            (Some(existing), Some(incoming)) => existing.merge(incoming),
            (None, incoming @ Some(_)) => *self = incoming,
            _ => {}
        }
    }

    fn is_empty(&self) -> bool {
        match self {
            Some(inner) => inner.is_empty(),
            None => true,
        }
    }
}

/// `Vec<T>` — extend (append-only).
impl<T: Clone> Merge for Vec<T> {
    fn merge(&mut self, other: Self) {
        self.extend(other);
    }

    fn is_empty(&self) -> bool {
        self.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_btreeset_merge_is_union() {
        let mut a: BTreeSet<u32> = [1, 2, 3].into_iter().collect();
        let b: BTreeSet<u32> = [3, 4, 5].into_iter().collect();
        a.merge(b);
        assert_eq!(a, [1, 2, 3, 4, 5].into_iter().collect());
    }

    #[test]
    fn test_option_merge_both_some() {
        let mut a: Option<Vec<u32>> = Some(vec![1, 2]);
        let b: Option<Vec<u32>> = Some(vec![3, 4]);
        a.merge(b);
        assert_eq!(a, Some(vec![1, 2, 3, 4]));
    }

    #[test]
    fn test_option_merge_none_plus_some() {
        let mut a: Option<Vec<u32>> = None;
        let b: Option<Vec<u32>> = Some(vec![3, 4]);
        a.merge(b);
        assert_eq!(a, Some(vec![3, 4]));
    }

    #[test]
    fn test_option_merge_some_plus_none() {
        let mut a: Option<Vec<u32>> = Some(vec![1, 2]);
        let b: Option<Vec<u32>> = None;
        a.merge(b);
        assert_eq!(a, Some(vec![1, 2]));
    }

    #[test]
    fn test_vec_merge_extends() {
        let mut a = vec![1, 2];
        let b = vec![3, 4];
        a.merge(b);
        assert_eq!(a, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_take_empty() {
        let mut v: Vec<u32> = Vec::new();
        assert!(v.take().is_none());
    }

    #[test]
    fn test_take_non_empty() {
        let mut v = vec![1, 2, 3];
        let taken = v.take();
        assert_eq!(taken, Some(vec![1, 2, 3]));
        assert!(v.is_empty());
    }

    #[test]
    fn test_btreemap_merge_recursive() {
        // Values are Vec<u32> which impl Merge via extend
        let mut a: BTreeMap<&str, Vec<u32>> = BTreeMap::new();
        a.insert("x", vec![1]);
        let mut b: BTreeMap<&str, Vec<u32>> = BTreeMap::new();
        b.insert("x", vec![2]);
        b.insert("y", vec![3]);
        a.merge(b);
        assert_eq!(a.get("x"), Some(&vec![1, 2]));
        assert_eq!(a.get("y"), Some(&vec![3]));
    }
}
