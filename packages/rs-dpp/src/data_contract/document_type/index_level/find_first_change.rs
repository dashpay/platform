//! Pairwise diff helpers for [`IndexLevel`] trees, used by
//! `validate_update` to surface the first index-path where a
//! count- or sum-affecting flag differs between an old and a new
//! contract version.
//!
//! Both flags (count and sum) drive grovedb tree-variant choice at
//! contract creation, so toggling either after creation would
//! require rebuilding the index tree and is rejected. The error
//! messages include from→to values so contract authors can see
//! *what* changed at the rejected path, not just *that* something
//! did.

use super::IndexLevel;

impl IndexLevel {
    /// Recursively finds the first index path where a count-affecting
    /// property (`countable` or `range_countable`) differs between
    /// two `IndexLevel` trees.
    ///
    /// Both flags drive grovedb tree-variant choice at contract
    /// creation (`NormalTree` / `CountTree` / `ProvableCountTree` at
    /// the `[0]` terminal, and additionally `NonCounted`-wrapped
    /// continuations + `ProvableCountTree` property-name level for
    /// `range_countable`), so toggling either after creation would
    /// require rebuilding the index tree and is rejected.
    /// Returns `None` if both properties are the same everywhere.
    #[cfg(feature = "validation")]
    pub(super) fn find_first_countability_change(&self, new: &IndexLevel) -> Option<String> {
        if let (Some(old_info), Some(new_info)) =
            (&self.has_index_with_type, &new.has_index_with_type)
        {
            if old_info.countable != new_info.countable {
                // Include both ends so the contract author can see
                // which countability tier shifted (e.g. NotCountable
                // → Countable vs Countable → CountableAllowingOffset)
                // rather than just learning *that* something changed.
                return Some(format!(
                    "(countable: {:?} -> {:?})",
                    old_info.countable, new_info.countable,
                ));
            }
            if old_info.range_countable != new_info.range_countable {
                return Some(format!(
                    "(range_countable: {} -> {})",
                    old_info.range_countable, new_info.range_countable,
                ));
            }
        }

        // Recurse into sub-levels that exist in both old and new
        for (key, old_sub) in &self.sub_index_levels {
            if let Some(new_sub) = new.sub_index_levels.get(key) {
                if let Some(inner_path) = old_sub.find_first_countability_change(new_sub) {
                    return Some(format!("{} -> {}", key, inner_path));
                }
            }
        }

        None
    }

    /// Sum-tree counterpart of [`Self::find_first_countability_change`].
    /// Recursively finds the first index path where a sum-affecting
    /// property (`summable` property-name or `range_summable`) differs
    /// between two `IndexLevel` trees. Both flags drive grovedb
    /// tree-variant choice at contract creation (`NormalTree` /
    /// `SumTree` / `ProvableSumTree`, and the reference variant
    /// under each level), so toggling either after creation would
    /// require rebuilding the index tree and is rejected.
    ///
    /// Returns `None` if both properties are the same everywhere.
    #[cfg(feature = "validation")]
    pub(super) fn find_first_summability_change(&self, new: &IndexLevel) -> Option<String> {
        if let (Some(old_info), Some(new_info)) =
            (&self.has_index_with_type, &new.has_index_with_type)
        {
            if old_info.summable != new_info.summable {
                // Include the from→to summable property names so the
                // author can see whether the change was an opt-in
                // (None → Some("fee")), an opt-out, or a swap to a
                // different property (Some("fee") → Some("amount")).
                let fmt = |s: &Option<String>| {
                    s.as_deref()
                        .map(|p| format!("Some({:?})", p))
                        .unwrap_or_else(|| "None".to_string())
                };
                return Some(format!(
                    "(summable: {} -> {})",
                    fmt(&old_info.summable),
                    fmt(&new_info.summable),
                ));
            }
            if old_info.range_summable != new_info.range_summable {
                return Some(format!(
                    "(range_summable: {} -> {})",
                    old_info.range_summable, new_info.range_summable,
                ));
            }
        }

        for (key, old_sub) in &self.sub_index_levels {
            if let Some(new_sub) = new.sub_index_levels.get(key) {
                if let Some(inner_path) = old_sub.find_first_summability_change(new_sub) {
                    return Some(format!("{} -> {}", key, inner_path));
                }
            }
        }

        None
    }
}
