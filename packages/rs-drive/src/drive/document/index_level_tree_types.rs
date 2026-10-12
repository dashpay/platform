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
//!
//! ## Division of labour with `ranked_index_tree_type`
//!
//! This module owns exactly one decision: **the value-tree type**,
//! including the continuation demotion. The **property-name tree
//! type** (and the ranking axes it must carry) is owned by
//! [`crate::drive::document::ranked_index_tree_type`], which the
//! derivation below delegates to — that resolver also serves contract
//! registration, contract update, cost estimation and the query /
//! verify side, so keeping a second copy of its dispatch table here
//! would let the write path and the read path drift.
//!
//! The two decisions live one level apart and never contend: the
//! ranked upgrade replaces a property-name tree with its *indexed
//! mirror*, while the demotion only ever rewrites a value tree hanging
//! *underneath* such a tree. A demoted `CountSumTree` value tree
//! contributes its (count, sum) to an indexed parent exactly as the
//! provable variant did, so a ranked index's secondaries keep ranking
//! correctly over a shared-prefix shape.

use crate::drive::document::ranked_index_tree_type::{
    property_name_tree_type_and_ranked_axes_for_level, ranked_chain_value_tree_type,
};
use crate::error::Error;
#[cfg(feature = "server")]
use crate::util::object_size_info::DriveKeyInfo;
#[cfg(feature = "server")]
use crate::util::object_size_info::{DocumentInfo, DocumentInfoV0Methods};
use dpp::data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters};
#[cfg(feature = "server")]
use dpp::data_contract::document_type::IndexBucketing;
use dpp::data_contract::document_type::{
    DocumentTypeRef, IndexCountability, IndexLevel, IndexLevelTypeInfo,
};
use dpp::document::{Document, DocumentV0Getters};
use dpp::platform_value::Value;
#[cfg(feature = "server")]
use grovedb::batch::key_info::KeyInfo;
use grovedb::element::IndexAxis;
use grovedb_merk::tree_type::TreeType;

/// The two tree types an index sub-level materializes: the
/// property-name tree (keys = the property's distinct values) and the
/// per-value trees underneath it (hosting the `[0]` ref-bucket plus
/// any continuation property-name trees).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct IndexLevelTreeTypes {
    /// Tree type of the property-name tree. Upgraded from `NormalTree`
    /// by the `range*` flags, which opt into per-node aggregate
    /// commitments for range-aggregate proofs, and then by the
    /// `ranked*` flags to the matching indexed mirror. Resolved by
    /// [`property_name_tree_type_and_ranked_axes`].
    pub property_name_tree_type: TreeType,
    /// The ranking axes `property_name_tree_type` must carry. Empty for
    /// every non-ranked index — i.e. for everything a pre-v14 contract
    /// can express — in which case the property-name tree is a plain
    /// (non-indexed) tree.
    pub ranked_axes: Vec<IndexAxis>,
    /// Tree type of each per-value tree. Aggregating whenever the
    /// sub-level terminates a countable and/or summable index, with
    /// the continuation demotion applied (see module docs).
    pub value_tree_type: TreeType,
}

/// Derives both tree types for an index sub-level, applying the
/// continuation demotion. This is the single source of truth for the
/// v2 walkers; the v1 tables live inline in the (consensus-frozen) v1
/// walker modules.
///
/// Fails closed when the level's ranking flags contradict its range
/// flags — see [`property_name_tree_type_and_ranked_axes`].
pub(crate) fn index_level_tree_types_with_continuation_demotion(
    sub_level: &IndexLevel,
) -> Result<IndexLevelTreeTypes, Error> {
    let (property_name_tree_type, ranked_axes) =
        property_name_tree_type_and_ranked_axes_for_level(sub_level)?;
    // A prefix-ranking chain level (a `{ at }` grouping level or a
    // propagating level below it) counts its chain continuation: the value
    // tree's count (and, on a `summableOffCountIndex` chain, its sum) IS the
    // subtree total the grouping secondaries rank by, so that continuation is inserted
    // contributing rather than zero-wrapped (see the walkers). A plain
    // sibling's branch sharing such a level (`count_exempt_branch` on the
    // CHILD level) is the one other child the validation admits — the
    // walkers insert it contributing zero, wrapped as the chain's value tree
    // needs (`zero_contribution_wrapper`: `Element::NonCounted` under a
    // `CountTree`, not counted or summed under a `CountSumTree`, and under a
    // `SumTree` not summed when it carries a sum, else unwrapped); only the
    // provable count-bearing variants reject suppressed children. No index terminates at a chain
    // level itself (the resolver above fails closed on one), so the
    // terminator-flag derivation below never applies to it.
    let value_tree_type =
        if let Some(chain_value_tree_type) = ranked_chain_value_tree_type(sub_level) {
            chain_value_tree_type
        } else {
            let info = sub_level.has_index_with_type();
            derive_value_tree_type(
                info.map(|i| i.countable.is_countable()).unwrap_or(false),
                info.map(|i| i.range_countable).unwrap_or(false),
                info.map(|i| i.summable.is_some()).unwrap_or(false),
                info.map(|i| i.range_summable).unwrap_or(false),
                !sub_level.sub_levels().is_empty(),
            )
        };
    Ok(IndexLevelTreeTypes {
        property_name_tree_type,
        ranked_axes,
        value_tree_type,
    })
}

