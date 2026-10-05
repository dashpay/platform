#[cfg(feature = "validation")]
mod find_first_change;

#[cfg(feature = "validation")]
use crate::consensus::basic::data_contract::DataContractInvalidIndexDefinitionUpdateError;
use crate::consensus::basic::data_contract::DuplicateIndexError;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::data_contract::document_type::index::IndexCountability;
use crate::data_contract::document_type::index::{IndexBucketing, TimeRangeTransform};
use crate::data_contract::document_type::index_level::IndexType::{
    ContestedResourceIndex, NonUniqueIndex, UniqueIndex,
};
use crate::data_contract::document_type::Index;
use crate::document::property_names::CREATED_AT;
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
    /// On an indexOnly document type, the property — or ordered list of
    /// properties, for a composite terminal — whose encoded value(s) form
    /// this index's member key: the terminal key under the `0` storage
    /// marker, where a normal index stores the document id — stored as an
    /// `Item` instead of a `Reference` because there is no primary-storage
    /// row. Always `Some` here when the declaring type is indexOnly (the
    /// parser normalizes an omitted terminal to `["$ownerId"]`), except on a
    /// `summableOffCountIndex` index, which keeps a counter instead of member
    /// entries and takes no terminal; always `None` otherwise. Carried on the
    /// level info because index levels merge
    /// across indexes sharing prefixes, and the write path only sees the
    /// level at the terminal — but two indexes can never share a full
    /// property list (duplicates are rejected), so each terminating level
    /// belongs to exactly one index and the field is unambiguous.
    pub terminal: Option<Vec<String>>,
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
    /// Whether the terminating index's entries outlive a delete of their
    /// document ([`Index::outlives_delete`], see
    /// [`crate::data_contract::document_type::index::OUTLIVES_DELETE`]): the
    /// delete walker leaves them to expire with their window, and the insert
    /// walker writes over an entry already there.
    /// Carried here for the same reason as `preallocated`. `false` on every
    /// pre-PV14 contract (the grammar rejects the keyword below meta-schema
    /// v3).
    pub outlives_delete: bool,
    /// Whether the terminating index is FLAT (an indexOnly index with no
    /// prefix properties, see [`Index::is_flat`]): its entries sit directly
    /// under the `0` bucket of its own level, which is registration-time
    /// structure like a property-name tree, so the delete walker stops its
    /// upward prune at that bucket exactly as on a preallocated index.
    /// Carried here so the walkers read the layout off the level info that
    /// defines it instead of inferring it from a path height. `false` on
    /// every pre-PV14 contract and on every prefixed index.
    pub flat: bool,
    /// The skip set of the terminating index
    /// ([`Index::skip_if_absent_properties`]): the walkers write this
    /// level's entry only for a document that carries every one of these
    /// properties, and build a level only when some entry at or below it is
    /// written. Carried here for the same reason as `terminal`: the walkers
    /// only see the merged levels, and indexes with different skip sets can
    /// share them. Empty on every index that does not skip, which is every
    /// index of every pre-PV14 contract.
    pub skip_if_absent_properties: Vec<String>,
    /// The source index of a summableOffCountIndex terminating index
    /// ([`Index::summable_off_count_index`]): the walkers keep one `SumItem` per group at
    /// this level's value position instead of a value tree, a `0` bucket and
    /// member entries. `None` on every other index, which is every index of
    /// every pre-PV14 contract.
    pub summable_off_count_index: Option<String>,
}

