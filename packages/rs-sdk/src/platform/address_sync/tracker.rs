//! Key-leaf tracking for iterative address synchronization.
//!
//! This module provides the [`KeyLeafTracker`] which tracks which target keys
//! need branch queries and which leaf subtrees they're located in.

use super::types::{AddressKey, LeafBoundaryKey};
use drive::grovedb::LeafInfo;
use std::collections::BTreeMap;

/// Tracks which target keys need branch queries and which leaf subtrees they're in.
///
/// During iterative synchronization:
/// 1. After trunk query: keys not found are traced to leaf keys (subtree roots)
/// 2. After each branch query: keys are either found, proven absent, or traced to deeper leaves
///
/// This tracker maintains the state needed to efficiently batch and execute branch queries.
#[derive(Debug)]
pub struct KeyLeafTracker {
    /// Map: target address key -> leaf boundary key (which subtree contains this target)
    key_to_leaf: BTreeMap<AddressKey, LeafBoundaryKey>,

    /// Refcount: leaf boundary key -> number of pending targets in this subtree
    leaf_refcount: BTreeMap<LeafBoundaryKey, usize>,

    /// LeafInfo for each leaf boundary key (hash and count for privacy calculations)
    leaf_info: BTreeMap<LeafBoundaryKey, LeafInfo>,
}

impl Default for KeyLeafTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyLeafTracker {
    /// Create a new empty tracker.
    pub fn new() -> Self {
        Self {
            key_to_leaf: BTreeMap::new(),
            leaf_refcount: BTreeMap::new(),
            leaf_info: BTreeMap::new(),
        }
    }

    /// Add a target key with its associated leaf boundary key.
    ///
    /// # Arguments
    /// - `target`: The address key we're searching for
    /// - `leaf`: The leaf boundary key whose subtree contains the target
    /// - `info`: LeafInfo with hash and optional count
    pub fn add_key(&mut self, target: AddressKey, leaf: LeafBoundaryKey, info: LeafInfo) {
        *self.leaf_refcount.entry(leaf.clone()).or_insert(0) += 1;
        self.key_to_leaf.insert(target, leaf.clone());
        self.leaf_info.insert(leaf, info);
    }

    /// Mark a key as found - removes it from tracking.
    ///
    /// # Arguments
    /// - `key`: The target key that was found
    pub fn key_found(&mut self, key: &[u8]) {
        if let Some(leaf) = self.key_to_leaf.remove(key) {
            if let Some(count) = self.leaf_refcount.get_mut(&leaf) {
                *count = count.saturating_sub(1);
            }
        }
    }

    /// Update a key's leaf to a deeper one.
    ///
    /// Called when a branch query reveals the key is in an even deeper subtree.
    ///
    /// # Arguments
    /// - `key`: The target address key being updated
    /// - `new_leaf`: The new (deeper) leaf boundary key
    /// - `info`: LeafInfo for the new leaf
    pub fn update_leaf(&mut self, key: &[u8], new_leaf: LeafBoundaryKey, info: LeafInfo) {
        if let Some(old_leaf) = self.key_to_leaf.get(key).cloned() {
            // Decrement old leaf's refcount
            if let Some(count) = self.leaf_refcount.get_mut(&old_leaf) {
                *count = count.saturating_sub(1);
            }
            // Add to new leaf
            *self.leaf_refcount.entry(new_leaf.clone()).or_insert(0) += 1;
            self.key_to_leaf.insert(key.to_vec(), new_leaf.clone());
            self.leaf_info.insert(new_leaf, info);
        }
    }

    /// Get LeafInfo for a specific leaf boundary key.
    #[allow(dead_code)]
    pub fn get_leaf_info(&self, leaf_key: &[u8]) -> Option<LeafInfo> {
        self.leaf_info.get(leaf_key).copied()
    }