/// Expands a document's raw top-field key into the set of index-entry keys a
/// bucketed (time- or integer-range) first-property node stores it under.
/// For a node without a grid the single key passes through untouched.
///
/// Shared by the insert and delete v2 walkers (same must-not-drift contract
/// as the tree-type derivation above); the entry-key rule itself — null keeps
/// its single null entry, epoch-sliver timestamps produce no entries,
/// undecodable values keep their raw key — lives in
/// [`IndexBucketing::entry_keys_for_raw`], which the update walker also
/// calls.
///
/// On the estimated-cost path (`KeySize`) the real value isn't available,
/// so this assumes the worst case of `overlap_factor` overlapping buckets —
/// and makes each worst-case key **distinct** by suffixing an ordinal:
/// identical `(path, key)` operations collapse inside grovedb's batch
/// structure, so `overlap` copies of one key would silently estimate a single
/// bucket's cost.
///
/// `max_overlap_factor` is the platform version's
/// `SystemLimits::max_time_range_overlap_factor` — a validated contract can
/// never exceed it, so the clamp only bounds estimation work for a transform
/// built outside validation, and reading it from the version keeps the
/// estimated fan-out in step with whatever a future protocol version allows.
#[cfg(feature = "server")]
pub(crate) fn bucket_index_keys<'a>(
    transform: Option<&IndexBucketing>,
    document_top_field: DriveKeyInfo<'a>,
    max_overlap_factor: u64,
) -> Vec<DriveKeyInfo<'a>> {
    let Some(transform) = transform else {
        return vec![document_top_field];
    };
    match &document_top_field {
        DriveKeyInfo::KeySize(key_info) => {
            // Not `clamp(1, max)`: `Ord::clamp` asserts `min <= max`, so a
            // future limits table carrying `Some(0)` would panic here.
            let overlap = transform.overlap_factor().min(max_overlap_factor).max(1) as usize;
            (0..overlap)
                .map(|ordinal| {
                    let mut key_info = key_info.clone();
                    let suffix = (ordinal as u16).to_be_bytes();
                    match &mut key_info {
                        KeyInfo::KnownKey(bytes) => bytes.extend_from_slice(&suffix),
                        KeyInfo::MaxKeySize { unique_id, .. } => {
                            unique_id.extend_from_slice(&suffix)
                        }
                    }
                    DriveKeyInfo::KeySize(key_info)
                })
                .collect()
        }
        DriveKeyInfo::Key(raw) => transform
            .entry_keys_for_raw(raw)
            .into_iter()
            .map(DriveKeyInfo::Key)
            .collect(),
        DriveKeyInfo::KeyRef(raw) => transform
            .entry_keys_for_raw(raw)
            .into_iter()
            .map(DriveKeyInfo::Key)
            .collect(),
    }
}

/// The tree type of the `0` member bucket at an index's terminal level
/// — the tree holding one member per document (stored types: references
/// keyed by document id; indexOnly types: entry items keyed by the
/// terminal property's value). Composed from the index's countability
/// and summability axes, per-axis provable vs root-only (grovedb PR
/// 670's expanded `TreeType` set) — see the dispatch commentary in
/// `add_reference_for_index_level_for_contract_operations`.
///
/// Single source of truth for the four terminal branches (insert/delete
/// × stored/indexOnly): the insert side creates the tree and the delete
/// side emits `EstimatedLayerInformation` describing it, and any drift
/// produces dry-run fees that disagree with applied fees.
///
/// Shipped generations depend on this function:
/// `add_reference_for_index_level_for_contract_operations` v0 (protocol
/// versions 1-14) and `remove_reference_for_index_level_for_contract_operations`
/// v0 (protocol versions 1-13) call it for every stored-type index terminal;
/// protocol version 14's remove-reference v1 calls it too.
/// Changing what it returns for an index shape protocol versions 1-13 can
/// declare changes the trees and fees of those versions; make such a change a
/// new versioned method instead of editing this function.
pub(crate) fn terminal_member_tree_type(index_type: &IndexLevelTypeInfo) -> TreeType {
    let count_provable = matches!(
        index_type.countable,
        IndexCountability::CountableAllowingOffset
    );
    let count_root_only =
        matches!(index_type.countable, IndexCountability::Countable) && !count_provable;
    let sum_provable = index_type.range_summable;
    let sum_root_only = index_type.summable.is_some() && !sum_provable;
    match (count_provable, count_root_only, sum_provable, sum_root_only) {
        (false, false, false, false) => TreeType::NormalTree,
        (false, true, false, false) => TreeType::CountTree,
        (true, _, false, false) => TreeType::ProvableCountTree,
        (false, false, false, true) => TreeType::SumTree,
        (false, false, true, _) => TreeType::ProvableSumTree,
        (false, true, false, true) => TreeType::CountSumTree,
        (true, _, false, true) => TreeType::ProvableCountSumTree,
        (true, _, true, _) => TreeType::ProvableCountProvableSumTree,
        (false, true, true, _) => TreeType::ProvableCountProvableSumTree,
    }
}