impl IndexLevelTypeInfo {
    /// Whether the terminating index is a `summableOffCountIndex` index.
    pub fn is_summable_off_count_index(&self) -> bool {
        self.summable_off_count_index.is_some()
    }
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
    /// When set, the property reached at this level is bucketed into
    /// windows: a timestamp by a `timeRange` grid (see [`TimeRangeTransform`])
    /// or an integer by an `integerRange` grid. Only ever set on a
    /// *first-property* node (a direct child of the root), because a grid
    /// must bucket its index's leading property. At insert/delete/update
    /// time the document's value for this property is expanded into one key
    /// per containing window instead of a single key. Immutable after
    /// contract creation.
    bucketing: Option<IndexBucketing>,
    /// When `true`, the property-name tree materialized at this level is the
    /// prefix-level Count ranking tree of exactly one index — the one whose
    /// [`Index::ranked_countable_at`] names this level's property. The
    /// rs-drive write path lays that tree out as the Count-axis indexed tree
    /// and this level's value trees as count-bearing, so the ordered
    /// secondary ranks the property's values by whole-subtree document
    /// count. Only ever `true` on PV14+ contracts (the grammar rejects the
    /// object form of `rankedCountable` below meta-schema v3), and never on
    /// a terminating level (`at` naming the last property canonicalizes to
    /// the terminal boolean form). Unambiguous despite level merging: the
    /// contract-level structural validation rejects any other index sharing
    /// a ranked prefix level or anything below it.
    ranked_count_grouping: bool,
    /// When `true`, this level sits strictly between an index's ranked
    /// prefix level ([`Self::ranked_count_grouping`] above it) and that
    /// index's terminal level: the rs-drive write path lays out its
    /// property-name tree and value trees count-bearing so every
    /// insert/delete propagates its count delta up to the prefix level's
    /// ranking secondary. Never set on the terminating level itself, whose
    /// count-bearing layout already follows from its own
    /// [`IndexLevelTypeInfo`] (`rankedCountable`'s `at` form requires
    /// `rangeCountable`). Same PV14+ gating and same non-ambiguity argument
    /// as `ranked_count_grouping`.
    count_propagating: bool,
    /// Sum-axis counterpart of [`Self::ranked_count_grouping`]: the
    /// property-name tree at this level ranks its values by the sum beneath
    /// them, for the one index whose [`Index::ranked_summable_at`] names this
    /// level. Only a `summableOffCountIndex` index declares it, whose
    /// counters carry the sums up the chain.
    ranked_sum_grouping: bool,
    /// Avg-axis counterpart of [`Self::ranked_count_grouping`], from
    /// [`Index::ranked_averageable_at`]. The level's value trees carry both a
    /// count and a sum, which the average is read from.
    ranked_average_grouping: bool,
    /// Sum-chain counterpart of [`Self::count_propagating`]: this level sits
    /// below the shallowest level ranking by sum or average and ranks by
    /// neither itself, so its trees carry the sums up to that level.
    sum_propagating: bool,
    /// When `true`, this level is the branch point of a PLAIN sibling index
    /// inside another index's prefix-ranking chain: its parent level is a
    /// chain level (a grouping or propagating level of a count, sum or
    /// average chain, [`Self::is_ranked_chain_level`]), but this level
    /// continues no ranked chain (it is neither stamped nor a range-countable
    /// or range-summable terminal). The rs-drive write paths lay its
    /// property-name tree out contributing zero to the chain's value trees —
    /// readable and provable as usual, adding nothing to any subtree total the
    /// ranking keys on — wrapped as each parent needs
    /// (`zero_contribution_wrapper`: `Element::NonCounted` under a count
    /// tree, not counted or summed under a count-and-sum tree, unwrapped
    /// under a sum tree when the branch carries no sum), the same demotion
    /// range-countable value trees apply to their sibling continuations. Structural validation admits only flag-free
    /// (countable/summable/range/ranked-free) siblings here, so nothing
    /// under an exempt branch ever needs the counts the wrapper suppresses.
    /// Stamped by a post-pass over the fully merged tree (the sibling and
    /// the ranked index can be walked in either order), and only ever `true`
    /// on PV14+ contracts (the `at` grammar is rejected below meta-schema
    /// v3), so every historical index level derives bit-identically.
    count_exempt_branch: bool,
    /// Whether an index with a non-empty skip set
    /// ([`Index::skip_if_absent_properties`]) passes through or ends at this
    /// level. The walkers only have to ask which indexes a document takes
    /// part in below a level that is stamped: every other level has an entry
    /// for every document. Never set on a contract without a skipIfAbsent
    /// index, so every historical index level derives bit-identically.
    skip_at_or_below: bool,
    /// Whether an index whose entries outlive a delete of their document
    /// ([`Index::outlives_delete`]) passes through or ends at this level. The
    /// delete walkers only have to ask which indexes a delete clears below a
    /// level that is stamped. Never set on a contract without such an index,
    /// so every historical index level derives bit-identically.
    outlives_delete_at_or_below: bool,
    /// Whether an index whose entries a delete clears (not
    /// [`Index::outlives_delete`]) passes through or ends at this level. With
    /// [`Self::outlives_delete_at_or_below`] it tells a delete walker in one
    /// read whether a stamped level holds anything it removes.
    cleared_on_delete_at_or_below: bool,
    /// On the root level: whether every index involving `$createdAt` outlives
    /// a delete, and at least one does. Then no index a delete clears is keyed
    /// by the timestamp (see
    /// [`crate::data_contract::document_type::index_only_row_commits_created_at`]).
    /// `false` on every other level, and on every contract without such an
    /// index.
    created_at_indexed_only_by_outliving: bool,
    /// unique level identifier
    level_identifier: u64,
}

impl IndexLevel {
    /// A level with no sub-levels, no terminating index and no stamp, the
    /// node every level starts as before its indexes stamp it.
    fn empty(level_identifier: u64) -> Self {
        IndexLevel {
            sub_index_levels: Default::default(),
            has_index_with_type: None,
            bucketing: None,
            ranked_count_grouping: false,
            count_propagating: false,
            ranked_sum_grouping: false,
            ranked_average_grouping: false,
            sum_propagating: false,
            count_exempt_branch: false,
            skip_at_or_below: false,
            outlives_delete_at_or_below: false,
            cleared_on_delete_at_or_below: false,
            created_at_indexed_only_by_outliving: false,
            level_identifier,
        }
    }

    pub fn identifier(&self) -> u64 {
        self.level_identifier
    }

    pub fn sub_levels(&self) -> &BTreeMap<String, IndexLevel> {
        &self.sub_index_levels
    }

    /// The time-range transform applied to the property reached at this
    /// level, if any. Only set on first-property nodes.
    pub fn time_range(&self) -> Option<&TimeRangeTransform> {
        self.bucketing.as_ref().and_then(IndexBucketing::time_range)
    }

    /// The grid (time or integer) applied to the property reached at this
    /// level, if any. Only set on first-property nodes.
    pub fn bucketing(&self) -> Option<&IndexBucketing> {
        self.bucketing.as_ref()
    }

    /// Whether this level hosts a prefix-level Count ranking — see the field
    /// docs on [`IndexLevel`].
    pub fn ranked_count_grouping(&self) -> bool {
        self.ranked_count_grouping
    }

    /// Whether this level sits between a prefix-level Count ranking and its
    /// index's terminal, and must be laid out count-bearing — see the field
    /// docs on [`IndexLevel`].
    pub fn count_propagating(&self) -> bool {
        self.count_propagating
    }

    /// Whether this level hosts a prefix-level Sum ranking — see the field
    /// docs on [`IndexLevel`].
    pub fn ranked_sum_grouping(&self) -> bool {
        self.ranked_sum_grouping
    }

    /// Whether this level hosts a prefix-level Avg ranking — see the field
    /// docs on [`IndexLevel`].
    pub fn ranked_average_grouping(&self) -> bool {
        self.ranked_average_grouping
    }

    /// Whether this level carries sums up to a prefix-level Sum or Avg
    /// ranking without ranking itself — see the field docs on [`IndexLevel`].
    pub fn sum_propagating(&self) -> bool {
        self.sum_propagating
    }

    /// Whether the value trees of this prefix-ranking chain level carry a
    /// count: it ranks by count or average, or carries counts up to a level
    /// that does.
    pub fn chain_carries_counts(&self) -> bool {
        self.ranked_count_grouping || self.ranked_average_grouping || self.count_propagating
    }

    /// Whether the value trees of this prefix-ranking chain level carry a
    /// sum: it ranks by sum or average, or carries sums up to a level that
    /// does.
    pub fn chain_carries_sums(&self) -> bool {
        self.ranked_sum_grouping || self.ranked_average_grouping || self.sum_propagating
    }

