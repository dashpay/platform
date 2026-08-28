#[cfg(feature = "validation")]
mod find_first_change;

#[cfg(feature = "validation")]
use crate::consensus::basic::data_contract::DataContractInvalidIndexDefinitionUpdateError;
use crate::consensus::basic::data_contract::DuplicateIndexError;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::data_contract::document_type::index::IndexCountability;
use crate::data_contract::document_type::index::TimeRangeTransform;
use crate::data_contract::document_type::index_level::IndexType::{
    ContestedResourceIndex, NonUniqueIndex, UniqueIndex,
};
use crate::data_contract::document_type::Index;
#[cfg(feature = "validation")]
use crate::validation::SimpleConsensusValidationResult;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use std::borrow::Borrow;
use std::collections::BTreeMap;

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum IndexType {
    /// A normal non unique index
    NonUniqueIndex,
    /// A unique index, that means that the values for this index are unique
    /// As long as one of the values is not nil
    UniqueIndex,
    /// A contested resource: This is a unique index but that can be contested through a resolution
    /// The simplest to understand resolution is a masternode votes, but could also be something
    /// like a bidding war.
    /// For example the path/name in the dpns contract must be unique but it is a contested potentially
    /// valuable resource.
    ContestedResourceIndex,
}

#[derive(Debug, PartialEq, Clone)]
pub struct IndexLevelTypeInfo {
    /// should we insert if all fields up to here are null
    pub should_insert_with_all_null: bool,
    /// The index type
    pub index_type: IndexType,
    /// Whether and how this index supports count fast paths. Drives the GroveDB
    /// tree variant chosen at the terminal level of the index path:
    /// `NotCountable` → `NormalTree`,
    /// `Countable` → `CountTree`,
    /// `CountableAllowingOffset` → `ProvableCountTree`.
    pub countable: IndexCountability,
    /// Whether this index supports range-count queries. When true:
    /// - The property-name level (the level *above* this terminating
    ///   level, whose keys are the property's distinct values) is laid out
    ///   as a `ProvableCountTree`.
    /// - Each value tree under it is laid out as a `CountTree`.
    /// - Sibling continuations inside each value tree get wrapped with
    ///   `Element::NonCounted` so their counts don't leak into the value
    ///   tree's count.
    ///
    /// Mutually compatible with the `countable` flag — additive, not a
    /// replacement.
    pub range_countable: bool,
    /// When `Some(property_name)`, the terminal value-tree at this index
    /// path is a `SumTree` (or `CountSumTree` if `countable.is_countable()`
    /// and `range_summable` is false), and references stored under it
    /// carry `ItemWithSumItem` contributions that propagate to the parent
    /// tree's running sum. Mirrors `countable` for the sum surface.
    ///
    /// The named property must be `type: integer` and listed in the
    /// document type's `required` array — enforced by the doctype
    /// validator at contract creation.
    pub summable: Option<String>,
    /// Whether this index supports range-sum queries on its terminator
    /// property. When `true`:
    /// - The property-name level is laid out as a `ProvableSumTree`.
    /// - Each value tree under it is laid out as a `SumTree`.
    /// - Sibling continuations inside each value tree get wrapped with
    ///   `Element::NonCountedItemWithSumItem` so their sums don't pollute
    ///   the value tree's running sum.
    ///
    /// Composes orthogonally with `range_countable` — both flags
    /// together promote the tree to a `ProvableCountSumTree`. Requires
    /// `summable.is_some()`.
    pub range_summable: bool,
    /// Whether this index ranks its groups by **count**. When `true`, the
    /// property-name level (the level *above* this terminating level, whose
    /// keys are the terminator property's distinct values — one child tree per
    /// group) is upgraded from `ProvableCountTree` / `ProvableCountSumTree` to
    /// the matching *indexed* tree carrying an ordered secondary on the Count
    /// axis, so "top / bottom K groups by count" is O(log n + k) with a proof.
    ///
    /// The indexed primary mirrors the tree it replaces byte for byte, so the
    /// existing range-count reads are unaffected. Requires `range_countable`.
    pub ranked_countable: bool,
    /// Sum-axis counterpart of `ranked_countable`: the same property-name level
    /// gains an ordered secondary keyed by each group's running sum. Requires
    /// `range_summable`.
    pub ranked_summable: bool,
    /// Average-axis counterpart of `ranked_countable`: the same property-name
    /// level gains an ordered secondary keyed by each group's (count, sum)
    /// average. Requires both `range_countable` and `range_summable`.
    ///
    /// The three ranking axes are independent — this one does not imply the
    /// other two. The set of axes declared here is what the rs-drive write path
    /// turns into the indexed tree's axis list.
    pub ranked_averageable: bool,
    /// On an indexOnly document type, the property whose value is this
    /// index's member key: the terminal key under the `0` storage marker,
    /// where a normal index stores the document id — stored as an `Item`
    /// instead of a `Reference` because there is no primary-storage row.
    /// Always `Some` here when the declaring type is indexOnly (the parser
    /// normalizes an omitted terminal to `$ownerId`), always `None`
    /// otherwise. Carried on the level info because index levels merge
    /// across indexes sharing prefixes, and the write path only sees the
    /// level at the terminal — but two indexes can never share a full
    /// property list (duplicates are rejected), so each terminating level
    /// belongs to exactly one index and the field is unambiguous.
    pub terminal: Option<String>,
    /// Whether the terminating index is `preallocated` (see
    /// [`crate::data_contract::document_type::index::PREALLOCATED`]): its
    /// dynamic trees are created when a refersTo-referenced document is, and
    /// the delete walker must NOT prune them upward when the last member
    /// entry goes — removing the entry is the whole delete. Carried on the
    /// level info for the same reason `terminal` is: the delete walker only
    /// sees the terminating level, which belongs to exactly one index.
    /// `false` on every pre-PV14 contract (the grammar rejects the keyword
    /// below meta-schema v3).
    pub preallocated: bool,
}

