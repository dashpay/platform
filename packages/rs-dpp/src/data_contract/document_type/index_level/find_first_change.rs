//! Pairwise diff helpers for [`IndexLevel`] trees, used by
//! `validate_update` to surface the first index-path where a
//! count-, sum- or ranking-affecting flag differs between an old and
//! a new contract version.
//!
//! All of them drive grovedb tree-variant choice at contract
//! creation, so toggling any after creation would require rebuilding
//! the index tree and is rejected. The error messages include from→to
//! values so contract authors can see *what* changed at the rejected
//! path, not just *that* something did.

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

    /// Ranking counterpart of [`Self::find_first_countability_change`] and
    /// [`Self::find_first_summability_change`]. Recursively finds the first
    /// index path where any of the three ranking axes (`ranked_countable`,
    /// `ranked_summable`, `ranked_averageable`) differs between two
    /// `IndexLevel` trees.
    ///
    /// All three live in one helper because the Avg axis is neither a
    /// count-only nor a sum-only property, and because they are decided
    /// together: the set of declared axes picks the indexed tree variant
    /// (`ProvableCountIndexedTree` / `ProvableSumIndexedTree` /
    /// `ProvableCountProvableSumIndexedTree(axes)`) and its ordered secondaries
    /// at contract creation. Toggling any one of them would require rebuilding
    /// the secondaries for every existing group, so all three are immutable —
    /// same reasoning, and same error, as the count and sum flags.
    ///
    /// Returns `None` if all three properties are the same everywhere.
    #[cfg(feature = "validation")]
    pub(super) fn find_first_ranked_change(&self, new: &IndexLevel) -> Option<String> {
        // The prefix-level ranking markers live on the LEVEL, not on the
        // terminating info — an index's `rankedCountable: { at }` stamps a
        // non-terminal level, so moving or toggling it changes these two
        // flags while every `IndexLevelTypeInfo` stays identical. Same
        // rebuild-the-secondaries reasoning as the axis flags below.
        if self.ranked_count_grouping != new.ranked_count_grouping {
            return Some(format!(
                "(ranked_count_grouping: {} -> {})",
                self.ranked_count_grouping, new.ranked_count_grouping,
            ));
        }
        if self.count_propagating != new.count_propagating {
            return Some(format!(
                "(count_propagating: {} -> {})",
                self.count_propagating, new.count_propagating,
            ));
        }
        // The exempt-branch marker decides whether the level's
        // property-name tree is created `Element::NonCounted`-wrapped
        // inside the chain's value trees or inserted contributing —
        // frozen layout like the two chain stamps above. Every flip is
        // already accompanied by a chain-stamp or countability change
        // (the marker is derived from them), but the layout flag itself
        // is what the walkers read, so it gets its own first-class check.
        if self.count_exempt_branch() != new.count_exempt_branch() {
            return Some(format!(
                "(count_exempt_branch: {} -> {})",
                self.count_exempt_branch(),
                new.count_exempt_branch(),
            ));
        }
        if let (Some(old_info), Some(new_info)) =
            (&self.has_index_with_type, &new.has_index_with_type)
        {
            if old_info.ranked_countable != new_info.ranked_countable {
                return Some(format!(
                    "(ranked_countable: {} -> {})",
                    old_info.ranked_countable, new_info.ranked_countable,
                ));
            }
            if old_info.ranked_summable != new_info.ranked_summable {
                return Some(format!(
                    "(ranked_summable: {} -> {})",
                    old_info.ranked_summable, new_info.ranked_summable,
                ));
            }
            if old_info.ranked_averageable != new_info.ranked_averageable {
                return Some(format!(
                    "(ranked_averageable: {} -> {})",
                    old_info.ranked_averageable, new_info.ranked_averageable,
                ));
            }
        }

        for (key, old_sub) in &self.sub_index_levels {
            if let Some(new_sub) = new.sub_index_levels.get(key) {
                if let Some(inner_path) = old_sub.find_first_ranked_change(new_sub) {
                    return Some(format!("{} -> {}", key, inner_path));
                }
            }
        }

        None
    }

    /// Time-range counterpart of [`Self::find_first_countability_change`].
    /// Recursively finds the first index path where the `time_range`
    /// transform differs between two `IndexLevel` trees. The transform
    /// dictates how many index entries each document produces and under
    /// which bucket keys, so changing it after creation would leave already
    /// stored documents indexed under stale buckets — it is immutable.
    /// That includes `ttl`, which the storage key leaves out: entries
    /// written before a TTL carry storage flags, and an ephemeral level
    /// must stay flagless.
    ///
    /// Returns `None` if the transform is the same everywhere.
    #[cfg(feature = "validation")]
    pub(super) fn find_first_time_range_change(&self, new: &IndexLevel) -> Option<String> {
        if self.time_range() != new.time_range() {
            let fmt = |t: Option<&super::TimeRangeTransform>| match t {
                Some(t) => format!(
                    "Some(on: {:?}, range: {}s, step: {}s, phase: {}s, ttl: {})",
                    t.source,
                    t.range_seconds,
                    t.step_seconds,
                    t.phase_seconds,
                    t.ttl_seconds
                        .map(|ttl| format!("{}s", ttl))
                        .unwrap_or_else(|| "None".to_string()),
                ),
                None => "None".to_string(),
            };
            return Some(format!(
                "(timeRange: {} -> {})",
                fmt(self.time_range()),
                fmt(new.time_range()),
            ));
        }

        for (key, old_sub) in &self.sub_index_levels {
            if let Some(new_sub) = new.sub_index_levels.get(key) {
                if let Some(inner_path) = old_sub.find_first_time_range_change(new_sub) {
                    return Some(format!("{} -> {}", key, inner_path));
                }
            }
        }

        None
    }

    /// Preallocation counterpart of [`Self::find_first_countability_change`].
    /// Recursively finds the first index path where the `preallocated` flag
    /// differs between two `IndexLevel` trees. The flag decides who creates
    /// (and who is allowed to prune) the index's dynamic trees: turning it on
    /// would leave every existing referenced document without preallocated
    /// trees while its delete walker already refuses to prune, and turning it
    /// off would let last-entry deletes prune trees a referenced document's
    /// creator paid for as permanent structure — so it is immutable.
    ///
    /// Returns `None` if the flag is the same everywhere.
    #[cfg(feature = "validation")]
    pub(super) fn find_first_preallocated_change(&self, new: &IndexLevel) -> Option<String> {
        if let (Some(old_info), Some(new_info)) =
            (&self.has_index_with_type, &new.has_index_with_type)
        {
            if old_info.preallocated != new_info.preallocated {
                return Some(format!(
                    "(preallocated: {} -> {})",
                    old_info.preallocated, new_info.preallocated,
                ));
            }
        }

        for (key, old_sub) in &self.sub_index_levels {
            if let Some(new_sub) = new.sub_index_levels.get(key) {
                if let Some(inner_path) = old_sub.find_first_preallocated_change(new_sub) {
                    return Some(format!("{} -> {}", key, inner_path));
                }
            }
        }

        None
    }
}