    /// Whether this level is on a prefix-ranking chain at all.
    pub fn is_ranked_chain_level(&self) -> bool {
        self.chain_carries_counts() || self.chain_carries_sums()
    }

    /// Whether this level is a plain sibling's branch point inside a
    /// prefix-ranking chain, laid out contributing nothing to the chain's
    /// totals — see the field docs on [`IndexLevel`].
    pub fn count_exempt_branch(&self) -> bool {
        self.count_exempt_branch
    }

    /// Whether a skipIfAbsent index passes through or ends at this level —
    /// see the field docs on [`IndexLevel`].
    pub fn skip_at_or_below(&self) -> bool {
        self.skip_at_or_below
    }

    /// Whether an index whose entries outlive a delete passes through or ends
    /// at this level — see the field docs on [`IndexLevel`].
    pub fn outlives_delete_at_or_below(&self) -> bool {
        self.outlives_delete_at_or_below
    }

    /// Whether an index whose entries a delete clears passes through or ends
    /// at this level — see the field docs on [`IndexLevel`].
    pub fn cleared_on_delete_at_or_below(&self) -> bool {
        self.cleared_on_delete_at_or_below
    }

    /// On the root level: whether only indexes whose entries outlive a
    /// delete involve `$createdAt` — see the field docs on [`IndexLevel`].
    pub fn created_at_indexed_only_by_outliving(&self) -> bool {
        self.created_at_indexed_only_by_outliving
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

    /// The type info of the `summableOffCountIndex` index this level ends,
    /// whose counter stands at the level's value position in place of a value
    /// tree; `None` when the level ends no such index.
    pub fn summable_off_count_index_info(&self) -> Option<&IndexLevelTypeInfo> {
        self.has_index_with_type()
            .filter(|info| info.is_summable_off_count_index())
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
        let mut index_level = IndexLevel::empty(0);

        let mut counter: u64 = 0;
        let mut created_at_in_outliving = false;
        let mut created_at_in_cleared = false;

        for index_to_borrow in indices {
            let index = index_to_borrow.borrow();
            if index.involves(CREATED_AT) {
                if index.outlives_delete {
                    created_at_in_outliving = true;
                } else {
                    created_at_in_cleared = true;
                }
            }
            // The positions of the properties hosting prefix-level Count
            // rankings, when the index declares any. The parser guarantees
            // each name resolves to a non-terminal property; the lookups
            // stay defensive because unvalidated `Index` values can reach
            // this derivation (check_tx, fixtures), and for those a
            // dangling `at` simply stamps nothing.
            let ranked_at_positions: Vec<usize> = index
                .at_level_positions(&index.ranked_countable_at)
                .collect();
            let sum_at_positions: Vec<usize> = index
                .at_level_positions(&index.ranked_summable_at)
                .collect();
            let average_at_positions: Vec<usize> = index
                .at_level_positions(&index.ranked_averageable_at)
                .collect();
            // The count chain starts at the shallowest level ranking by count
            // or average, the sum chain at the shallowest ranking by sum or
            // average: an average reads both.
            let min_ranked_at_position = index.shallowest_count_chain_position();
            let min_sum_at_position = index.shallowest_sum_chain_position();
            // A FLAT indexOnly index has no prefix properties: its entries
            // live directly under one level keyed by the terminal's
            // component names (`flat_level_key_for`, whose zero-byte
            // prefix keeps it disjoint from every property-name tree). A
            // property-less index with no terminal cannot exist on a
            // parsed type — the parser refuses it — so a hand-built one
            // stamps nothing here rather than a level nothing could key.
            if index.properties.is_empty() {
                let Some(flat_key) = index.flat_level_key() else {
                    continue;
                };
                let flat_level =
                    index_level
                        .sub_index_levels
                        .entry(flat_key)
                        .or_insert_with(|| {
                            counter += 1;
                            IndexLevel::empty(counter)
                        });
                if flat_level.has_index_with_type.is_some() {
                    return Err(ConsensusError::BasicError(BasicError::DuplicateIndexError(
                        DuplicateIndexError::new(document_type_name.to_owned(), index.name.clone()),
                    ))
                    .into());
                }
                flat_level.has_index_with_type = Some(Self::terminator_info(index));
                continue;
            }

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
                        IndexLevel::empty(counter)
                    });

                if !index.skip_if_absent_properties.is_empty() {
                    current_level.skip_at_or_below = true;
                }
                if index.outlives_delete {
                    current_level.outlives_delete_at_or_below = true;
                } else {
                    current_level.cleared_on_delete_at_or_below = true;
                }

                if position == 0 {
                    if let Some(bucketing) = index.bucketing() {
                        current_level.bucketing = Some(bucketing);
                    }
                }

                // Prefix-level Count ranking stamps: every `at` level hosts
                // a grouping tree; the levels strictly between the
                // shallowest of them and the terminal that host none must
                // be count-bearing so deltas propagate up through the
                // chain. The terminal level never takes either stamp: its
                // ranked/count-bearing layout follows from its own
                // `IndexLevelTypeInfo` below (`at` requires
                // `rangeCountable`), and the parser folds a last-property
                // `at` name into the terminal boolean — so a hand-built
                // `Index` carrying the terminal name in the vector stamps
                // nothing rather than marking a level both grouping and
                // terminating.
                //
                // The Sum and Avg axes (a `summableOffCountIndex` index's
                // chain) stamp the same way, an average ranking needing both
                // the count and the sum chain.
                if properties_iter.peek().is_some() {
                    let ranks_count = ranked_at_positions.contains(&position);
                    let ranks_sum = sum_at_positions.contains(&position);
                    let ranks_average = average_at_positions.contains(&position);
                    current_level.ranked_count_grouping |= ranks_count;
                    current_level.ranked_sum_grouping |= ranks_sum;
                    current_level.ranked_average_grouping |= ranks_average;
                    if let Some(min_at_position) = min_ranked_at_position {
                        if !ranks_count && !ranks_average && position > min_at_position {
                            current_level.count_propagating = true;
                        }
                    }
                    if let Some(min_at_position) = min_sum_at_position {
                        if !ranks_sum && !ranks_average && position > min_at_position {
                            current_level.sum_propagating = true;
                        }
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

                    current_level.has_index_with_type = Some(Self::terminator_info(index));
                }
            }
        }