impl IndexType {
    pub fn is_unique(&self) -> bool {
        match self {
            NonUniqueIndex => false,
            UniqueIndex => true,
            ContestedResourceIndex => true,
        }
    }
}

pub type ShouldInsertWithAllNull = bool;

#[derive(Debug, PartialEq, Clone)]
pub struct IndexLevel {
    /// the lower index levels from this level
    sub_index_levels: BTreeMap<String, IndexLevel>,
    /// did an index terminate at this level
    has_index_with_type: Option<IndexLevelTypeInfo>,
    /// When set, the property reached at this level is a timestamp that is
    /// bucketed into time ranges (see [`TimeRangeTransform`]). Only ever set
    /// on a *first-property* node (a direct child of the root), because a
    /// time-range transform must be its index's leading property. At
    /// insert/delete/update time the document's timestamp for this property
    /// is expanded into one key per overlapping range bucket instead of a
    /// single key. Immutable after contract creation.
    time_range: Option<TimeRangeTransform>,
    /// unique level identifier
    level_identifier: u64,
}

impl IndexLevel {
    pub fn identifier(&self) -> u64 {
        self.level_identifier
    }

    pub fn sub_levels(&self) -> &BTreeMap<String, IndexLevel> {
        &self.sub_index_levels
    }

    /// The time-range transform applied to the property reached at this
    /// level, if any. Only set on first-property nodes.
    pub fn time_range(&self) -> Option<&TimeRangeTransform> {
        self.time_range.as_ref()
    }

    pub fn has_index_with_type(&self) -> Option<&IndexLevelTypeInfo> {
        // Was `Option<IndexLevelTypeInfo>` (Copy) before the v3 sum-tree
        // expansion added `summable: Option<String>` to the struct, which
        // forced dropping `Copy`. Existing callers that wrote
        // `.map(|info| info.countable.is_countable())` keep working because
        // the closure parameter just binds via auto-deref; callers that
        // needed an owned copy clone explicitly.
        self.has_index_with_type.as_ref()
    }

    /// Checks whether the given `rhs` IndexLevel is a subset of the current IndexLevel (`self`).
    ///
    /// A level is considered a subset if:
    /// - The `level_identifier` of both IndexLevels matches.
    /// - Each sub_index_level in `rhs` is also a subset of the corresponding sub_index_level in `self`.
    ///
    /// # Parameters
    /// - `self`: The current IndexLevel to compare with.
    /// - `rhs`: The IndexLevel to check if it's a subset of `self`.
    ///
    /// # Returns
    /// Returns `true` if `rhs` is a subset of `self`, otherwise `false`.
    pub fn contains_subset(&self, rhs: &IndexLevel) -> bool {
        self.contains_subset_first_non_subset_path(rhs).is_none()
    }

    /// Checks whether the given `rhs` IndexLevel is a subset of the current IndexLevel (`self`).
    /// If `rhs` is a subset, returns `None`. Otherwise, returns the invalid path as an `Option<String>`.
    ///
    /// A level is considered a subset if:
    /// - The `level_identifier` of both IndexLevels matches.
    /// - Each sub_index_level in `rhs` is also a subset of the corresponding sub_index_level in `self`.
    ///
    /// # Parameters
    /// - `self`: The current IndexLevel to compare with.
    /// - `rhs`: The IndexLevel to check if it's a subset of `self`.
    ///
    /// # Returns
    /// Returns `None` if `rhs` is a subset of `self`, otherwise returns `Some(String)` containing the invalid path.
    /// The invalid path is constructed by joining the keys that lead to the first mismatching sub_index_level.
    pub fn contains_subset_first_non_subset_path(&self, rhs: &IndexLevel) -> Option<String> {
        // If the rhs level's identifier doesn't match, it cannot be a subset.
        if self.level_identifier != rhs.level_identifier {
            return Some("Invalid path".to_string());
        }

        // Check if each sub_index_level in the rhs is a subset of self.
        for (key, rhs_sub_index) in &rhs.sub_index_levels {
            match self.sub_index_levels.get(key) {
                Some(self_sub_index) => {
                    // If the rhs sub_index is not a subset of the corresponding self sub_index, return the invalid path.
                    if let Some(invalid_path) =
                        self_sub_index.contains_subset_first_non_subset_path(rhs_sub_index)
                    {
                        return Some(format!("{} -> {}", key, invalid_path));
                    }
                }
                None => return Some(key.to_string()), // Key in rhs not found in self, return the invalid path.
            }
        }

        // If all checks pass, the rhs is a subset of self (return None for no invalid path).
        None
    }

