//! What changed in a saved collection that is kept as one aux entry per member.

use std::collections::BTreeSet;

/// The members of a per-entry saved collection that changed since the state was
/// last stored, so the store writes and deletes only those entries.
///
/// Not part of the saved record. A state read back from a record that has its
/// entries on disk starts with nothing pending; one built from scratch, or read
/// from a record that carries the collection itself, starts with everything
/// pending.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntryChanges<K: Ord> {
    /// Members inserted or changed: their entries are rewritten.
    pub upserted: BTreeSet<K>,
    /// Members removed: their entries are deleted.
    pub removed: BTreeSet<K>,
    /// Every member is written and every entry not backing a member is deleted,
    /// because the collection was changed through a path that cannot say which
    /// members it touched.
    pub rewrite_all: bool,
}

impl<K: Ord> Default for EntryChanges<K> {
    fn default() -> Self {
        Self {
            upserted: BTreeSet::new(),
            removed: BTreeSet::new(),
            rewrite_all: false,
        }
    }
}

impl<K: Ord> EntryChanges<K> {
    /// Everything pending: the collection has never been stored as entries, or
    /// what is on disk cannot be trusted to match.
    pub fn all() -> Self {
        Self {
            rewrite_all: true,
            ..Self::default()
        }
    }

    /// `key` was inserted or changed.
    pub fn upsert(&mut self, key: K) {
        self.removed.remove(&key);
        self.upserted.insert(key);
    }

    /// `key` was removed.
    pub fn remove(&mut self, key: K) {
        self.upserted.remove(&key);
        self.removed.insert(key);
    }

    /// The collection changed in a way that is not tracked member by member.
    pub fn mark_rewrite_all(&mut self) {
        self.rewrite_all = true;
    }

    /// The store wrote everything pending.
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Nothing to write or delete.
    pub fn is_empty(&self) -> bool {
        !self.rewrite_all && self.upserted.is_empty() && self.removed.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_removal_after_an_upsert_leaves_only_the_removal() {
        let mut changes = EntryChanges::default();
        changes.upsert(1u8);
        changes.remove(1u8);
        assert!(changes.upserted.is_empty());
        assert_eq!(changes.removed, BTreeSet::from([1u8]));
    }

    #[test]
    fn an_upsert_after_a_removal_leaves_only_the_upsert() {
        let mut changes = EntryChanges::default();
        changes.remove(1u8);
        changes.upsert(1u8);
        assert!(changes.removed.is_empty());
        assert_eq!(changes.upserted, BTreeSet::from([1u8]));
    }

    #[test]
    fn clear_empties_everything_including_the_rewrite_flag() {
        let mut changes = EntryChanges::<u8>::all();
        changes.upsert(1);
        assert!(!changes.is_empty());
        changes.clear();
        assert!(changes.is_empty());
        assert_eq!(changes, EntryChanges::default());
    }
}