/// The value-tree type an index's terminal level lives inside, derived
/// from the level info's four terminator flags — the tree the `0` member
/// bucket is inserted INTO. Used by the indexOnly terminal branch's
/// stateless apply type so estimation accounts the parent's aggregate
/// bytes (a `NormalTree` claim under-counts a count-bearing value tree's
/// per-child propagation, and the bucket fan-out multiplies the gap).
/// Continuations only demote provable variants, whose stateless costs
/// match their demoted forms at this call site, so `false` is passed.
pub(crate) fn terminal_value_tree_type(index_type: &IndexLevelTypeInfo) -> TreeType {
    derive_value_tree_type(
        index_type.countable.is_countable(),
        index_type.range_countable,
        index_type.summable.is_some(),
        index_type.range_summable,
        false,
    )
}

/// Pure derivation of the value-tree type over the level's four
/// terminator flags plus whether continuations hang beneath it. Split
/// out so the full input space is unit-testable without constructing
/// `IndexLevel`s.
fn derive_value_tree_type(
    countable_terminator: bool,
    range_countable: bool,
    summable_terminator: bool,
    range_summable: bool,
    has_continuations: bool,
) -> TreeType {
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

    if has_continuations {
        match value_tree_type {
            TreeType::ProvableCountSumTree | TreeType::ProvableCountProvableSumTree => {
                TreeType::CountSumTree
            }
            other => other,
        }
    } else {
        value_tree_type
    }
}

/// The wrapper an empty continuation property-name tree is inserted in so
/// it contributes nothing to the aggregating value tree above it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ZeroContributionWrapper {
    /// `Element::NonCounted`: counted as zero by a count-bearing parent.
    NonCounted,
    /// `Element::NotSummed`: summed as zero by a sum-bearing parent.
    NotSummed,
    /// `Element::NotCountedOrSummed`: neither counted nor summed.
    NotCountedOrSummed,
}

/// Why a continuation tree cannot be made to contribute zero to its parent.
/// `LowLevelDriveOperation::for_known_path_key_empty_tree_contributing_zero_to_parent`
/// turns each into its `NotSupported` error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ZeroContributionRefusal {
    /// An indexed (ranked) tree cannot be wrapped under an aggregating parent.
    IndexedInner,
    /// An indexed tree is a property-name tree, never a value tree.
    IndexedParent,
    /// A provable count-bearing parent rejects count-suppressed children.
    ProvableCountParent,
    /// The parent does not aggregate, so no wrapper applies.
    NonAggregatingParent,
}

/// Whether the continuation property-name tree of `sub_level`, under a value
/// tree of `parent_level` whose type is `parent_value_tree_type`, is inserted
/// so it contributes zero to that value tree: the parent aggregates, and its
/// level is no prefix-ranking chain level (a grouping or propagating level of
/// a count, sum or average chain, [`IndexLevel::is_ranked_chain_level`], whose
/// value trees aggregate their chain continuation), unless `sub_level` is a
/// count-exempt branch. The entry-insert walker and the preallocation path both decide
/// with this.
pub(crate) fn continuation_contributes_zero(
    parent_value_tree_type: TreeType,
    parent_counts_continuations: bool,
    sub_level: &IndexLevel,
) -> bool {
    !matches!(parent_value_tree_type, TreeType::NormalTree)
        && (!parent_counts_continuations || sub_level.count_exempt_branch())
}

/// Whether a document without a value for `property` writes nothing under a
/// level keyed by it: an unrequired property of an indexOnly type is a skip
/// property of every index that holds it (the parser admits no other
/// optional property), so every index through the level skips the document.
/// The walkers prune such a level before reaching it
/// ([`level_reaches_entry`]); this is the defensive arm for a level reached
/// anyway, and `drive::document::layout` notes it.
pub(crate) fn index_only_level_skips_when_absent(
    document_type: DocumentTypeRef,
    property: &str,
) -> bool {
    document_type.index_only() && !document_type.required_fields().contains(property)
}