    pub fn try_from_indices<I, T>(
        indices: I,
        document_type_name: &str, // TODO: We shouldn't pass document type, it's only for errors
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        I: IntoIterator<Item = T>, // T is the type of elements in the collection
        T: Borrow<Index>,          // Assuming Index is the type stored in the collection
    {
        match platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .index_versions
            .index_levels_from_indices
        {
            0 => Self::try_from_indices_v0(indices, document_type_name),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "IndexLevel::try_from_indices".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    fn try_from_indices_v0<I, T>(
        indices: I,
        document_type_name: &str,
    ) -> Result<Self, ProtocolError>
    where
        I: IntoIterator<Item = T>, // T is the type of elements in the collection
        T: Borrow<Index>,          // Assuming Index is the type stored in the collection
    {
        let mut index_level = IndexLevel {
            sub_index_levels: Default::default(),
            has_index_with_type: None,
            time_range: None,
            level_identifier: 0,
        };

        let mut counter: u64 = 0;

        for index_to_borrow in indices {
            let index = index_to_borrow.borrow();
            let mut current_level = &mut index_level;
            let mut properties_iter = index.properties.iter().enumerate().peekable();

            while let Some((position, index_part)) = properties_iter.next() {
                // A time-range transform always targets the index's first
                // property, and its level is keyed by the property name
                // *qualified with the grid* (`Index::level_key`, backed by
                // `TimeRangeTransform::storage_key`) rather than the bare
                // name. That fork is what lets several grids over one
                // timestamp — and a plain index over the same timestamp —
                // coexist: each grid's bucket starts live in their own
                // subtree instead of interleaving in one keyspace. Identical
                // grids map to the identical key, so indices sharing a grid
                // still share the level.
                let level_key = index.level_key(position, &index_part.name);
                current_level = current_level
                    .sub_index_levels
                    .entry(level_key)
                    .or_insert_with(|| {
                        counter += 1;
                        IndexLevel {
                            level_identifier: counter,
                            sub_index_levels: Default::default(),
                            has_index_with_type: None,
                            time_range: None,
                        }
                    });

                if position == 0 {
                    if let Some(transform) = &index.time_range {
                        current_level.time_range = Some(transform.clone());
                    }
                }

                // The last property
                if properties_iter.peek().is_none() {
                    // This level already has been initialized.
                    // It means there are two indices with the same combination of properties.

                    // We might need to take into account the sorting order when we have it
                    if current_level.has_index_with_type.is_some() {
                        // an index already exists return error
                        return Err(ConsensusError::BasicError(BasicError::DuplicateIndexError(
                            DuplicateIndexError::new(
                                document_type_name.to_owned(),
                                index.name.clone(),
                            ),
                        ))
                        .into());
                    }

                    let index_type = if index.unique {
                        UniqueIndex
                    } else {
                        NonUniqueIndex
                    };

                    // if things are null searchable that means we should insert with all null

                    current_level.has_index_with_type = Some(IndexLevelTypeInfo {
                        should_insert_with_all_null: index.null_searchable,
                        index_type,
                        countable: index.countable,
                        range_countable: index.range_countable,
                        summable: index.summable.clone(),
                        range_summable: index.range_summable,
                        // The ranking axes live on the same terminating level
                        // as the range axes they extend: this is the level
                        // named after the index's LAST property, whose children
                        // are that property's value trees (one per group). The
                        // rs-drive write path reads them off the very same
                        // `IndexLevelTypeInfo` it already consults for
                        // `range_countable` / `range_summable` when it picks
                        // the property-name tree variant.
                        ranked_countable: index.ranked_countable,
                        ranked_summable: index.ranked_summable,
                        ranked_averageable: index.ranked_averageable,
                        // indexOnly member key. Only ever `Some` on PV14+
                        // contracts (the grammar rejects the keyword below
                        // generation 3), so stamping it here changes nothing
                        // for any historical index level.
                        terminal: index.terminal.clone(),
                        // Same PV14+ gating as `terminal` — `false` on
                        // every historical index level.
                        preallocated: index.preallocated,
                    });
                }
            }
        }

        Ok(index_level)
    }

    #[cfg(feature = "validation")]
    pub fn validate_update(
        &self,
        document_type_name: &str,
        new_indices: &Self,
    ) -> SimpleConsensusValidationResult {
        // There is no changes. All good
        if self == new_indices {
            return SimpleConsensusValidationResult::new();
        }

        // We do not allow any index modifications now, but we want to figure out
        // what changed, so we compare one way then the other

        // If the new contract document type doesn't contain all previous indexes
        if let Some(non_subset_path) = new_indices.contains_subset_first_non_subset_path(self) {
            return SimpleConsensusValidationResult::new_with_error(
                DataContractInvalidIndexDefinitionUpdateError::new(
                    document_type_name.to_string(),
                    non_subset_path,
                )
                .into(),
            );
        }

        // If the old contract document type doesn't contain all new indexes
        if let Some(non_subset_path) = self.contains_subset_first_non_subset_path(new_indices) {
            return SimpleConsensusValidationResult::new_with_error(
                DataContractInvalidIndexDefinitionUpdateError::new(
                    document_type_name.to_string(),
                    non_subset_path,
                )
                .into(),
            );
        }

        // Check that the countability properties (`countable` and
        // `range_countable`) have not changed on any existing index.
        // Both flags drive GroveDB tree-variant choice at contract
        // creation, so changing either would require rebuilding the
        // index tree structure — both are immutable after creation.
        if let Some(countable_change_path) = self.find_first_countability_change(new_indices) {
            return SimpleConsensusValidationResult::new_with_error(
                DataContractInvalidIndexDefinitionUpdateError::new(
                    document_type_name.to_string(),
                    countable_change_path,
                )
                .into(),
            );
        }

        // Same check on the sum surface (`summable` property-name and
        // `range_summable`). Identical reasoning to the countability
        // immutability above — both flags drive GroveDB tree variant
        // choice (NormalTree / SumTree / ProvableSumTree / CountSumTree /
        // ProvableCountSumTree depending on the `(countable, summable)`
        // combination), and toggling them post-creation invalidates the
        // on-disk layout. Additionally, changing the *name* of the
        // summed property changes which document field gets read into
        // `ItemWithSumItem` references on insert — silently breaking
        // every subsequent aggregation if allowed.
        if let Some(summable_change_path) = self.find_first_summability_change(new_indices) {
            return SimpleConsensusValidationResult::new_with_error(
                DataContractInvalidIndexDefinitionUpdateError::new(
                    document_type_name.to_string(),
                    summable_change_path,
                )
                .into(),
            );
        }

        // Same check on the ranking surface (`ranked_countable` /
        // `ranked_summable` / `ranked_averageable`). All three are checked
        // together in one helper rather than folded into the count and sum
        // helpers above because the Avg axis straddles both — it is neither a
        // count-only nor a sum-only property — and because the set of ranking
        // axes is what determines the indexed tree's axis list, which is
        // committed into the parent hash at contract creation. Adding or
        // removing an axis after the fact would require rebuilding the ordered
        // secondaries for every existing group, so the whole set is immutable.
        if let Some(ranked_change_path) = self.find_first_ranked_change(new_indices) {
            return SimpleConsensusValidationResult::new_with_error(
                DataContractInvalidIndexDefinitionUpdateError::new(
                    document_type_name.to_string(),
                    ranked_change_path,
                )
                .into(),
            );
        }

        // A time-range transform determines how many index entries each
        // document produces and under which bucket keys. Changing it after
        // creation would leave already-stored documents indexed under stale
        // buckets, so it is immutable — reject any change.
        if let Some(time_range_change_path) = self.find_first_time_range_change(new_indices) {
            return SimpleConsensusValidationResult::new_with_error(
                DataContractInvalidIndexDefinitionUpdateError::new(
                    document_type_name.to_string(),
                    time_range_change_path,
                )
                .into(),
            );
        }

        // The `preallocated` flag decides who creates the index's dynamic
        // trees and whether last-entry deletes may prune them — see
        // `find_first_preallocated_change` for why a flip in either
        // direction breaks already-written state. Immutable like the flags
        // above.
        if let Some(preallocated_change_path) = self.find_first_preallocated_change(new_indices) {
            return SimpleConsensusValidationResult::new_with_error(
                DataContractInvalidIndexDefinitionUpdateError::new(
                    document_type_name.to_string(),
                    preallocated_change_path,
                )
                .into(),
            );
        }

        SimpleConsensusValidationResult::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::document_type::IndexProperty;
    use assert_matches::assert_matches;

    #[test]
    fn should_pass_if_indices_are_the_same() {
        let platform_version = PlatformVersion::latest();
        let document_type_name = "test";

        let old_indices = vec![Index {
            name: "test".to_string(),
            properties: vec![IndexProperty {
                name: "test".to_string(),
                ascending: false,
            }],
            unique: false,
            null_searchable: true,
            contested_index: None,
            countable: IndexCountability::NotCountable,
            range_countable: false,
            summable: None,
            range_summable: false,
            ranked_countable: false,
            ranked_summable: false,
            ranked_averageable: false,
            time_range: None,
            terminal: None,
            preallocated: false,
        }];

        let old_index_structure =
            IndexLevel::try_from_indices(&old_indices, document_type_name, platform_version)
                .expect("failed to create old index level");

        let new_index_structure = old_index_structure.clone();

        let result = old_index_structure.validate_update(document_type_name, &new_index_structure);

        assert!(result.is_valid());
    }

    #[test]
    fn should_pass_if_new_index_with_only_new_field_is_add() {
        let platform_version = PlatformVersion::latest();
        let document_type_name = "test";

        let old_indices = vec![Index {
            name: "test".to_string(),
            properties: vec![IndexProperty {
                name: "test".to_string(),
                ascending: false,
            }],
            unique: false,
            null_searchable: true,
            contested_index: None,
            countable: IndexCountability::NotCountable,
            range_countable: false,
            summable: None,
            range_summable: false,
            ranked_countable: false,
            ranked_summable: false,
            ranked_averageable: false,
            time_range: None,
            terminal: None,
            preallocated: false,
        }];

        let new_indices = vec![
            Index {
                name: "test".to_string(),
                properties: vec![IndexProperty {
                    name: "test".to_string(),
                    ascending: false,
                }],
                unique: false,
                null_searchable: true,
                contested_index: None,
                countable: IndexCountability::NotCountable,
                range_countable: false,
                summable: None,
                range_summable: false,
                ranked_countable: false,
                ranked_summable: false,
                ranked_averageable: false,
                time_range: None,
                terminal: None,
                preallocated: false,
            },
            Index {
                name: "test2".to_string(),
                properties: vec![IndexProperty {
                    name: "test2".to_string(),
                    ascending: false,
                }],
                unique: false,
                null_searchable: true,
                contested_index: None,
                countable: IndexCountability::NotCountable,
                range_countable: false,
                summable: None,
                range_summable: false,
                ranked_countable: false,
                ranked_summable: false,
                ranked_averageable: false,
                time_range: None,
                terminal: None,
                preallocated: false,
            },
        ];

        let old_index_structure =
            IndexLevel::try_from_indices(&old_indices, document_type_name, platform_version)
                .expect("failed to create old index level");

        let new_index_structure =
            IndexLevel::try_from_indices(&new_indices, document_type_name, platform_version)
                .expect("failed to create old index level");

        let result = old_index_structure.validate_update(document_type_name, &new_index_structure);

        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::DataContractInvalidIndexDefinitionUpdateError(e)
            )] if e.index_path() == "test2"
        );
    }

    #[test]
    fn should_return_invalid_result_if_some_indices_are_removed() {
        let platform_version = PlatformVersion::latest();
        let document_type_name = "test";

        let old_indices = vec![
            Index {
                name: "test".to_string(),
                properties: vec![IndexProperty {
                    name: "test".to_string(),
                    ascending: false,
                }],
                unique: false,
                null_searchable: true,
                contested_index: None,
                countable: IndexCountability::NotCountable,
                range_countable: false,
                summable: None,
                range_summable: false,
                ranked_countable: false,
                ranked_summable: false,
                ranked_averageable: false,
                time_range: None,
                terminal: None,
                preallocated: false,
            },
            Index {
                name: "test2".to_string(),
                properties: vec![IndexProperty {
                    name: "test2".to_string(),
                    ascending: false,
                }],
                unique: false,
                null_searchable: true,
                contested_index: None,
                countable: IndexCountability::NotCountable,
                range_countable: false,
                summable: None,
                range_summable: false,
                ranked_countable: false,
                ranked_summable: false,
                ranked_averageable: false,
                time_range: None,
                terminal: None,
                preallocated: false,
            },
        ];

        let new_indices = vec![Index {
            name: "test".to_string(),
            properties: vec![IndexProperty {
                name: "test".to_string(),
                ascending: false,
            }],
            unique: false,
            null_searchable: true,
            contested_index: None,
            countable: IndexCountability::NotCountable,
            range_countable: false,
            summable: None,
            range_summable: false,
            ranked_countable: false,
            ranked_summable: false,
            ranked_averageable: false,
            time_range: None,
            terminal: None,
            preallocated: false,
        }];

        let old_index_structure =
            IndexLevel::try_from_indices(&old_indices, document_type_name, platform_version)
                .expect("failed to create old index level");

        let new_index_structure =
            IndexLevel::try_from_indices(&new_indices, document_type_name, platform_version)
                .expect("failed to create old index level");

        let result = old_index_structure.validate_update(document_type_name, &new_index_structure);

        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::DataContractInvalidIndexDefinitionUpdateError(e)
            )] if e.index_path() == "test2"
        );
    }

    #[test]
    fn should_return_invalid_result_if_additional_property_is_added_to_existing_index() {
        let platform_version = PlatformVersion::latest();
        let document_type_name = "test";

        let old_indices = vec![Index {
            name: "test".to_string(),
            properties: vec![IndexProperty {
                name: "test".to_string(),
                ascending: false,
            }],
            unique: false,
            null_searchable: true,
            contested_index: None,
            countable: IndexCountability::NotCountable,
            range_countable: false,
            summable: None,
            range_summable: false,
            ranked_countable: false,
            ranked_summable: false,
            ranked_averageable: false,
            time_range: None,
            terminal: None,
            preallocated: false,
        }];

        let new_indices = vec![Index {
            name: "test".to_string(),
            properties: vec![
                IndexProperty {
                    name: "test".to_string(),
                    ascending: false,
                },
                IndexProperty {
                    name: "test2".to_string(),
                    ascending: false,
                },
            ],
            unique: false,
            null_searchable: true,
            contested_index: None,
            countable: IndexCountability::NotCountable,
            range_countable: false,
            summable: None,
            range_summable: false,
            ranked_countable: false,
            ranked_summable: false,
            ranked_averageable: false,
            time_range: None,
            terminal: None,
            preallocated: false,
        }];

        let old_index_structure =
            IndexLevel::try_from_indices(&old_indices, document_type_name, platform_version)
                .expect("failed to create old index level");

        let new_index_structure =
            IndexLevel::try_from_indices(&new_indices, document_type_name, platform_version)
                .expect("failed to create old index level");

        let result = old_index_structure.validate_update(document_type_name, &new_index_structure);

        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::DataContractInvalidIndexDefinitionUpdateError(e)
            )] if e.index_path() == "test -> test2"
        );
    }

    #[test]
    fn should_return_invalid_result_if_property_is_removed_to_existing_index() {
        let platform_version = PlatformVersion::latest();
        let document_type_name = "test";

        let old_indices = vec![Index {
            name: "test".to_string(),
            properties: vec![
                IndexProperty {
                    name: "test".to_string(),
                    ascending: false,
                },
                IndexProperty {
                    name: "test2".to_string(),
                    ascending: false,
                },
            ],
            unique: false,
            null_searchable: true,
            contested_index: None,
            countable: IndexCountability::NotCountable,
            range_countable: false,
            summable: None,
            range_summable: false,
            ranked_countable: false,
            ranked_summable: false,
            ranked_averageable: false,
            time_range: None,
            terminal: None,
            preallocated: false,
        }];

        let new_indices = vec![Index {
            name: "test".to_string(),
            properties: vec![IndexProperty {
                name: "test".to_string(),
                ascending: false,
            }],
            unique: false,
            null_searchable: true,
            contested_index: None,
            countable: IndexCountability::NotCountable,
            range_countable: false,
            summable: None,
            range_summable: false,
            ranked_countable: false,
            ranked_summable: false,
            ranked_averageable: false,
            time_range: None,
            terminal: None,
            preallocated: false,
        }];

        let old_index_structure =
            IndexLevel::try_from_indices(&old_indices, document_type_name, platform_version)
                .expect("failed to create old index level");

        let new_index_structure =
            IndexLevel::try_from_indices(&new_indices, document_type_name, platform_version)
                .expect("failed to create old index level");

        let result = old_index_structure.validate_update(document_type_name, &new_index_structure);

        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::DataContractInvalidIndexDefinitionUpdateError(e)
            )] if e.index_path() == "test -> test2"
        );
    }

    #[test]
    fn should_return_invalid_result_if_countable_changed_from_false_to_true() {
        let platform_version = PlatformVersion::latest();
        let document_type_name = "test";

        let old_indices = vec![Index {
            name: "test".to_string(),
            properties: vec![IndexProperty {
                name: "test".to_string(),
                ascending: false,
            }],
            unique: false,
            null_searchable: true,
            contested_index: None,
            countable: IndexCountability::NotCountable,
            range_countable: false,
            summable: None,
            range_summable: false,
            ranked_countable: false,
            ranked_summable: false,
            ranked_averageable: false,
            time_range: None,
            terminal: None,
            preallocated: false,
        }];

        let new_indices = vec![Index {
            name: "test".to_string(),
            properties: vec![IndexProperty {
                name: "test".to_string(),
                ascending: false,
            }],
            unique: false,
            null_searchable: true,
            contested_index: None,
            countable: IndexCountability::Countable,
            range_countable: false,
            summable: None,
            range_summable: false,
            ranked_countable: false,
            ranked_summable: false,
            ranked_averageable: false,
            time_range: None,
            terminal: None,
            preallocated: false,
        }];

        let old_index_structure =
            IndexLevel::try_from_indices(&old_indices, document_type_name, platform_version)
                .expect("failed to create old index level");

        let new_index_structure =
            IndexLevel::try_from_indices(&new_indices, document_type_name, platform_version)
                .expect("failed to create new index level");

        let result = old_index_structure.validate_update(document_type_name, &new_index_structure);

        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::DataContractInvalidIndexDefinitionUpdateError(e)
            )] if e.index_path() == "test -> (countable: NotCountable -> Countable)"
        );
    }

    #[test]
    fn should_return_invalid_result_if_countable_changed_from_true_to_false() {
        let platform_version = PlatformVersion::latest();
        let document_type_name = "test";

        let old_indices = vec![Index {
            name: "test".to_string(),
            properties: vec![IndexProperty {
                name: "test".to_string(),
                ascending: false,
            }],
            unique: false,
            null_searchable: true,
            contested_index: None,
            countable: IndexCountability::Countable,
            range_countable: false,
            summable: None,
            range_summable: false,
            ranked_countable: false,
            ranked_summable: false,
            ranked_averageable: false,
            time_range: None,
            terminal: None,
            preallocated: false,
        }];

        let new_indices = vec![Index {
            name: "test".to_string(),
            properties: vec![IndexProperty {
                name: "test".to_string(),
                ascending: false,
            }],
            unique: false,
            null_searchable: true,
            contested_index: None,
            countable: IndexCountability::NotCountable,
            range_countable: false,
            summable: None,
            range_summable: false,
            ranked_countable: false,
            ranked_summable: false,
            ranked_averageable: false,
            time_range: None,
            terminal: None,
            preallocated: false,
        }];

        let old_index_structure =
            IndexLevel::try_from_indices(&old_indices, document_type_name, platform_version)
                .expect("failed to create old index level");

        let new_index_structure =
            IndexLevel::try_from_indices(&new_indices, document_type_name, platform_version)
                .expect("failed to create new index level");

        let result = old_index_structure.validate_update(document_type_name, &new_index_structure);

        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::DataContractInvalidIndexDefinitionUpdateError(e)
            )] if e.index_path() == "test -> (countable: Countable -> NotCountable)"
        );
    }

    #[test]
    fn should_pass_if_countable_unchanged_on_update() {
        let platform_version = PlatformVersion::latest();
        let document_type_name = "test";

        let old_indices = vec![Index {
            name: "test".to_string(),
            properties: vec![IndexProperty {
                name: "test".to_string(),
                ascending: false,
            }],
            unique: false,
            null_searchable: true,
            contested_index: None,
            countable: IndexCountability::Countable,
            range_countable: false,
            summable: None,
            range_summable: false,
            ranked_countable: false,
            ranked_summable: false,
            ranked_averageable: false,
            time_range: None,
            terminal: None,
            preallocated: false,
        }];

        let old_index_structure =
            IndexLevel::try_from_indices(&old_indices, document_type_name, platform_version)
                .expect("failed to create old index level");

        // Clone so countable stays the same
        let new_index_structure = old_index_structure.clone();

        let result = old_index_structure.validate_update(document_type_name, &new_index_structure);

        assert!(result.is_valid());
    }

    /// `range_countable` is layered on top of `countable` (it changes
    /// the index's tree shape: property-name → ProvableCountTree, value
    /// level → CountTree, sibling continuations → NonCounted) and is
    /// just as load-bearing as `countable` itself for state-sync
    /// determinism. Toggling it post-creation must be rejected for the
    /// same reasons.
    #[test]
    fn should_return_invalid_result_if_range_countable_changed_from_false_to_true() {
        let platform_version = PlatformVersion::latest();
        let document_type_name = "test";

        let old_indices = vec![Index {
            name: "test".to_string(),
            properties: vec![IndexProperty {
                name: "test".to_string(),
                ascending: false,
            }],
            unique: false,
            null_searchable: true,
            contested_index: None,
            countable: IndexCountability::Countable,
            range_countable: false,
            summable: None,
            range_summable: false,
            ranked_countable: false,
            ranked_summable: false,
            ranked_averageable: false,
            time_range: None,
            terminal: None,
            preallocated: false,
        }];

        let new_indices = vec![Index {
            name: "test".to_string(),
            properties: vec![IndexProperty {
                name: "test".to_string(),
                ascending: false,
            }],
            unique: false,
            null_searchable: true,
            contested_index: None,
            countable: IndexCountability::Countable,
            range_countable: true,
            summable: None,
            range_summable: false,
            ranked_countable: false,
            ranked_summable: false,
            ranked_averageable: false,
            time_range: None,
            terminal: None,
            preallocated: false,
        }];

        let old_index_structure =
            IndexLevel::try_from_indices(&old_indices, document_type_name, platform_version)
                .expect("failed to create old index level");

        let new_index_structure =
            IndexLevel::try_from_indices(&new_indices, document_type_name, platform_version)
                .expect("failed to create new index level");

        let result = old_index_structure.validate_update(document_type_name, &new_index_structure);

        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::DataContractInvalidIndexDefinitionUpdateError(e)
            )] if e.index_path() == "test -> (range_countable: false -> true)"
        );
    }

    #[test]
    fn should_return_invalid_result_if_range_countable_changed_from_true_to_false() {
        let platform_version = PlatformVersion::latest();
        let document_type_name = "test";

        let old_indices = vec![Index {
            name: "test".to_string(),
            properties: vec![IndexProperty {
                name: "test".to_string(),
                ascending: false,
            }],
            unique: false,
            null_searchable: true,
            contested_index: None,
            countable: IndexCountability::Countable,
            range_countable: true,
            summable: None,
            range_summable: false,
            ranked_countable: false,
            ranked_summable: false,
            ranked_averageable: false,
            time_range: None,
            terminal: None,
            preallocated: false,
        }];

        let new_indices = vec![Index {
            name: "test".to_string(),
            properties: vec![IndexProperty {
                name: "test".to_string(),
                ascending: false,
            }],
            unique: false,
            null_searchable: true,
            contested_index: None,
            countable: IndexCountability::Countable,
            range_countable: false,
            summable: None,
            range_summable: false,
            ranked_countable: false,
            ranked_summable: false,
            ranked_averageable: false,
            time_range: None,
            terminal: None,
            preallocated: false,
        }];

        let old_index_structure =
            IndexLevel::try_from_indices(&old_indices, document_type_name, platform_version)
                .expect("failed to create old index level");

        let new_index_structure =
            IndexLevel::try_from_indices(&new_indices, document_type_name, platform_version)
                .expect("failed to create new index level");

        let result = old_index_structure.validate_update(document_type_name, &new_index_structure);

        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::DataContractInvalidIndexDefinitionUpdateError(e)
            )] if e.index_path() == "test -> (range_countable: true -> false)"
        );
    }

    #[test]
    fn should_return_invalid_result_if_range_countable_changed_on_compound_index() {
        let platform_version = PlatformVersion::latest();
        let document_type_name = "test";

        let old_indices = vec![Index {
            name: "compound".to_string(),
            properties: vec![
                IndexProperty {
                    name: "first".to_string(),
                    ascending: true,
                },
                IndexProperty {
                    name: "second".to_string(),
                    ascending: true,
                },
            ],
            unique: false,
            null_searchable: true,
            contested_index: None,
            countable: IndexCountability::Countable,
            range_countable: false,
            summable: None,
            range_summable: false,
            ranked_countable: false,
            ranked_summable: false,
            ranked_averageable: false,
            time_range: None,
            terminal: None,
            preallocated: false,
        }];

        let new_indices = vec![Index {
            name: "compound".to_string(),
            properties: vec![
                IndexProperty {
                    name: "first".to_string(),
                    ascending: true,
                },
                IndexProperty {
                    name: "second".to_string(),
                    ascending: true,
                },
            ],
            unique: false,
            null_searchable: true,
            contested_index: None,
            countable: IndexCountability::Countable,
            range_countable: true,
            summable: None,
            range_summable: false,
            ranked_countable: false,
            ranked_summable: false,
            ranked_averageable: false,
            time_range: None,
            terminal: None,
            preallocated: false,
        }];

        let old_index_structure =
            IndexLevel::try_from_indices(&old_indices, document_type_name, platform_version)
                .expect("failed to create old index level");

        let new_index_structure =
            IndexLevel::try_from_indices(&new_indices, document_type_name, platform_version)
                .expect("failed to create new index level");

        let result = old_index_structure.validate_update(document_type_name, &new_index_structure);

        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::DataContractInvalidIndexDefinitionUpdateError(e)
            )] if e.index_path() == "first -> second -> (range_countable: false -> true)"
        );
    }

    #[test]
    fn should_return_invalid_result_if_countable_changed_on_compound_index() {
        let platform_version = PlatformVersion::latest();
        let document_type_name = "test";

        let old_indices = vec![Index {
            name: "compound".to_string(),
            properties: vec![
                IndexProperty {
                    name: "first".to_string(),
                    ascending: true,
                },
                IndexProperty {
                    name: "second".to_string(),
                    ascending: true,
                },
            ],
            unique: false,
            null_searchable: true,
            contested_index: None,
            countable: IndexCountability::NotCountable,
            range_countable: false,
            summable: None,
            range_summable: false,
            ranked_countable: false,
            ranked_summable: false,
            ranked_averageable: false,
            time_range: None,
            terminal: None,
            preallocated: false,
        }];

        let new_indices = vec![Index {
            name: "compound".to_string(),
            properties: vec![
                IndexProperty {
                    name: "first".to_string(),
                    ascending: true,
                },
                IndexProperty {
                    name: "second".to_string(),
                    ascending: true,
                },
            ],
            unique: false,
            null_searchable: true,
            contested_index: None,
            countable: IndexCountability::Countable,
            range_countable: false,
            summable: None,
            range_summable: false,
            ranked_countable: false,
            ranked_summable: false,
            ranked_averageable: false,
            time_range: None,
            terminal: None,
            preallocated: false,
        }];

        let old_index_structure =
            IndexLevel::try_from_indices(&old_indices, document_type_name, platform_version)
                .expect("failed to create old index level");

        let new_index_structure =
            IndexLevel::try_from_indices(&new_indices, document_type_name, platform_version)
                .expect("failed to create new index level");

        let result = old_index_structure.validate_update(document_type_name, &new_index_structure);

        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::DataContractInvalidIndexDefinitionUpdateError(e)
            )] if e.index_path() == "first -> second -> (countable: NotCountable -> Countable)"
        );
    }

    // -----------------------------------------------------------------------
    // Ranked aggregate axes (meta-schema v3 / PV14)
    // -----------------------------------------------------------------------

    /// Fully range-averageable index on `[first, second]` with the supplied
    /// ranking axes. Compound on purpose: the ranking flags must land on the
    /// level of the LAST property, which is where the value trees (one per
    /// group) hang and therefore where rs-drive picks the indexed tree variant.
    fn ranked_index(
        ranked_countable: bool,
        ranked_summable: bool,
        ranked_averageable: bool,
    ) -> Index {
        Index {
            name: "compound".to_string(),
            properties: vec![
                IndexProperty {
                    name: "first".to_string(),
                    ascending: true,
                },
                IndexProperty {
                    name: "second".to_string(),
                    ascending: true,
                },
            ],
            unique: false,
            null_searchable: true,
            contested_index: None,
            countable: IndexCountability::Countable,
            range_countable: true,
            summable: Some("score".to_string()),
            range_summable: true,
            ranked_countable,
            ranked_summable,
            ranked_averageable,
            time_range: None,
            terminal: None,
            preallocated: false,
        }
    }

    /// The ranking flags must be carried onto the terminating level — the one
    /// named after the index's LAST property — next to the range flags they
    /// extend, because that is the `IndexLevelTypeInfo` the rs-drive write path
    /// consults when it picks the property-name tree variant.
    #[test]
    fn ranked_flags_land_on_the_terminal_property_level() {
        let platform_version = PlatformVersion::latest();
        let indices = vec![ranked_index(true, false, true)];

        let structure = IndexLevel::try_from_indices(&indices, "test", platform_version)
            .expect("failed to create index level");

        let first = structure
            .sub_levels()
            .get("first")
            .expect("first level exists");
        assert!(
            first.has_index_with_type().is_none(),
            "no index terminates at the prefix level, so it carries no type info"
        );

        let second = first
            .sub_levels()
            .get("second")
            .expect("second (terminal) level exists");
        let info = second
            .has_index_with_type()
            .expect("the index terminates at the last property level");
        assert!(info.ranked_countable);
        assert!(!info.ranked_summable);
        assert!(info.ranked_averageable);
        // The range axes the ranking extends live on the same info.
        assert!(info.range_countable);
        assert!(info.range_summable);
    }

    #[test]
    fn should_return_invalid_result_if_ranked_countable_changed_from_false_to_true() {
        let platform_version = PlatformVersion::latest();
        let document_type_name = "test";

        let old_indices = vec![ranked_index(false, false, false)];
        let new_indices = vec![ranked_index(true, false, false)];

        let old_index_structure =
            IndexLevel::try_from_indices(&old_indices, document_type_name, platform_version)
                .expect("failed to create old index level");
        let new_index_structure =
            IndexLevel::try_from_indices(&new_indices, document_type_name, platform_version)
                .expect("failed to create new index level");

        let result = old_index_structure.validate_update(document_type_name, &new_index_structure);

        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::DataContractInvalidIndexDefinitionUpdateError(e)
            )] if e.index_path() == "first -> second -> (ranked_countable: false -> true)"
        );
    }

    #[test]
    fn should_return_invalid_result_if_ranked_summable_changed_from_true_to_false() {
        let platform_version = PlatformVersion::latest();
        let document_type_name = "test";

        let old_indices = vec![ranked_index(false, true, false)];
        let new_indices = vec![ranked_index(false, false, false)];

        let old_index_structure =
            IndexLevel::try_from_indices(&old_indices, document_type_name, platform_version)
                .expect("failed to create old index level");
        let new_index_structure =
            IndexLevel::try_from_indices(&new_indices, document_type_name, platform_version)
                .expect("failed to create new index level");

        let result = old_index_structure.validate_update(document_type_name, &new_index_structure);

        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::DataContractInvalidIndexDefinitionUpdateError(e)
            )] if e.index_path() == "first -> second -> (ranked_summable: true -> false)"
        );
    }

    #[test]
    fn should_return_invalid_result_if_ranked_averageable_changed() {
        let platform_version = PlatformVersion::latest();
        let document_type_name = "test";

        let old_indices = vec![ranked_index(false, false, false)];
        let new_indices = vec![ranked_index(false, false, true)];

        let old_index_structure =
            IndexLevel::try_from_indices(&old_indices, document_type_name, platform_version)
                .expect("failed to create old index level");
        let new_index_structure =
            IndexLevel::try_from_indices(&new_indices, document_type_name, platform_version)
                .expect("failed to create new index level");

        let result = old_index_structure.validate_update(document_type_name, &new_index_structure);

        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::DataContractInvalidIndexDefinitionUpdateError(e)
            )] if e.index_path() == "first -> second -> (ranked_averageable: false -> true)"
        );
    }

    #[test]
    fn should_pass_if_ranked_flags_unchanged_on_update() {
        let platform_version = PlatformVersion::latest();
        let document_type_name = "test";

        let old_indices = vec![ranked_index(true, true, true)];

        let old_index_structure =
            IndexLevel::try_from_indices(&old_indices, document_type_name, platform_version)
                .expect("failed to create old index level");
        let new_index_structure = old_index_structure.clone();

        let result = old_index_structure.validate_update(document_type_name, &new_index_structure);

        assert!(result.is_valid());
    }

    /// Adding a brand-new ranked index on update follows the same policy as
    /// adding any other new index: rejected, because the new structure is not a
    /// subset of the old one. Pinned here so the ranking work can't be read as
    /// carving out an exception.
    #[test]
    fn should_return_invalid_result_if_new_ranked_index_is_added_on_update() {
        let platform_version = PlatformVersion::latest();
        let document_type_name = "test";

        let old_indices = vec![Index {
            name: "test".to_string(),
            properties: vec![IndexProperty {
                name: "test".to_string(),
                ascending: false,
            }],
            unique: false,
            null_searchable: true,
            contested_index: None,
            countable: IndexCountability::NotCountable,
            range_countable: false,
            summable: None,
            range_summable: false,
            ranked_countable: false,
            ranked_summable: false,
            ranked_averageable: false,
            time_range: None,
            terminal: None,
            preallocated: false,
        }];

        let mut new_indices = old_indices.clone();
        new_indices.push(ranked_index(true, false, false));

        let old_index_structure =
            IndexLevel::try_from_indices(&old_indices, document_type_name, platform_version)
                .expect("failed to create old index level");
        let new_index_structure =
            IndexLevel::try_from_indices(&new_indices, document_type_name, platform_version)
                .expect("failed to create new index level");

        let result = old_index_structure.validate_update(document_type_name, &new_index_structure);

        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::DataContractInvalidIndexDefinitionUpdateError(e)
            )] if e.index_path() == "first"
        );
    }

    /// The `preallocated` flag is immutable across contract updates in
    /// either direction: turning it on leaves existing referenced
    /// documents without preallocated trees while deletes already refuse
    /// to prune, and turning it off lets last-entry deletes prune trees a
    /// referenced document's creator paid for as permanent structure.
    #[test]
    fn should_return_invalid_result_if_preallocated_changed() {
        let platform_version = PlatformVersion::latest();
        let document_type_name = "test";

        let index_with_preallocated = |preallocated: bool| Index {
            name: "test".to_string(),
            properties: vec![IndexProperty {
                name: "test".to_string(),
                ascending: false,
            }],
            unique: false,
            null_searchable: true,
            contested_index: None,
            countable: IndexCountability::NotCountable,
            range_countable: false,
            summable: None,
            range_summable: false,
            ranked_countable: false,
            ranked_summable: false,
            ranked_averageable: false,
            time_range: None,
            terminal: None,
            preallocated,
        };

        for (old_flag, new_flag) in [(false, true), (true, false)] {
            let old_index_structure = IndexLevel::try_from_indices(
                &[index_with_preallocated(old_flag)],
                document_type_name,
                platform_version,
            )
            .expect("failed to create old index level");
            let new_index_structure = IndexLevel::try_from_indices(
                &[index_with_preallocated(new_flag)],
                document_type_name,
                platform_version,
            )
            .expect("failed to create new index level");

            let result =
                old_index_structure.validate_update(document_type_name, &new_index_structure);

            let expected_path = format!("test -> (preallocated: {} -> {})", old_flag, new_flag);
            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::BasicError(
                    BasicError::DataContractInvalidIndexDefinitionUpdateError(e)
                )] if e.index_path() == expected_path
            );
        }
    }
}
