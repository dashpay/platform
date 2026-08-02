//! Shared index-walker tree-type derivation for the v2 walkers.
//!
//! The four v2 index walkers (insert/delete × top-level/recursive) must
//! agree byte-for-byte on which grovedb `TreeType` each property-name
//! tree and each per-value tree gets: the insert side writes the trees
//! and the delete side emits `EstimatedLayerInformation` describing
//! them, and any drift between the two produces dry-run fees that
//! disagree with applied fees. The v1 walkers each carried a private
//! copy of the dispatch tables; v2 centralizes them here so the four
//! call sites cannot drift.
//!
//! ## The continuation demotion (new in v2)
//!
//! v2 exists to fix the shared-prefix aggregate layout defect (a
//! contract declaring an aggregating index `[a]` next to a compound
//! index `[a, b]` registered fine but rejected most document inserts).
//! Continuation property-name trees (`b`) are stored as children of
//! the aggregating value trees of `[a]`, and must contribute zero to
//! every axis the value tree aggregates. grovedb's provable
//! count-bearing trees (`ProvableCountSumTree`,
//! `ProvableCountProvableSumTree`) commit their count into every node
//! hash and therefore reject count-suppressed (`NonCounted` /
//! `NotCountedOrSummed`) children *by design* — and there is no legal
//! wrapper at all for a plain continuation under them
//! (`Element::new_not_counted_or_summed` requires a sum-bearing
//! inner). So when a sub-level has continuations, v2 demotes those two
//! value-tree variants to plain `CountSumTree`, whose count/sum live
//! in the element (not the node hashes) and which accepts suppressed
//! children.
//!
//! The demotion loses nothing observable: point-lookup count/sum
//! proofs read the aggregate off the value-tree *element* (proven by
//! inclusion in the parent merk) for provable and non-provable
//! variants alike, and the range-aggregate queries
//! (`AggregateCountOnRange` / `AggregateSumOnRange`) walk the
//! *property-name* tree one level up, to which a `CountSumTree` child
//! contributes its (count, sum) exactly like a provable child would.
//! Per-node commitments *inside* a value tree would only matter for
//! range aggregation over the value tree's own children (the `[0]`
//! ref-bucket and sibling continuations) — a query no reader
//! performs.
//!
//! Gating the demotion on "has continuations" keeps v2 bit-identical
//! to v1 for every shape without a compound sibling. One caveat for
//! shapes WITH one: pre-v14, a provable count-bearing value tree with
//! exclusively sum-bearing continuations could actually be inserted —
//! grovedb's wrapper-vs-provable guard fires only when the parent merk
//! pre-exists, and the walker always creates parent and wrapped child
//! in one batch. Contracts that used that hole (possible only since
//! the v13 sum-index grammar activated) keep their existing provable
//! value trees; values first seen at v14+ get `CountSumTree` ones.
//! Readers are indifferent — both variants serialize their (count,
//! sum) into the element and contribute identically to the
//! property-name tree's per-node aggregates — but the demotion means
//! new writes no longer depend on the unenforced guard hole.

use dpp::data_contract::document_type::IndexLevel;
use grovedb::TreeType;

/// The two tree types an index sub-level materializes: the
/// property-name tree (keys = the property's distinct values) and the
/// per-value trees underneath it (hosting the `[0]` ref-bucket plus
/// any continuation property-name trees).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct IndexLevelTreeTypes {
    /// Tree type of the property-name tree. Upgraded from
    /// `NormalTree` only by the `range*` flags, which opt into
    /// per-node aggregate commitments for range-aggregate proofs.
    pub property_name_tree_type: TreeType,
    /// Tree type of each per-value tree. Aggregating whenever the
    /// sub-level terminates a countable and/or summable index, with
    /// the continuation demotion applied (see module docs).
    pub value_tree_type: TreeType,
}

/// Derives both tree types for an index sub-level, applying the
/// continuation demotion. This is the single source of truth for the
/// v2 walkers; the v1 tables live inline in the (consensus-frozen) v1
/// walker modules.
pub(crate) fn index_level_tree_types_with_continuation_demotion(
    sub_level: &IndexLevel,
) -> IndexLevelTreeTypes {
    let info = sub_level.has_index_with_type();
    derive_index_level_tree_types(
        info.map(|i| i.countable.is_countable()).unwrap_or(false),
        info.map(|i| i.range_countable).unwrap_or(false),
        info.map(|i| i.summable.is_some()).unwrap_or(false),
        info.map(|i| i.range_summable).unwrap_or(false),
        !sub_level.sub_levels().is_empty(),
    )
}

