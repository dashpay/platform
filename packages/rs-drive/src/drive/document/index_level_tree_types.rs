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

use crate::drive::document::ranked_index_tree_type::property_name_tree_type_and_ranked_axes;
use crate::error::Error;
use crate::util::object_size_info::DriveKeyInfo;
use dpp::data_contract::document_type::{IndexLevel, TimeRangeTransform};
use grovedb::batch::key_info::KeyInfo;
use grovedb::element::IndexAxis;
use grovedb::TreeType;

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
    let info = sub_level.has_index_with_type();
    let (property_name_tree_type, ranked_axes) = property_name_tree_type_and_ranked_axes(info)?;
    let value_tree_type = derive_value_tree_type(
        info.map(|i| i.countable.is_countable()).unwrap_or(false),
        info.map(|i| i.range_countable).unwrap_or(false),
        info.map(|i| i.summable.is_some()).unwrap_or(false),
        info.map(|i| i.range_summable).unwrap_or(false),
        !sub_level.sub_levels().is_empty(),
    );
    Ok(IndexLevelTreeTypes {
        property_name_tree_type,
        ranked_axes,
        value_tree_type,
    })
}

/// Expands a document's raw top-field key into the set of index-entry keys a
/// time-range first-property node stores it under. For a node without a
/// transform the single key passes through untouched.
///
/// Shared by the insert and delete v2 walkers (same must-not-drift contract
/// as the tree-type derivation above); the entry-key rule itself — null keeps
/// its single null entry, epoch-sliver timestamps produce no entries,
/// undecodable values keep their raw key — lives in
/// [`TimeRangeTransform::entry_keys_for_raw`], which the update walker also
/// calls.
///
/// On the estimated-cost path (`KeySize`) the real timestamp isn't available,
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
pub(crate) fn time_range_index_keys<'a>(
    transform: Option<&TimeRangeTransform>,
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

#[cfg(test)]
mod tests {
    use super::*;
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
            ranked_summable: false,
            ranked_averageable: true,
            time_range: None,
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
            ranked_summable: false,
            ranked_averageable: false,
            time_range: None,
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
    /// The estimated-cost (`KeySize`) branch of [`time_range_index_keys`] is
    /// consensus-sensitive fee math: it must emit exactly the bounded
    /// overlap count, keep every synthetic key's `max_size` untouched, and
    /// make each `unique_id` distinct — grovedb's batch structure collapses
    /// identical `(path, key)` operations, so `overlap` copies of one key
    /// would silently estimate a single bucket's cost.
    #[test]
    fn estimated_time_range_fan_out_emits_distinct_worst_case_keys() {
        use super::time_range_index_keys;
        use crate::util::object_size_info::DriveKeyInfo;
        use dpp::data_contract::document_type::TimeRangeTransform;
        use grovedb::batch::key_info::KeyInfo;

        // 6h window sliding every 2h — overlap factor 3.
        let transform = TimeRangeTransform {
            source: "$createdAt".to_string(),
            range_seconds: 21_600,
            step_seconds: 7_200,
            phase_seconds: 0,
        };
        let key = DriveKeyInfo::KeySize(KeyInfo::MaxKeySize {
            unique_id: vec![7u8; 4],
            max_size: 8,
        });

        let keys = time_range_index_keys(Some(&transform), key.clone(), 24);
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
        };
        assert_eq!(oversized.overlap_factor(), 100);
        let keys = time_range_index_keys(Some(&oversized), key, 24);
        assert_eq!(keys.len(), 24, "fan-out must clamp to the versioned cap");
    }
}