        // Post-pass: stamp the branch points of plain siblings inside
        // prefix-ranking chains. Runs over the fully merged tree because
        // the stamps depend on BOTH indexes — the ranked one stamps the
        // chain levels, the sibling contributes the branch child — and
        // the iteration order of `indices` must not matter. A no-op for
        // every index set without an `at` ranking (no level is stamped
        // grouping or propagating), which is every pre-PV14 contract.
        Self::stamp_count_exempt_branches(&mut index_level);
        index_level.created_at_indexed_only_by_outliving =
            created_at_in_outliving && !created_at_in_cleared;

        Ok(index_level)
    }

    /// The terminator stamp an index leaves on the level its last property
    /// reaches (or, for a flat index, on its flat level): the index type and
    /// every per-index axis the write path reads off the level.
    fn terminator_info(index: &Index) -> IndexLevelTypeInfo {
        let index_type = if index.unique {
            UniqueIndex
        } else {
            NonUniqueIndex
        };
        // if things are null searchable that means we should insert with all null
        IndexLevelTypeInfo {
            should_insert_with_all_null: index.null_searchable,
            index_type,
            countable: index.countable,
            range_countable: index.range_countable,
            summable: index.summable.clone(),
            range_summable: index.range_summable,
            // The ranking axes live on the same terminating level as the
            // range axes they extend: this is the level named after the
            // index's LAST property, whose children are that property's
            // value trees (one per group). The rs-drive write path reads
            // them off the very same `IndexLevelTypeInfo` it already
            // consults for `range_countable` / `range_summable` when it
            // picks the property-name tree variant.
            ranked_countable: index.ranked_countable,
            ranked_summable: index.ranked_summable,
            ranked_averageable: index.ranked_averageable,
            // indexOnly member key. Only ever `Some` on PV14+ contracts
            // (the grammar rejects the keyword below generation 3), so
            // stamping it here changes nothing for any historical index
            // level.
            terminal: index.terminal.clone(),
            // Same PV14+ gating as `terminal` — `false` on every
            // historical index level.
            preallocated: index.preallocated,
            outlives_delete: index.outlives_delete,
            // A flat index terminates on its own level, directly under the
            // document type: the one layout whose prune boundary is the
            // level's `0` bucket rather than the document type.
            flat: index.is_flat(),
            skip_if_absent_properties: index.skip_if_absent_properties.clone(),
            // Same PV14+ gating as `terminal`.
            summable_off_count_index: index.summable_off_count_index.clone(),
        }
    }

    /// Recursively marks, under every prefix-ranking chain level (a grouping
    /// or propagating level of a count, sum or average chain), the child
    /// levels that do NOT continue the chain as
    /// [`Self::count_exempt_branch`]. The chain child is the one that is
    /// itself stamped (a deeper `at` level, or a propagating level between
    /// two chain levels) or that terminates the ranked index — a count
    /// chain's `at` grammar requires `rangeCountable` and a
    /// `summableOffCountIndex` index requires `rangeSummable`, so the
    /// chain's terminal level always carries a range-countable or
    /// range-summable terminator stamp, while structural validation
    /// guarantees every admitted sibling is flag-free at and below the
    /// shared levels. For an unvalidated index set that
    /// violates those invariants this derivation stays total (fixtures and
    /// check_tx reach it); the rs-drive tree-type resolver keeps its own
    /// fail-closed guards for the genuinely contradictory shapes.
    fn stamp_count_exempt_branches(level: &mut IndexLevel) {
        let level_is_chain = level.is_ranked_chain_level();
        for child in level.sub_index_levels.values_mut() {
            if level_is_chain {
                // A Sum or Avg chain's terminal (a `summableOffCountIndex`
                // index) is range-summable rather than range-countable.
                let child_continues_chain = child.is_ranked_chain_level()
                    || child
                        .has_index_with_type
                        .as_ref()
                        .map(|info| info.range_countable || info.range_summable)
                        .unwrap_or(false);
                child.count_exempt_branch = !child_continues_chain;
            }
            Self::stamp_count_exempt_branches(child);
        }
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

        // A time-range or integer-range grid determines how many index
        // entries each document produces and under which bucket keys. Changing it after
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

        // Whether an index's entries outlive their document's delete decides
        // what a delete carries and what a row commits to: turning it on
        // would leave every existing row committed to values its deletes no
        // longer carry, and turning it off would ask every delete for a value
        // the rows written since never committed to. Immutable like the flags
        // above.
        if let Some(outlives_delete_change_path) =
            self.find_first_outlives_delete_change(new_indices)
        {
            return SimpleConsensusValidationResult::new_with_error(
                DataContractInvalidIndexDefinitionUpdateError::new(
                    document_type_name.to_string(),
                    outlives_delete_change_path,
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
            ranked_countable_at: vec![],
            ranked_summable_at: Vec::new(),
            ranked_averageable_at: Vec::new(),
            ranked_summable: false,
            ranked_averageable: false,
            time_range: None,
            integer_range: None,
            terminal: None,
            preallocated: false,
            outlives_delete: false,
            skip_if_absent: false,
            skip_if_absent_properties: Vec::new(),
            summable_off_count_index: None,
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
            ranked_countable_at: vec![],
            ranked_summable_at: Vec::new(),
            ranked_averageable_at: Vec::new(),
            ranked_summable: false,
            ranked_averageable: false,
            time_range: None,
            integer_range: None,
            terminal: None,
            preallocated: false,
            outlives_delete: false,
            skip_if_absent: false,
            skip_if_absent_properties: Vec::new(),
            summable_off_count_index: None,
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
                ranked_countable_at: vec![],
                ranked_summable_at: Vec::new(),
                ranked_averageable_at: Vec::new(),
                ranked_summable: false,
                ranked_averageable: false,
                time_range: None,
                integer_range: None,
                terminal: None,
                preallocated: false,
                outlives_delete: false,
                skip_if_absent: false,
                skip_if_absent_properties: Vec::new(),
                summable_off_count_index: None,
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
                ranked_countable_at: vec![],
                ranked_summable_at: Vec::new(),
                ranked_averageable_at: Vec::new(),
                ranked_summable: false,
                ranked_averageable: false,
                time_range: None,
                integer_range: None,
                terminal: None,
                preallocated: false,
                outlives_delete: false,
                skip_if_absent: false,
                skip_if_absent_properties: Vec::new(),
                summable_off_count_index: None,
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
                ranked_countable_at: vec![],
                ranked_summable_at: Vec::new(),
                ranked_averageable_at: Vec::new(),
                ranked_summable: false,
                ranked_averageable: false,
                time_range: None,
                integer_range: None,
                terminal: None,
                preallocated: false,
                outlives_delete: false,
                skip_if_absent: false,
                skip_if_absent_properties: Vec::new(),
                summable_off_count_index: None,
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
                ranked_countable_at: vec![],
                ranked_summable_at: Vec::new(),
                ranked_averageable_at: Vec::new(),
                ranked_summable: false,
                ranked_averageable: false,
                time_range: None,
                integer_range: None,
                terminal: None,
                preallocated: false,
                outlives_delete: false,
                skip_if_absent: false,
                skip_if_absent_properties: Vec::new(),
                summable_off_count_index: None,
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
            ranked_countable_at: vec![],
            ranked_summable_at: Vec::new(),
            ranked_averageable_at: Vec::new(),
            ranked_summable: false,
            ranked_averageable: false,
            time_range: None,
            integer_range: None,
            terminal: None,
            preallocated: false,
            outlives_delete: false,
            skip_if_absent: false,
            skip_if_absent_properties: Vec::new(),
            summable_off_count_index: None,
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
            ranked_countable_at: vec![],
            ranked_summable_at: Vec::new(),
            ranked_averageable_at: Vec::new(),
            ranked_summable: false,
            ranked_averageable: false,
            time_range: None,
            integer_range: None,
            terminal: None,
            preallocated: false,
            outlives_delete: false,
            skip_if_absent: false,
            skip_if_absent_properties: Vec::new(),
            summable_off_count_index: None,
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
            ranked_countable_at: vec![],
            ranked_summable_at: Vec::new(),
            ranked_averageable_at: Vec::new(),
            ranked_summable: false,
            ranked_averageable: false,
            time_range: None,
            integer_range: None,
            terminal: None,
            preallocated: false,
            outlives_delete: false,
            skip_if_absent: false,
            skip_if_absent_properties: Vec::new(),
            summable_off_count_index: None,
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
            ranked_countable_at: vec![],
            ranked_summable_at: Vec::new(),
            ranked_averageable_at: Vec::new(),
            ranked_summable: false,
            ranked_averageable: false,
            time_range: None,
            integer_range: None,
            terminal: None,
            preallocated: false,
            outlives_delete: false,
            skip_if_absent: false,
            skip_if_absent_properties: Vec::new(),
            summable_off_count_index: None,
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
            ranked_countable_at: vec![],
            ranked_summable_at: Vec::new(),
            ranked_averageable_at: Vec::new(),
            ranked_summable: false,
            ranked_averageable: false,
            time_range: None,
            integer_range: None,
            terminal: None,
            preallocated: false,
            outlives_delete: false,
            skip_if_absent: false,
            skip_if_absent_properties: Vec::new(),
            summable_off_count_index: None,
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
            ranked_countable_at: vec![],
            ranked_summable_at: Vec::new(),
            ranked_averageable_at: Vec::new(),
            ranked_summable: false,
            ranked_averageable: false,
            time_range: None,
            integer_range: None,
            terminal: None,
            preallocated: false,
            outlives_delete: false,
            skip_if_absent: false,
            skip_if_absent_properties: Vec::new(),
            summable_off_count_index: None,
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
            ranked_countable_at: vec![],
            ranked_summable_at: Vec::new(),
            ranked_averageable_at: Vec::new(),
            ranked_summable: false,
            ranked_averageable: false,
            time_range: None,
            integer_range: None,
            terminal: None,
            preallocated: false,
            outlives_delete: false,
            skip_if_absent: false,
            skip_if_absent_properties: Vec::new(),
            summable_off_count_index: None,
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
            ranked_countable_at: vec![],
            ranked_summable_at: Vec::new(),
            ranked_averageable_at: Vec::new(),
            ranked_summable: false,
            ranked_averageable: false,
            time_range: None,
            integer_range: None,
            terminal: None,
            preallocated: false,
            outlives_delete: false,
            skip_if_absent: false,
            skip_if_absent_properties: Vec::new(),
            summable_off_count_index: None,
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
            ranked_countable_at: vec![],
            ranked_summable_at: Vec::new(),
            ranked_averageable_at: Vec::new(),
            ranked_summable: false,
            ranked_averageable: false,
            time_range: None,
            integer_range: None,
            terminal: None,
            preallocated: false,
            outlives_delete: false,
            skip_if_absent: false,
            skip_if_absent_properties: Vec::new(),
            summable_off_count_index: None,
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
            ranked_countable_at: vec![],
            ranked_summable_at: Vec::new(),
            ranked_averageable_at: Vec::new(),
            ranked_summable: false,
            ranked_averageable: false,
            time_range: None,
            integer_range: None,
            terminal: None,
            preallocated: false,
            outlives_delete: false,
            skip_if_absent: false,
            skip_if_absent_properties: Vec::new(),
            summable_off_count_index: None,
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
            ranked_countable_at: vec![],
            ranked_summable_at: Vec::new(),
            ranked_averageable_at: Vec::new(),
            ranked_summable: false,
            ranked_averageable: false,
            time_range: None,
            integer_range: None,
            terminal: None,
            preallocated: false,
            outlives_delete: false,
            skip_if_absent: false,
            skip_if_absent_properties: Vec::new(),
            summable_off_count_index: None,
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
            ranked_countable_at: vec![],
            ranked_summable_at: Vec::new(),
            ranked_averageable_at: Vec::new(),
            ranked_summable: false,
            ranked_averageable: false,
            time_range: None,
            integer_range: None,
            terminal: None,
            preallocated: false,
            outlives_delete: false,
            skip_if_absent: false,
            skip_if_absent_properties: Vec::new(),
            summable_off_count_index: None,
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
            ranked_countable_at: vec![],
            ranked_summable_at: Vec::new(),
            ranked_averageable_at: Vec::new(),
            ranked_summable: false,
            ranked_averageable: false,
            time_range: None,
            integer_range: None,
            terminal: None,
            preallocated: false,
            outlives_delete: false,
            skip_if_absent: false,
            skip_if_absent_properties: Vec::new(),
            summable_off_count_index: None,
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
            ranked_countable_at: vec![],
            ranked_summable_at: Vec::new(),
            ranked_averageable_at: Vec::new(),
            ranked_summable: false,
            ranked_averageable: false,
            time_range: None,
            integer_range: None,
            terminal: None,
            preallocated: false,
            outlives_delete: false,
            skip_if_absent: false,
            skip_if_absent_properties: Vec::new(),
            summable_off_count_index: None,
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
            ranked_countable_at: vec![],
            ranked_summable_at: Vec::new(),
            ranked_averageable_at: Vec::new(),
            ranked_summable: false,
            ranked_averageable: false,
            time_range: None,
            integer_range: None,
            terminal: None,
            preallocated: false,
            outlives_delete: false,
            skip_if_absent: false,
            skip_if_absent_properties: Vec::new(),
            summable_off_count_index: None,
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
            ranked_countable_at: vec![],
            ranked_summable_at: Vec::new(),
            ranked_averageable_at: Vec::new(),
            ranked_summable: false,
            ranked_averageable: false,
            time_range: None,
            integer_range: None,
            terminal: None,
            preallocated: false,
            outlives_delete: false,
            skip_if_absent: false,
            skip_if_absent_properties: Vec::new(),
            summable_off_count_index: None,
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
            ranked_countable_at: vec![],
            ranked_summable_at: Vec::new(),
            ranked_averageable_at: Vec::new(),
            ranked_summable: false,
            ranked_averageable: false,
            time_range: None,
            integer_range: None,
            terminal: None,
            preallocated: false,
            outlives_delete: false,
            skip_if_absent: false,
            skip_if_absent_properties: Vec::new(),
            summable_off_count_index: None,
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
            ranked_countable_at: vec![],
            ranked_summable_at: Vec::new(),
            ranked_averageable_at: Vec::new(),
            ranked_summable: false,
            ranked_averageable: false,
            time_range: None,
            integer_range: None,
            terminal: None,
            preallocated: false,
            outlives_delete: false,
            skip_if_absent: false,
            skip_if_absent_properties: Vec::new(),
            summable_off_count_index: None,
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
            ranked_countable_at: vec![],
            ranked_summable_at: Vec::new(),
            ranked_averageable_at: Vec::new(),
            ranked_summable,
            ranked_averageable,
            time_range: None,
            integer_range: None,
            terminal: None,
            preallocated: false,
            outlives_delete: false,
            skip_if_absent: false,
            skip_if_absent_properties: Vec::new(),
            summable_off_count_index: None,
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

    /// A prefix-ranked countable index on `[first, second, third]` whose
    /// ranking sits at the named property's level.
    fn prefix_ranked_index(at: &str) -> Index {
        Index {
            name: "prefixRanked".to_string(),
            properties: ["first", "second", "third"]
                .into_iter()
                .map(|name| IndexProperty {
                    name: name.to_string(),
                    ascending: true,
                })
                .collect(),
            unique: false,
            null_searchable: true,
            contested_index: None,
            countable: IndexCountability::Countable,
            range_countable: true,
            summable: None,
            range_summable: false,
            ranked_countable: false,
            ranked_countable_at: vec![at.to_string()],
            ranked_summable_at: Vec::new(),
            ranked_averageable_at: Vec::new(),
            ranked_summable: false,
            ranked_averageable: false,
            time_range: None,
            integer_range: None,
            terminal: None,
            preallocated: false,
            outlives_delete: false,
            skip_if_absent: false,
            skip_if_absent_properties: Vec::new(),
            summable_off_count_index: None,
        }
    }

    /// `rankedCountable: { at: "first" }` on `[first, second, third]` stamps
    /// the `at` level as the grouping level, the level below it as
    /// count-propagating, and leaves the terminal level's stamps to its own
    /// `IndexLevelTypeInfo` — where the boolean ranked axis stays off.
    #[test]
    fn ranked_countable_at_stamps_grouping_and_propagation_levels() {
        let platform_version = PlatformVersion::latest();
        let indices = vec![prefix_ranked_index("first")];

        let structure = IndexLevel::try_from_indices(&indices, "test", platform_version)
            .expect("failed to create index level");

        let first = structure
            .sub_levels()
            .get("first")
            .expect("first level exists");
        assert!(first.ranked_count_grouping());
        assert!(!first.count_propagating());
        assert!(first.has_index_with_type().is_none());

        let second = first
            .sub_levels()
            .get("second")
            .expect("second level exists");
        assert!(!second.ranked_count_grouping());
        assert!(second.count_propagating());
        assert!(second.has_index_with_type().is_none());

        let third = second
            .sub_levels()
            .get("third")
            .expect("third (terminal) level exists");
        assert!(!third.ranked_count_grouping());
        assert!(
            !third.count_propagating(),
            "the terminal level's count-bearing layout follows from its own info stamp"
        );
        let info = third
            .has_index_with_type()
            .expect("the index terminates at the last property level");
        assert!(!info.ranked_countable);
        assert!(info.range_countable);
    }

    /// A plain lookup index with the given properties and no flags at all.
    fn plain_index(name: &str, properties: &[&str]) -> Index {
        Index {
            name: name.to_string(),
            properties: properties
                .iter()
                .map(|name| IndexProperty {
                    name: name.to_string(),
                    ascending: true,
                })
                .collect(),
            unique: false,
            null_searchable: true,
            contested_index: None,
            countable: IndexCountability::NotCountable,
            range_countable: false,
            summable: None,
            range_summable: false,
            ranked_countable: false,
            ranked_countable_at: vec![],
            ranked_summable_at: Vec::new(),
            ranked_averageable_at: Vec::new(),
            ranked_summable: false,
            ranked_averageable: false,
            time_range: None,
            integer_range: None,
            terminal: None,
            preallocated: false,
            outlives_delete: false,
            skip_if_absent: false,
            skip_if_absent_properties: Vec::new(),
            summable_off_count_index: None,
        }
    }

    /// A plain sibling sharing a prefix-ranking's `at` level gets its
    /// branch point stamped [`IndexLevel::count_exempt_branch`], while the
    /// chain's own continuation (here the ranked terminal, which carries
    /// `range_countable`) stays contributing — in either walk order, since
    /// the stamp comes from a post-pass over the merged tree.
    #[test]
    fn a_plain_sibling_branch_is_stamped_count_exempt() {
        let platform_version = PlatformVersion::latest();
        let ranked = prefix_ranked_index("first");
        let sibling = plain_index("sibling", &["first", "day", "extra"]);

        for indices in [
            vec![ranked.clone(), sibling.clone()],
            vec![sibling.clone(), ranked.clone()],
        ] {
            let structure = IndexLevel::try_from_indices(&indices, "test", platform_version)
                .expect("index level must build");

            let first = structure
                .sub_levels()
                .get("first")
                .expect("the shared at level exists");
            assert!(first.ranked_count_grouping());
            assert!(!first.count_exempt_branch());

            let second = first
                .sub_levels()
                .get("second")
                .expect("the chain continuation exists");
            assert!(second.count_propagating());
            assert!(
                !second.count_exempt_branch(),
                "the chain's own continuation must stay contributing"
            );

            let day = first
                .sub_levels()
                .get("day")
                .expect("the sibling branch exists");
            assert!(!day.ranked_count_grouping());
            assert!(!day.count_propagating());
            assert!(
                day.count_exempt_branch(),
                "the sibling branch point must be stamped count-exempt"
            );

            let extra = day
                .sub_levels()
                .get("extra")
                .expect("the sibling's deeper level exists");
            assert!(
                !extra.count_exempt_branch(),
                "only the branch point is exempt; below it the wrapper already shields the chain"
            );
        }
    }

    /// A plain sibling that shares the ENTIRE ranked property list and
    /// extends past its terminal stamps nothing: the ranked terminal is a
    /// chain member (its `range_countable` terminator is the chain's count
    /// source), and the extension hangs under a non-chain level, where the
    /// pre-existing continuation demotion already applies.
    #[test]
    fn a_sibling_past_the_ranked_terminal_stamps_nothing() {
        let platform_version = PlatformVersion::latest();
        let mut ranked = prefix_ranked_index("first");
        ranked.properties.truncate(2); // [first, second], at first
        let sibling = plain_index("sibling", &["first", "second", "day"]);

        let structure = IndexLevel::try_from_indices([&ranked, &sibling], "test", platform_version)
            .expect("index level must build");

        let second = structure
            .sub_levels()
            .get("first")
            .and_then(|level| level.sub_levels().get("second"))
            .expect("the shared terminal level exists");
        assert!(
            !second.count_exempt_branch(),
            "the ranked terminal is the chain's contributing continuation"
        );
        assert!(
            second
                .has_index_with_type()
                .expect("the ranked index terminates here")
                .range_countable
        );

        let day = second
            .sub_levels()
            .get("day")
            .expect("the sibling's extension exists");
        assert!(
            !day.count_exempt_branch(),
            "an extension below a non-chain level relies on the existing demotion, not the stamp"
        );
    }

    /// Without an `at` ranking the post-pass stamps nothing — pinned so
    /// the derivation stays bit-identical for every existing contract.
    #[test]
    fn count_exempt_branch_defaults_off_without_at() {
        let platform_version = PlatformVersion::latest();
        let indices = vec![
            ranked_index(true, false, false),
            plain_index("sibling", &["first", "day"]),
        ];

        let structure = IndexLevel::try_from_indices(&indices, "test", platform_version)
            .expect("index level must build");
        let first = structure
            .sub_levels()
            .get("first")
            .expect("first level exists");
        assert!(!first.count_exempt_branch());
        for child in first.sub_levels().values() {
            assert!(!child.count_exempt_branch());
        }
    }

    /// Adding the plain sibling to an existing prefix-ranked doctype on
    /// update is rejected like any added index (subset rule); pinned so
    /// the sibling admission can't be read as an update carve-out.
    #[test]
    fn adding_a_sibling_on_update_stays_rejected() {
        let platform_version = PlatformVersion::latest();
        let document_type_name = "test";

        let old_indices = vec![prefix_ranked_index("first")];
        let new_indices = vec![
            prefix_ranked_index("first"),
            plain_index("sibling", &["first", "day", "extra"]),
        ];

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
            )] if e.index_path() == "first -> day"
        );
    }

    /// A middle-property `at` leaves the levels above it unstamped.
    #[test]
    fn ranked_countable_at_middle_property_leaves_levels_above_unstamped() {
        let platform_version = PlatformVersion::latest();
        let indices = vec![prefix_ranked_index("second")];

        let structure = IndexLevel::try_from_indices(&indices, "test", platform_version)
            .expect("failed to create index level");

        let first = structure
            .sub_levels()
            .get("first")
            .expect("first level exists");
        assert!(!first.ranked_count_grouping());
        assert!(!first.count_propagating());

        let second = first
            .sub_levels()
            .get("second")
            .expect("second level exists");
        assert!(second.ranked_count_grouping());
        assert!(!second.count_propagating());
    }

    /// A hand-built `Index` (this derivation accepts unvalidated values)
    /// carrying the LAST property's name in `ranked_countable_at` — a
    /// spelling the parser always folds into the terminal boolean — must
    /// not stamp the terminal level as a grouping level: the terminal's
    /// ranked layout is its info's business, and a level that is both
    /// grouping and terminating has no coherent layout.
    #[test]
    fn a_terminal_name_in_the_at_vector_stamps_nothing() {
        let platform_version = PlatformVersion::latest();
        let mut invalid = prefix_ranked_index("first");
        invalid.ranked_countable_at = vec!["third".to_string()];

        let structure = IndexLevel::try_from_indices([&invalid], "test", platform_version)
            .expect("index level must build");
        let third = structure
            .sub_levels()
            .get("first")
            .and_then(|level| level.sub_levels().get("second"))
            .and_then(|level| level.sub_levels().get("third"))
            .expect("the terminal level exists");
        assert!(!third.ranked_count_grouping());
        assert!(!third.count_propagating());
        assert!(third.has_index_with_type().is_some());
    }

    /// Without the `at` form nothing stamps the level flags — pinned so the
    /// derivation stays bit-identical for every existing contract.
    #[test]
    fn ranked_level_flags_default_off_without_at() {
        let platform_version = PlatformVersion::latest();
        let indices = vec![ranked_index(true, false, true)];

        let structure = IndexLevel::try_from_indices(&indices, "test", platform_version)
            .expect("failed to create index level");

        let first = structure
            .sub_levels()
            .get("first")
            .expect("first level exists");
        assert!(!first.ranked_count_grouping());
        assert!(!first.count_propagating());
        let second = first
            .sub_levels()
            .get("second")
            .expect("second level exists");
        assert!(!second.ranked_count_grouping());
        assert!(!second.count_propagating());
    }

    /// Adding, removing or moving a prefix-level ranking on update is
    /// rejected: the flags live on the level, so the diff helper must
    /// catch them even though every `IndexLevelTypeInfo` stays identical.
    #[test]
    fn should_return_invalid_result_if_ranked_countable_at_changed() {
        let platform_version = PlatformVersion::latest();
        let document_type_name = "test";

        let unranked = {
            let mut index = prefix_ranked_index("first");
            index.ranked_countable_at = vec![];
            index
        };

        for (old_index, new_index, expected_path) in [
            (
                unranked.clone(),
                prefix_ranked_index("first"),
                "first -> (ranked_count_grouping: false -> true)",
            ),
            (
                prefix_ranked_index("first"),
                unranked.clone(),
                "first -> (ranked_count_grouping: true -> false)",
            ),
            (
                prefix_ranked_index("first"),
                prefix_ranked_index("second"),
                "first -> (ranked_count_grouping: true -> false)",
            ),
        ] {
            let old_index_structure =
                IndexLevel::try_from_indices(&[old_index], document_type_name, platform_version)
                    .expect("failed to create old index level");
            let new_index_structure =
                IndexLevel::try_from_indices(&[new_index], document_type_name, platform_version)
                    .expect("failed to create new index level");

            let result =
                old_index_structure.validate_update(document_type_name, &new_index_structure);

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::BasicError(
                    BasicError::DataContractInvalidIndexDefinitionUpdateError(e)
                )] if e.index_path() == expected_path
            );
        }
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
    /// Pins `IndexLevel::validate_update`'s helper. No protocol version
    /// reaches it with a counter: the keyword parses from protocol version
    /// 14, whose document type `validate_update` v1 refuses a changed index
    /// by comparing whole `Index` definitions instead.
    fn should_return_invalid_result_if_the_counted_source_changed() {
        let platform_version = PlatformVersion::latest();
        let document_type_name = "test";
        let counter_index = |source: &str| Index {
            summable: None,
            summable_off_count_index: Some(source.to_string()),
            ..ranked_index(false, false, false)
        };

        let old_index_structure = IndexLevel::try_from_indices(
            &[counter_index("byPost")],
            document_type_name,
            platform_version,
        )
        .expect("failed to create old index level");
        let new_index_structure = IndexLevel::try_from_indices(
            &[counter_index("byReply")],
            document_type_name,
            platform_version,
        )
        .expect("failed to create new index level");

        let result = old_index_structure.validate_update(document_type_name, &new_index_structure);

        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::DataContractInvalidIndexDefinitionUpdateError(e)
            )] if e.index_path()
                == "first -> second -> (summable_off_count_index: Some(\"byPost\") -> Some(\"byReply\"))"
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
            ranked_countable_at: vec![],
            ranked_summable_at: Vec::new(),
            ranked_averageable_at: Vec::new(),
            ranked_summable: false,
            ranked_averageable: false,
            time_range: None,
            integer_range: None,
            terminal: None,
            preallocated: false,
            outlives_delete: false,
            skip_if_absent: false,
            skip_if_absent_properties: Vec::new(),
            summable_off_count_index: None,
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
            ranked_countable_at: vec![],
            ranked_summable_at: Vec::new(),
            ranked_averageable_at: Vec::new(),
            ranked_summable: false,
            ranked_averageable: false,
            time_range: None,
            integer_range: None,
            terminal: None,
            preallocated,
            outlives_delete: false,
            skip_if_absent: false,
            skip_if_absent_properties: Vec::new(),
            summable_off_count_index: None,
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

    #[test]
    fn should_return_invalid_result_if_outlives_delete_changed() {
        let platform_version = PlatformVersion::latest();
        let document_type_name = "test";

        let index_with_outlives_delete = |outlives_delete: bool| Index {
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
            ranked_countable_at: vec![],
            ranked_summable_at: Vec::new(),
            ranked_averageable_at: Vec::new(),
            ranked_summable: false,
            ranked_averageable: false,
            time_range: None,
            integer_range: None,
            terminal: None,
            preallocated: false,
            outlives_delete,
            skip_if_absent: false,
            skip_if_absent_properties: Vec::new(),
            summable_off_count_index: None,
        };

        for (old_flag, new_flag) in [(false, true), (true, false)] {
            let old_index_structure = IndexLevel::try_from_indices(
                &[index_with_outlives_delete(old_flag)],
                document_type_name,
                platform_version,
            )
            .expect("failed to create old index level");
            let new_index_structure = IndexLevel::try_from_indices(
                &[index_with_outlives_delete(new_flag)],
                document_type_name,
                platform_version,
            )
            .expect("failed to create new index level");

            let result =
                old_index_structure.validate_update(document_type_name, &new_index_structure);

            let expected_path = format!("test -> (outlivesDelete: {} -> {})", old_flag, new_flag);
            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::BasicError(
                    BasicError::DataContractInvalidIndexDefinitionUpdateError(e)
                )] if e.index_path() == expected_path
            );
        }
    }
}