/// Pure derivation over the level's four terminator flags plus whether
/// continuations hang beneath its value trees. Split out so the full
/// input space is unit-testable without constructing `IndexLevel`s.
fn derive_index_level_tree_types(
    countable_terminator: bool,
    range_countable: bool,
    summable_terminator: bool,
    range_summable: bool,
    has_continuations: bool,
) -> IndexLevelTreeTypes {
    let property_name_tree_type = match (range_countable, range_summable) {
        (true, true) => TreeType::ProvableCountProvableSumTree,
        (true, false) => TreeType::ProvableCountTree,
        (false, true) => TreeType::ProvableSumTree,
        (false, false) => TreeType::NormalTree,
    };

    // Same dispatch table as the v1 walkers.
    let value_tree_type = match (
        countable_terminator,
        range_countable,
        summable_terminator,
        range_summable,
    ) {
        (true, true, true, true) => TreeType::ProvableCountProvableSumTree,
        (true, false, true, false) => TreeType::CountSumTree,
        (true, true, true, false) => TreeType::ProvableCountSumTree,
        (true, false, true, true) => TreeType::ProvableCountProvableSumTree,
        (true, _, false, false) => TreeType::CountTree,
        (false, false, true, _) => TreeType::SumTree,
        (false, _, false, _) => TreeType::NormalTree,
        _ => TreeType::NormalTree,
    };

    let value_tree_type = if has_continuations {
        match value_tree_type {
            TreeType::ProvableCountSumTree | TreeType::ProvableCountProvableSumTree => {
                TreeType::CountSumTree
            }
            other => other,
        }
    } else {
        value_tree_type
    };

    IndexLevelTreeTypes {
        property_name_tree_type,
        value_tree_type,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fees::op::LowLevelDriveOperation;

    fn all_flag_combinations() -> impl Iterator<Item = (bool, bool, bool, bool)> {
        (0u8..16).map(|bits| (bits & 1 != 0, bits & 2 != 0, bits & 4 != 0, bits & 8 != 0))
    }

    /// The load-bearing cross-module invariant: whenever continuations
    /// exist, the derived value-tree type must be accepted as a parent
    /// by the zero-contribution dispatcher for every continuation
    /// property-name tree type the derivation can produce. A future
    /// edit to either table that breaks this surfaces here instead of
    /// as a `NotSupported` insert failure at v14.
    #[test]
    fn demoted_value_trees_are_accepted_zero_contribution_parents() {
        // Every property-name (continuation) tree type the derivation
        // can produce for a child level.
        let possible_continuations = [
            TreeType::NormalTree,
            TreeType::ProvableCountTree,
            TreeType::ProvableSumTree,
            TreeType::ProvableCountProvableSumTree,
        ];

        for (countable, range_countable, summable, range_summable) in all_flag_combinations() {
            let with_continuations = derive_index_level_tree_types(
                countable,
                range_countable,
                summable,
                range_summable,
                true,
            );

            // Provable count-bearing value trees must never host
            // continuations — grovedb rejects count-suppressed
            // children under them.
            assert!(
                !matches!(
                    with_continuations.value_tree_type,
                    TreeType::ProvableCountTree
                        | TreeType::ProvableCountSumTree
                        | TreeType::ProvableCountProvableSumTree
                ),
                "flags ({countable}, {range_countable}, {summable}, {range_summable}): \
                 value tree with continuations must not be provable count-bearing, got {:?}",
                with_continuations.value_tree_type
            );

            if matches!(with_continuations.value_tree_type, TreeType::NormalTree) {
                // Non-aggregating parents take the plain insert path.
                continue;
            }
            for continuation in possible_continuations {
                LowLevelDriveOperation::for_known_path_key_empty_tree_contributing_zero_to_parent(
                    vec![b"path".to_vec()],
                    b"key".to_vec(),
                    with_continuations.value_tree_type,
                    continuation,
                    None,
                )
                .unwrap_or_else(|error| {
                    panic!(
                        "flags ({countable}, {range_countable}, {summable}, {range_summable}): \
                         dispatcher must accept parent {:?} with continuation {continuation:?}: \
                         {error}",
                        with_continuations.value_tree_type
                    )
                });
            }
        }
    }

    /// Without continuations, the derivation must match the v1
    /// walkers' (consensus-frozen) tables exactly — restated here
    /// literally as the frozen expectation.
    #[test]
    fn derivation_without_continuations_matches_v1_tables() {
        for (countable, range_countable, summable, range_summable) in all_flag_combinations() {
            let derived = derive_index_level_tree_types(
                countable,
                range_countable,
                summable,
                range_summable,
                false,
            );

            let expected_property = match (range_countable, range_summable) {
                (true, true) => TreeType::ProvableCountProvableSumTree,
                (true, false) => TreeType::ProvableCountTree,
                (false, true) => TreeType::ProvableSumTree,
                (false, false) => TreeType::NormalTree,
            };
            let expected_value = match (countable, range_countable, summable, range_summable) {
                (true, true, true, true) => TreeType::ProvableCountProvableSumTree,
                (true, false, true, false) => TreeType::CountSumTree,
                (true, true, true, false) => TreeType::ProvableCountSumTree,
                (true, false, true, true) => TreeType::ProvableCountProvableSumTree,
                (true, _, false, false) => TreeType::CountTree,
                (false, false, true, _) => TreeType::SumTree,
                (false, _, false, _) => TreeType::NormalTree,
                _ => TreeType::NormalTree,
            };

            assert_eq!(derived.property_name_tree_type, expected_property);
            assert_eq!(derived.value_tree_type, expected_value);
        }
    }
}