/// Whether a document takes part in an index whose skip set is `skip_set`
/// (`Index::skip_if_absent_properties`): it carries every one of those
/// properties, as `carries` reports. An index that does not skip has an empty
/// set, so every document takes part in it. The single definition the write,
/// delete and replace walkers, the indexOnly delete probes and the SDK's
/// document cost all apply.
pub(crate) fn takes_part_in_index_by<F>(skip_set: &[String], carries: &mut F) -> Result<bool, Error>
where
    F: FnMut(&str) -> Result<bool, Error>,
{
    for skip_property in skip_set {
        if !carries(skip_property)? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Whether the document writes an entry at or below `level`: the index ending
/// at `level` takes part ([`takes_part_in_index_by`]), or a level below
/// reaches such an entry.
///
/// The walkers build a level (its property-name tree and value tree) only
/// when this holds, so an index that skips a document leaves no tree of its
/// own behind, while a level it shares with an index the document takes part
/// in is built as usual. A level no skipIfAbsent index passes through
/// ([`IndexLevel::skip_at_or_below`]) reaches an entry for every document, so
/// it is answered without looking at the document; that is every level of a
/// document type without a skipping index.
pub(crate) fn level_reaches_entry_by<F>(level: &IndexLevel, carries: &mut F) -> Result<bool, Error>
where
    F: FnMut(&str) -> Result<bool, Error>,
{
    if !level.skip_at_or_below() {
        return Ok(true);
    }
    if let Some(index_type) = level.has_index_with_type() {
        if takes_part_in_index_by(&index_type.skip_if_absent_properties, carries)? {
            return Ok(true);
        }
    }
    for sub_level in level.sub_levels().values() {
        if level_reaches_entry_by(sub_level, carries)? {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Whether `document` carries `property`, a skip property: it holds a value
/// for it other than null. A skip property is a top-level, non-system property
/// or a derived index property (the parser refuses any other), so its presence
/// is a single lookup. Drive puts a derived value into the document before
/// keying it (`derived_index_values`), null where the reference or the field
/// it reads is absent, and that null is absent as a missing property is: it
/// would be keyed under the same empty key.
pub(crate) fn document_carries(document: &Document, property: &str) -> bool {
    !matches!(
        document.properties().get(property),
        None | Some(Value::Null)
    )
}

/// Whether `document_info` carries `property` ([`document_carries`]). A
/// worst-case size carries every property: estimation then walks every index,
/// which keeps it an upper bound. A create's dry run reads the real document,
/// so it skips where the apply does, though never on a derived value it does
/// not read, which is a placeholder of the field's type.
#[cfg(feature = "server")]
fn document_info_carries(document_info: &DocumentInfo, property: &str) -> bool {
    match document_info.get_borrowed_document() {
        Some(document) => document_carries(document, property),
        None => true,
    }
}

/// [`takes_part_in_index_by`] for the document of `document_info`.
#[cfg(feature = "server")]
pub(crate) fn document_takes_part_in_index(
    skip_set: &[String],
    document_info: &DocumentInfo,
) -> Result<bool, Error> {
    takes_part_in_index_by(skip_set, &mut |property| {
        Ok(document_info_carries(document_info, property))
    })
}

/// [`level_reaches_entry_by`] for the document of `document_info`.
#[cfg(feature = "server")]
pub(crate) fn level_reaches_entry(
    level: &IndexLevel,
    document_info: &DocumentInfo,
) -> Result<bool, Error> {
    level_reaches_entry_by(level, &mut |property| {
        Ok(document_info_carries(document_info, property))
    })
}

/// Whether a delete of the document of `document_info` removes an entry at or
/// below `level`: an index ending there that neither skipped the document nor
/// outlives its delete ([`IndexLevelTypeInfo::outlives_delete`]), or a level
/// below reaching one. The delete walkers descend only where this holds, so
/// they leave the entries of an index that outlives the delete, and never
/// read the values only such an index is keyed by (a delete does not carry
/// them). A level no such index passes through is answered by
/// [`level_reaches_entry`], every level of a document type without one, and a
/// level only such indexes pass through by its stamps alone; only a level both
/// kinds share is walked.
#[cfg(feature = "server")]
pub(crate) fn level_removes_entry(
    level: &IndexLevel,
    document_info: &DocumentInfo,
) -> Result<bool, Error> {
    if !level.outlives_delete_at_or_below() {
        return level_reaches_entry(level, document_info);
    }
    // Only indexes whose entries outlive the delete pass here
    if !level.cleared_on_delete_at_or_below() {
        return Ok(false);
    }
    if let Some(index_type) = level.has_index_with_type() {
        if !index_type.outlives_delete
            && document_takes_part_in_index(&index_type.skip_if_absent_properties, document_info)?
        {
            return Ok(true);
        }
    }
    for sub_level in level.sub_levels().values() {
        if level_removes_entry(sub_level, document_info)? {
            return Ok(true);
        }
    }
    Ok(false)
}

/// The wrapper that makes an empty `inner_tree_type` tree contribute zero to
/// an `aggregating_parent_tree_type` parent, or `None` when it contributes
/// zero unwrapped (a non-sum tree under a sum-only parent):
/// - a `CountTree` parent: `NonCounted`, whatever the inner;
/// - a `CountSumTree` parent: `NotCountedOrSummed` for a sum-bearing inner,
///   else `NonCounted`;
/// - a `SumTree`, `BigSumTree` or `ProvableSumTree` parent: `NotSummed` for a
///   sum-bearing inner, else no wrapper.
///
/// An indexed inner, an indexed or provable count-bearing parent, and a
/// parent that does not aggregate are refused.
pub(crate) fn zero_contribution_wrapper(
    aggregating_parent_tree_type: TreeType,
    inner_tree_type: TreeType,
) -> Result<Option<ZeroContributionWrapper>, ZeroContributionRefusal> {
    if matches!(
        inner_tree_type,
        TreeType::ProvableSumIndexedTree
            | TreeType::ProvableCountIndexedTree
            | TreeType::ProvableCountProvableSumIndexedTree
    ) {
        return Err(ZeroContributionRefusal::IndexedInner);
    }
    let inner_is_sum_bearing = matches!(
        inner_tree_type,
        TreeType::SumTree
            | TreeType::BigSumTree
            | TreeType::ProvableSumTree
            | TreeType::CountSumTree
            | TreeType::ProvableCountSumTree
            | TreeType::ProvableCountProvableSumTree
    );
    match aggregating_parent_tree_type {
        TreeType::CountTree => Ok(Some(ZeroContributionWrapper::NonCounted)),
        TreeType::CountSumTree => Ok(Some(if inner_is_sum_bearing {
            ZeroContributionWrapper::NotCountedOrSummed
        } else {
            ZeroContributionWrapper::NonCounted
        })),
        TreeType::SumTree | TreeType::BigSumTree | TreeType::ProvableSumTree => {
            Ok(inner_is_sum_bearing.then_some(ZeroContributionWrapper::NotSummed))
        }
        TreeType::ProvableCountIndexedTree
        | TreeType::ProvableSumIndexedTree
        | TreeType::ProvableCountProvableSumIndexedTree => {
            Err(ZeroContributionRefusal::IndexedParent)
        }
        TreeType::ProvableCountTree
        | TreeType::ProvableCountSumTree
        | TreeType::ProvableCountProvableSumTree => {
            Err(ZeroContributionRefusal::ProvableCountParent)
        }
        _ => Err(ZeroContributionRefusal::NonAggregatingParent),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::document::ranked_index_tree_type::property_name_tree_type_and_ranked_axes;
    use crate::fees::op::LowLevelDriveOperation;
    use dpp::data_contract::document_type::{IndexCountability, IndexLevelTypeInfo, IndexType};

    fn all_flag_combinations() -> impl Iterator<Item = (bool, bool, bool, bool)> {
        (0u8..16).map(|bits| (bits & 1 != 0, bits & 2 != 0, bits & 4 != 0, bits & 8 != 0))
    }

    /// A terminator level carrying the four aggregate flags and no
    /// ranking axis — the only shape a pre-v14 contract can express.
    fn unranked_terminator_info(
        countable: bool,
        range_countable: bool,
        summable: bool,
        range_summable: bool,
    ) -> IndexLevelTypeInfo {
        IndexLevelTypeInfo {
            should_insert_with_all_null: false,
            index_type: IndexType::NonUniqueIndex,
            countable: if countable {
                IndexCountability::Countable
            } else {
                IndexCountability::NotCountable
            },
            range_countable,
            summable: summable.then(|| "score".to_string()),
            range_summable,
            ranked_countable: false,
            ranked_summable: false,
            ranked_averageable: false,
            terminal: None,
            preallocated: false,
            outlives_delete: false,
            flat: false,
            skip_if_absent_properties: Vec::new(),
            summable_off_count_index: None,
        }
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
            let with_continuations =
                derive_value_tree_type(countable, range_countable, summable, range_summable, true);

            // Provable count-bearing value trees must never host
            // continuations — grovedb rejects count-suppressed
            // children under them.
            assert!(
                !matches!(
                    with_continuations,
                    TreeType::ProvableCountTree
                        | TreeType::ProvableCountSumTree
                        | TreeType::ProvableCountProvableSumTree
                ),
                "flags ({countable}, {range_countable}, {summable}, {range_summable}): \
                 value tree with continuations must not be provable count-bearing, got \
                 {with_continuations:?}"
            );

            if matches!(with_continuations, TreeType::NormalTree) {
                // Non-aggregating parents take the plain insert path.
                continue;
            }
            for continuation in possible_continuations {
                LowLevelDriveOperation::for_known_path_key_empty_tree_contributing_zero_to_parent(
                    vec![b"path".to_vec()],
                    b"key".to_vec(),
                    with_continuations,
                    continuation,
                    None,
                )
                .unwrap_or_else(|error| {
                    panic!(
                        "flags ({countable}, {range_countable}, {summable}, {range_summable}): \
                         dispatcher must accept parent {with_continuations:?} with continuation \
                         {continuation:?}: {error}"
                    )
                });
            }
        }
    }

    /// Without continuations, the derivation must match the v1
    /// walkers' (consensus-frozen) tables exactly — restated here
    /// literally as the frozen expectation. The property-name half now
    /// resolves through `ranked_index_tree_type`, so this also pins
    /// that an unranked level keeps its pre-v14 property-name type.
    #[test]
    fn derivation_without_continuations_matches_v1_tables() {
        for (countable, range_countable, summable, range_summable) in all_flag_combinations() {
            let derived_value =
                derive_value_tree_type(countable, range_countable, summable, range_summable, false);
            let (derived_property, ranked_axes) = property_name_tree_type_and_ranked_axes(Some(
                &unranked_terminator_info(countable, range_countable, summable, range_summable),
            ))
            .expect("unranked resolution must succeed");
            assert!(ranked_axes.is_empty());

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

            assert_eq!(derived_property, expected_property);
            assert_eq!(derived_value, expected_value);
        }
    }

    /// A `rankedCountable: { at }` index resolves its chain levels to the
    /// documented tree types: the grouping level to the Count-axis indexed
    /// tree over `CountTree` value trees, the propagating level to a
    /// `CountTree` pair, and the terminal level to its unchanged
    /// terminator-flag derivation.
    #[test]
    fn prefix_ranked_chain_levels_resolve_to_the_chain_tree_types() {
        use dpp::data_contract::document_type::{Index, IndexProperty};
        use dpp::version::PlatformVersion;
        use grovedb::element::IndexAxis;

        let prefix_ranked = Index {
            name: "byHashtagRegionPost".to_string(),
            properties: ["hashtag", "region", "postId"]
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
            ranked_countable_at: vec!["hashtag".to_string()],
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
        };

        let index_structure =
            IndexLevel::try_from_indices([&prefix_ranked], "like", PlatformVersion::latest())
                .expect("index level must build");

        let hashtag_level = index_structure
            .sub_levels()
            .get("hashtag")
            .expect("the grouping level exists");
        let hashtag_types = index_level_tree_types_with_continuation_demotion(hashtag_level)
            .expect("grouping resolution must succeed");
        assert_eq!(
            hashtag_types,
            IndexLevelTreeTypes {
                property_name_tree_type: TreeType::ProvableCountIndexedTree,
                ranked_axes: vec![IndexAxis::Count],
                value_tree_type: TreeType::CountTree,
            }
        );

        let region_level = hashtag_level
            .sub_levels()
            .get("region")
            .expect("the propagating level exists");
        let region_types = index_level_tree_types_with_continuation_demotion(region_level)
            .expect("propagating resolution must succeed");
        assert_eq!(
            region_types,
            IndexLevelTreeTypes {
                property_name_tree_type: TreeType::CountTree,
                ranked_axes: Vec::new(),
                value_tree_type: TreeType::CountTree,
            }
        );

        let post_level = region_level
            .sub_levels()
            .get("postId")
            .expect("the terminal level exists");
        let post_types = index_level_tree_types_with_continuation_demotion(post_level)
            .expect("terminal resolution must succeed");
        assert_eq!(
            post_types,
            IndexLevelTreeTypes {
                property_name_tree_type: TreeType::ProvableCountTree,
                ranked_axes: Vec::new(),
                value_tree_type: TreeType::CountTree,
            },
            "the terminal keeps its rangeCountable derivation — the boolean ranked axis is off"
        );
    }

    /// A plain sibling admitted beside a `rankedCountable: { at }` chain
    /// resolves its branch level to plain trees, and the chain's
    /// `CountTree` value trees accept it as a zero-contributing
    /// (`Element::NonCounted`-wrapped) child — the layout the walkers
    /// create for the count-exempt sibling shape.
    #[test]
    fn a_count_exempt_sibling_branch_resolves_plain_and_wraps_under_the_chain() {
        use dpp::data_contract::document_type::{Index, IndexProperty};
        use dpp::version::PlatformVersion;

        let base = |name: &str, properties: &[&str]| Index {
            name: name.to_string(),
            properties: properties
                .iter()
                .map(|property| IndexProperty {
                    name: property.to_string(),
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
        };
        let mut ranked = base("byAuthorPost", &["postAuthor", "postId"]);
        ranked.countable = IndexCountability::Countable;
        ranked.range_countable = true;
        ranked.ranked_countable_at = vec!["postAuthor".to_string()];
        let sibling = base("byAuthorTimePost", &["postAuthor", "day", "postId"]);

        let index_structure =
            IndexLevel::try_from_indices([&ranked, &sibling], "like", PlatformVersion::latest())
                .expect("index level must build");

        let author_level = index_structure
            .sub_levels()
            .get("postAuthor")
            .expect("the grouping level exists");
        let author_types = index_level_tree_types_with_continuation_demotion(author_level)
            .expect("grouping resolution must succeed");
        assert_eq!(
            author_types.value_tree_type,
            TreeType::CountTree,
            "the chain's value trees stay count-bearing with the sibling present"
        );

        // The chain continuation is not exempt; the sibling branch is.
        let chain_child = author_level
            .sub_levels()
            .get("postId")
            .expect("the chain continuation exists");
        assert!(!chain_child.count_exempt_branch());
        let branch = author_level
            .sub_levels()
            .get("day")
            .expect("the sibling branch exists");
        assert!(branch.count_exempt_branch());

        // The branch resolves to plain trees…
        let branch_types = index_level_tree_types_with_continuation_demotion(branch)
            .expect("branch resolution must succeed");
        assert_eq!(
            branch_types,
            IndexLevelTreeTypes {
                property_name_tree_type: TreeType::NormalTree,
                ranked_axes: Vec::new(),
                value_tree_type: TreeType::NormalTree,
            }
        );

        // …and the zero-contribution dispatcher wraps it under the chain's
        // CountTree value trees (NonCounted around a plain tree).
        LowLevelDriveOperation::for_known_path_key_empty_tree_contributing_zero_to_parent(
            vec![b"path".to_vec()],
            b"day".to_vec(),
            author_types.value_tree_type,
            branch_types.property_name_tree_type,
            None,
        )
        .expect("the sibling branch must be insertable zero-wrapped under the chain value tree");
    }

    /// A grouping level that also carries a terminator stamp has two
    /// contradictory layouts; the resolver must fail closed instead of
    /// picking one. Reachable only through an index set the contract-level
    /// structural validation rejects (`try_from_indices` alone does not run
    /// it), which is exactly why the resolver keeps its own guard.
    #[test]
    fn a_grouping_level_with_a_terminator_fails_closed() {
        use dpp::data_contract::document_type::{Index, IndexProperty};
        use dpp::version::PlatformVersion;

        let base = |name: &str, properties: &[&str]| Index {
            name: name.to_string(),
            properties: properties
                .iter()
                .map(|property| IndexProperty {
                    name: property.to_string(),
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
        };
        let mut prefix_ranked = base("byHashtagPost", &["hashtag", "postId"]);
        prefix_ranked.ranked_countable_at = vec!["hashtag".to_string()];
        let terminating = base("byHashtag", &["hashtag"]);

        let index_structure = IndexLevel::try_from_indices(
            [&prefix_ranked, &terminating],
            "like",
            PlatformVersion::latest(),
        )
        .expect("index level must build — the overlap rule runs at contract parse, not here");

        let hashtag_level = index_structure
            .sub_levels()
            .get("hashtag")
            .expect("the shared level exists");
        assert!(
            index_level_tree_types_with_continuation_demotion(hashtag_level).is_err(),
            "a grouping level with a terminator stamp must fail closed"
        );

        // Same guard one level down: a COUNT-PROPAGATING level that also
        // carries a terminator stamp (an index terminating inside another
        // index's ranked chain — rejected by contract validation, but
        // constructible by hand) has the same two contradictory layouts.
        let mut chain = base("byHashtagRegionPost", &["hashtag", "region", "postId"]);
        chain.ranked_countable_at = vec!["hashtag".to_string()];
        let terminating_inside = base("byHashtagRegion", &["hashtag", "region"]);
        let index_structure = IndexLevel::try_from_indices(
            [&chain, &terminating_inside],
            "like",
            PlatformVersion::latest(),
        )
        .expect("index level must build — the overlap rule runs at contract parse, not here");
        let region_level = index_structure
            .sub_levels()
            .get("hashtag")
            .and_then(|level| level.sub_levels().get("region"))
            .expect("the shared propagating level exists");
        assert!(
            index_level_tree_types_with_continuation_demotion(region_level).is_err(),
            "a count-propagating level with a terminator stamp must fail closed"
        );
    }

    /// The two v14 fixes on one level: a ranked index terminating at
    /// `restaurantId` next to a compound index that continues below it.
    /// The ranking upgrades the property-name tree to its indexed
    /// mirror, and — independently, one level down — the continuation
    /// demotes the value trees to `CountSumTree`. Both must happen; a
    /// change that let one suppress the other would either lose the
    /// ranked secondaries or re-break document inserts on this shape.
    #[test]
    fn ranked_terminator_with_a_compound_continuation_gets_both_treatments() {
        use dpp::data_contract::document_type::{Index, IndexProperty};
        use dpp::version::PlatformVersion;
        use grovedb::element::IndexAxis;

        let ranked = Index {
            name: "byRestaurant".to_string(),
            properties: vec![IndexProperty {
                name: "restaurantId".to_string(),
                ascending: true,
            }],
            unique: false,
            null_searchable: true,
            contested_index: None,
            countable: IndexCountability::Countable,
            range_countable: true,
            summable: Some("grade".to_string()),
            range_summable: true,
            ranked_countable: false,
            ranked_countable_at: vec![],
            ranked_summable_at: Vec::new(),
            ranked_averageable_at: Vec::new(),
            ranked_summable: false,
            ranked_averageable: true,
            time_range: None,
            integer_range: None,
            terminal: None,
            preallocated: false,
            outlives_delete: false,
            skip_if_absent: false,
            skip_if_absent_properties: Vec::new(),
            summable_off_count_index: None,
        };
        let compound = Index {
            name: "byRestaurantChef".to_string(),
            properties: vec![
                IndexProperty {
                    name: "restaurantId".to_string(),
                    ascending: true,
                },
                IndexProperty {
                    name: "chefId".to_string(),
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
        };

        let index_structure =
            IndexLevel::try_from_indices([&ranked, &compound], "dish", PlatformVersion::latest())
                .expect("index level must build");
        let restaurant_level = index_structure
            .sub_levels()
            .get("restaurantId")
            .expect("the shared prefix level exists");

        let tree_types = index_level_tree_types_with_continuation_demotion(restaurant_level)
            .expect("resolution must succeed");

        assert_eq!(
            tree_types.property_name_tree_type,
            TreeType::ProvableCountProvableSumIndexedTree,
            "the ranking upgrade must survive the presence of a continuation"
        );
        assert_eq!(tree_types.ranked_axes, vec![IndexAxis::Avg]);
        assert_eq!(
            tree_types.value_tree_type,
            TreeType::CountSumTree,
            "the continuation must still demote the value trees under the indexed parent"
        );

        // And the demoted value tree is a legal parent for the plain
        // `chefId` continuation the compound index hangs beneath it.
        let chef_level = restaurant_level
            .sub_levels()
            .get("chefId")
            .expect("the continuation level exists");
        let chef_tree_types =
            index_level_tree_types_with_continuation_demotion(chef_level).expect("resolution");
        LowLevelDriveOperation::for_known_path_key_empty_tree_contributing_zero_to_parent(
            vec![b"path".to_vec()],
            b"chefId".to_vec(),
            tree_types.value_tree_type,
            chef_tree_types.property_name_tree_type,
            None,
        )
        .expect("the continuation must be insertable under the demoted value tree");
    }
    /// The estimated-cost (`KeySize`) branch of [`bucket_index_keys`] is
    /// consensus-sensitive fee math: it must emit exactly the bounded
    /// overlap count, keep every synthetic key's `max_size` untouched, and
    /// make each `unique_id` distinct — grovedb's batch structure collapses
    /// identical `(path, key)` operations, so `overlap` copies of one key
    /// would silently estimate a single bucket's cost.
    #[test]
    fn estimated_time_range_fan_out_emits_distinct_worst_case_keys() {
        use super::bucket_index_keys;
        use crate::util::object_size_info::DriveKeyInfo;
        use dpp::data_contract::document_type::TimeRangeTransform;
        use grovedb::batch::key_info::KeyInfo;

        // 6h window sliding every 2h — overlap factor 3.
        let transform = TimeRangeTransform {
            source: "$createdAt".to_string(),
            range_seconds: 21_600,
            step_seconds: 7_200,
            phase_seconds: 0,
            ttl_seconds: None,
        };
        let key = DriveKeyInfo::KeySize(KeyInfo::MaxKeySize {
            unique_id: vec![7u8; 4],
            max_size: 8,
        });

        let keys = bucket_index_keys(Some(&transform.into()), key.clone(), 24);
        assert_eq!(keys.len(), 3, "one worst-case key per overlapping bucket");
        let mut unique_ids = Vec::new();
        for entry in &keys {
            let DriveKeyInfo::KeySize(KeyInfo::MaxKeySize {
                unique_id,
                max_size,
            }) = entry
            else {
                panic!("the KeySize branch must stay on the estimation path");
            };
            assert_eq!(
                *max_size, 8,
                "the ordinal suffix disambiguates unique_id only; the estimated \
                 key size must be the timestamp key's"
            );
            unique_ids.push(unique_id.clone());
        }
        let distinct: std::collections::BTreeSet<_> = unique_ids.iter().collect();
        assert_eq!(
            distinct.len(),
            3,
            "identical unique_ids would collapse in the batch and under-estimate"
        );

        // An unvalidated transform above the version's cap is clamped: the
        // estimation work stays bounded by what the protocol version allows.
        let oversized = TimeRangeTransform {
            source: "$createdAt".to_string(),
            range_seconds: 100 * 3_600,
            step_seconds: 3_600,
            phase_seconds: 0,
            ttl_seconds: None,
        };
        assert_eq!(oversized.overlap_factor(), 100);
        let keys = bucket_index_keys(Some(&oversized.into()), key, 24);
        assert_eq!(keys.len(), 24, "fan-out must clamp to the versioned cap");
    }
}
