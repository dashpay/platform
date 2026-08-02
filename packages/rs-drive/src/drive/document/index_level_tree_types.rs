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
    let countable_terminator = info.map(|i| i.countable.is_countable()).unwrap_or(false);
    let range_countable = info.map(|i| i.range_countable).unwrap_or(false);
    let summable_terminator = info.map(|i| i.summable.is_some()).unwrap_or(false);
    let range_summable = info.map(|i| i.range_summable).unwrap_or(false);

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

    let has_continuations = !sub_level.sub_levels().is_empty();
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