    /// Get leaf boundary keys with refcount > 0 (still have pending targets).
    ///
    /// Returns tuples of (leaf_boundary_key, leaf_info) for leaves that still have
    /// unresolved target keys.
    pub fn active_leaves(&self) -> Vec<(LeafBoundaryKey, LeafInfo)> {
        self.leaf_refcount
            .iter()
            .filter(|(_, &count)| count > 0)
            .filter_map(|(k, _)| self.leaf_info.get(k).map(|info| (k.clone(), *info)))
            .collect()
    }

    /// Get all target address keys associated with a specific leaf boundary key.
    pub fn keys_for_leaf(&self, leaf: &[u8]) -> Vec<AddressKey> {
        self.key_to_leaf
            .iter()
            .filter(|(_, l)| l.as_slice() == leaf)
            .map(|(k, _)| k.clone())
            .collect()
    }

    /// Get the leaf boundary key for a specific target address key.
    #[allow(dead_code)]
    pub fn get_leaf_for_key(&self, key: &[u8]) -> Option<&LeafBoundaryKey> {
        self.key_to_leaf.get(key)
    }

    /// Get the number of remaining (unresolved) target keys.
    pub fn remaining_count(&self) -> usize {
        self.key_to_leaf.len()
    }

    /// Check if all target keys have been resolved.
    pub fn is_empty(&self) -> bool {
        self.key_to_leaf.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_leaf_info(hash: [u8; 32], count: Option<u64>) -> LeafInfo {
        LeafInfo { hash, count }
    }

    #[test]
    fn test_add_and_find_key() {
        let mut tracker = KeyLeafTracker::new();

        let target = vec![1, 2, 3];
        let leaf = vec![10, 20, 30];
        let info = make_leaf_info([0u8; 32], Some(100));

        tracker.add_key(target.clone(), leaf.clone(), info);

        assert_eq!(tracker.remaining_count(), 1);
        assert!(!tracker.is_empty());

        let active = tracker.active_leaves();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].0, leaf);

        tracker.key_found(&target);
        assert_eq!(tracker.remaining_count(), 0);
        assert!(tracker.is_empty());
    }

    #[test]
    fn test_multiple_keys_same_leaf() {
        let mut tracker = KeyLeafTracker::new();

        let leaf = vec![10, 20, 30];
        let info = make_leaf_info([0u8; 32], Some(100));

        tracker.add_key(vec![1], leaf.clone(), info);
        tracker.add_key(vec![2], leaf.clone(), info);
        tracker.add_key(vec![3], leaf.clone(), info);

        assert_eq!(tracker.remaining_count(), 3);
        assert_eq!(tracker.keys_for_leaf(&leaf).len(), 3);

        // Finding one key should not remove the leaf from active
        tracker.key_found(&[1]);
        assert_eq!(tracker.remaining_count(), 2);
        assert_eq!(tracker.active_leaves().len(), 1);

        tracker.key_found(&[2]);
        tracker.key_found(&[3]);
        assert!(tracker.is_empty());
        assert!(tracker.active_leaves().is_empty());
    }

    #[test]
    fn test_update_leaf() {
        let mut tracker = KeyLeafTracker::new();

        let target = vec![1, 2, 3];
        let leaf1 = vec![10];
        let leaf2 = vec![20];
        let info1 = make_leaf_info([1u8; 32], Some(100));
        let info2 = make_leaf_info([2u8; 32], Some(50));

        tracker.add_key(target.clone(), leaf1.clone(), info1);
        assert_eq!(tracker.active_leaves().len(), 1);
        assert_eq!(tracker.active_leaves()[0].0, leaf1);

        // Update to deeper leaf
        tracker.update_leaf(&target, leaf2.clone(), info2);

        // Old leaf should have 0 refcount, new leaf should have 1
        assert_eq!(tracker.active_leaves().len(), 1);
        assert_eq!(tracker.active_leaves()[0].0, leaf2);
        assert_eq!(tracker.get_leaf_for_key(&target), Some(&leaf2));
    }
}
