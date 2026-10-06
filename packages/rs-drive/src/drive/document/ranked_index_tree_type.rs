//! Terminal property-name tree resolution for indexes that declare ranking
//! axes (`rankedCountable` / `rankedSummable` / `rankedAverageable`, meta
//! schema v3 / PV14).
//!
//! An index's **terminal property-name tree** is the level whose children are
//! the last index property's value trees — one child per distinct value, i.e.
//! one child per *group*. Without ranking flags its `TreeType` is picked
//! purely from `(range_countable, range_summable)`:
//!
//! | range_countable | range_summable | tree type                      |
//! |-----------------|----------------|--------------------------------|
//! | `true`          | `true`         | `ProvableCountProvableSumTree` |
//! | `true`          | `false`        | `ProvableCountTree`            |
//! | `false`         | `true`         | `ProvableSumTree`              |
//! | `false`         | `false`        | `NormalTree`                   |
//!
//! A ranking flag upgrades that tree to the matching *indexed* variant
//! (grovedb PR 657): a primary Merk that is a byte-compatible mirror of the
//! tree it replaces, plus one ordered secondary Merk per declared axis.
//! Per-value range reads keep working against the primary; a range total
//! does not (grovedb's range aggregates take provable trees only and do not
//! prove through an indexed tree), so the query surfaces refuse one through
//! any index whose path passes through a ranked level. The secondaries are what make "top / bottom
//! K groups by count / sum / average" O(log n + k) with a proof.
//!
//! The upgrade table (`axes` = the declared ranking axes, canonically sorted
//! Count < Sum < Avg):
//!
//! | base                           | axes         | indexed tree                          |
//! |--------------------------------|--------------|---------------------------------------|
//! | any                            | `[]`         | unchanged (no ranking declared)       |
//! | `ProvableCountTree`            | `[Count]`    | `ProvableCountIndexedTree`            |
//! | `ProvableSumTree`              | `[Sum]`      | `ProvableSumIndexedTree`              |
//! | `ProvableCountProvableSumTree` | any non-empty| `ProvableCountProvableSumIndexedTree` |
//!
//! The single-axis PCIT / PSIT variants carry no axis list on the element at
//! all (their one secondary is implied by the variant), which is why they are
//! only reachable when the *base* layout is already single-axis. An index that
//! declares, say, `rankedCountable` alongside `rangeSummable` still lays out
//! as PCPS underneath, so it upgrades to the multi-axis PCPSIT carrying just
//! the Count axis in its TLV.
//!
//! Every rs-drive site that needs a terminal property-name tree type — contract
//! registration, the document insert / update / delete index walkers, and the
//! cost-estimation layers — routes through
//! [`property_name_tree_type_and_ranked_axes`] so the layouts they describe
//! cannot drift apart.

use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::data_contract::document_type::{IndexLevel, IndexLevelTypeInfo};
use grovedb::element::IndexAxis;
use grovedb_merk::tree_type::TreeType;

/// The ranking axes an index level declares, in grovedb's canonical TLV order
/// (Count < Sum < Avg, no duplicates).
///
/// The three flags are independent opt-ins — `ranked_averageable` alone is a
/// legal (and useful) declaration, and does **not** imply the other two.
/// Levels that terminate no index, and every non-terminal prefix level, yield
/// an empty set: rs-dpp only ever populates the ranking flags on the level of
/// an index's last property.
pub(crate) fn ranked_axes_for_index_level_info(
    index_level_info: Option<&IndexLevelTypeInfo>,
) -> Vec<IndexAxis> {
    let Some(info) = index_level_info else {
        return Vec::new();
    };
    ranked_axes(
        info.ranked_countable,
        info.ranked_summable,
        info.ranked_averageable,
    )
}

/// The axes `count`, `sum` and `avg` name, pushed in ascending tag order so
/// the list is already the canonical TLV order `Element::validate_pcpsit_axes`
/// demands.
fn ranked_axes(count: bool, sum: bool, avg: bool) -> Vec<IndexAxis> {
    [
        (count, IndexAxis::Count),
        (sum, IndexAxis::Sum),
        (avg, IndexAxis::Avg),
    ]
    .into_iter()
    .filter_map(|(ranked, axis)| ranked.then_some(axis))
    .collect()
}

/// The provable tree carrying `counts` and `sums`: the range-flag-selected
/// base of a terminal property-name tree, and the base a ranked chain level's
/// indexed tree mirrors. `NormalTree` when it carries neither.
fn provable_tree_type(counts: bool, sums: bool) -> TreeType {
    match (counts, sums) {
        (true, true) => TreeType::ProvableCountProvableSumTree,
        (true, false) => TreeType::ProvableCountTree,
        (false, true) => TreeType::ProvableSumTree,
        (false, false) => TreeType::NormalTree,
    }
}

/// Render a canonical axis list as the `(tag, secondary_root_key)` TLV a
/// freshly created `Element::ProvableCountProvableSumIndexedTree` carries.
/// Every secondary starts empty, hence `None` for each root key.
pub(crate) fn ranked_axes_tlv(ranked_axes: &[IndexAxis]) -> Vec<(u8, Option<Vec<u8>>)> {
    ranked_axes.iter().map(|axis| (axis.tag(), None)).collect()
}

/// Upgrade the range-flag-selected `base` tree type to its indexed mirror for
/// `ranked_axes`. Returns `base` unchanged when no ranking axis is declared,
/// which is every contract written before meta schema v3.
///
/// The `_ =>` arm is unreachable given rs-dpp's parse-time invariants
/// (`ranked_countable ⇒ range_countable`, `ranked_summable ⇒ range_summable`,
/// `ranked_averageable ⇒ both`), which force `base` to be provable on every
/// axis a ranking flag names. It is kept as a typed error rather than a
/// silent fallback so a future grammar change that breaks one of those
/// implications surfaces here instead of silently laying down a tree whose
/// secondaries nothing maintains.
pub(crate) fn ranked_property_name_tree_type(
    base: TreeType,
    ranked_axes: &[IndexAxis],
) -> Result<TreeType, Error> {
    if ranked_axes.is_empty() {
        return Ok(base);
    }
    match (base, ranked_axes) {
        (TreeType::ProvableCountTree, [IndexAxis::Count]) => Ok(TreeType::ProvableCountIndexedTree),
        (TreeType::ProvableSumTree, [IndexAxis::Sum]) => Ok(TreeType::ProvableSumIndexedTree),
        (TreeType::ProvableCountProvableSumTree, _) => {
            Ok(TreeType::ProvableCountProvableSumIndexedTree)
        }
        _ => Err(Error::Drive(DriveError::CorruptedContractIndexes(format!(
            "index declares ranking axes {:?} but its range flags lay the terminal \
             property-name tree out as {:?}; every ranked axis requires the matching \
             range axis (rankedCountable ⇒ rangeCountable, rankedSummable ⇒ \
             rangeSummable, rankedAverageable ⇒ both)",
            ranked_axes, base
        )))),
    }
}

/// One-stop resolution of a terminal property-name tree: the `TreeType` to
/// create plus the ranking axes it must carry (empty for every non-ranked
/// index).
///
/// Callers pass the `has_index_with_type()` of the level *named after the
/// property* — `None` for pure prefix levels, which resolve to
/// `(NormalTree, [])`.
///
/// Shipped generations depend on this function through
/// [`property_name_tree_type_and_ranked_axes_for_level`]: see the note there
/// before changing what it returns.
pub(crate) fn property_name_tree_type_and_ranked_axes(
    index_level_info: Option<&IndexLevelTypeInfo>,
) -> Result<(TreeType, Vec<IndexAxis>), Error> {
    let range_countable = index_level_info
        .map(|info| info.range_countable)
        .unwrap_or(false);
    let range_summable = index_level_info
        .map(|info| info.range_summable)
        .unwrap_or(false);
    let base = provable_tree_type(range_countable, range_summable);
    let ranked_axes = ranked_axes_for_index_level_info(index_level_info);
    Ok((
        ranked_property_name_tree_type(base, &ranked_axes)?,
        ranked_axes,
    ))
}

/// Level-aware form of [`property_name_tree_type_and_ranked_axes`], covering
/// the prefix-level rankings (`rankedCountable: { at }`, and on a
/// `summableOffCountIndex` index `rankedSummable: { at }` and
/// `rankedAverageable: { at }`, meta-schema v3):
///
/// - A **grouping** level (`ranked_count_grouping`, `ranked_sum_grouping`,
///   `ranked_average_grouping`) hosts the rankings itself: its property-name
///   tree is the indexed tree carrying those axes, whose secondaries rank the
///   property's values by each value tree's whole-subtree aggregate. A level
///   carrying only counts is a `ProvableCountIndexedTree`, only sums a
///   `ProvableSumIndexedTree`, and both a `ProvableCountProvableSumIndexedTree`.
/// - A **propagating** level (`count_propagating` / `sum_propagating`,
///   strictly between the shallowest grouping level and its index's terminal)
///   gets a `CountTree`, `SumTree` or `CountSumTree` property-name tree, for
///   what it carries, so the subtree aggregates flow through it toward the
///   grouping secondaries.
/// - Every other level resolves through the terminator-info path unchanged.
///
/// rs-dpp's structural validation guarantees no index terminates at a
/// grouping or propagating level; both fail closed here on a stamped
/// terminator rather than pick one of two contradictory layouts.
///
/// Shipped generations depend on this function: the `insert_contract` v0 and
/// `update_contract` v0 operations call it to choose the tree type of every
/// top-level index level they create, and every later generation of both
/// composes those operations, so every protocol version reaches it. Changing what it returns
/// for an index level protocol versions 1-13 can declare changes the trees
/// and fees of those versions; make such a change a new versioned method
/// instead of editing this function. (The Sum and Avg chains are reachable
/// only through `summableOffCountIndex`, which protocol version 14 brings, so
/// their arms change nothing earlier versions can declare.)
pub(crate) fn property_name_tree_type_and_ranked_axes_for_level(
    level: &IndexLevel,
) -> Result<(TreeType, Vec<IndexAxis>), Error> {
    // A level on a prefix-ranking chain (`is_ranked_chain_level`): the tree
    // its values get, aggregating what the chain carries.
    if let Some(value_tree_type) = ranked_chain_value_tree_type(level) {
        if level.has_index_with_type().is_some() {
            return Err(Error::Drive(DriveError::CorruptedContractIndexes(
                "a prefix-ranking (grouping or propagating) index level cannot also \
                 terminate an index; contract validation rejects every shape that shares \
                 such a level"
                    .to_string(),
            )));
        }
        let axes = ranked_axes(
            level.ranked_count_grouping(),
            level.ranked_sum_grouping(),
            level.ranked_average_grouping(),
        );
        // A propagating level's tree is the tree its values get; a grouping
        // level's is the indexed mirror of the provable tree carrying the
        // same aggregates, for the axes ranked at it.
        if axes.is_empty() {
            return Ok((value_tree_type, axes));
        }
        let provable_base =
            provable_tree_type(level.chain_carries_counts(), level.chain_carries_sums());
        return Ok((ranked_property_name_tree_type(provable_base, &axes)?, axes));
    }
    property_name_tree_type_and_ranked_axes(level.has_index_with_type())
}

/// The value-tree type of a prefix-ranking chain level: the tree each of the
/// level's values gets, aggregating what the chain carries up to its grouping
/// level. A count chain's value trees are `CountTree`s, a sum chain's (only a
/// `summableOffCountIndex` index has one) `SumTree`s and a chain carrying both
/// `CountSumTree`s. `None` for a level off every chain.
pub(crate) fn ranked_chain_value_tree_type(level: &IndexLevel) -> Option<TreeType> {
    match (level.chain_carries_counts(), level.chain_carries_sums()) {
        (true, false) => Some(TreeType::CountTree),
        (false, true) => Some(TreeType::SumTree),
        (true, true) => Some(TreeType::CountSumTree),
        (false, false) => None,
    }
}

/// The non-indexed tree type an indexed tree mirrors, or `tree_type` itself
/// when it isn't indexed.
///
/// grovedb's indexed primaries reuse the node shape (and therefore the
/// per-node aggregate byte cost) of the tree they replace, so every place that
/// only cares about "how expensive is a node of this tree" can collapse an
/// indexed tree onto its mirror. The extra bytes an indexed tree costs live on
/// the *parent's* element (one length byte plus 33 per axis for the secondary
/// root keys / TLV), not on its nodes.
pub(crate) fn non_indexed_mirror_tree_type(tree_type: TreeType) -> TreeType {
    match tree_type {
        TreeType::ProvableCountIndexedTree => TreeType::ProvableCountTree,
        TreeType::ProvableSumIndexedTree => TreeType::ProvableSumTree,
        TreeType::ProvableCountProvableSumIndexedTree => TreeType::ProvableCountProvableSumTree,
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::data_contract::document_type::{IndexCountability, IndexType};

    fn info(
        range_countable: bool,
        range_summable: bool,
        ranked_countable: bool,
        ranked_summable: bool,
        ranked_averageable: bool,
    ) -> IndexLevelTypeInfo {
        IndexLevelTypeInfo {
            should_insert_with_all_null: false,
            index_type: IndexType::NonUniqueIndex,
            countable: if range_countable {
                IndexCountability::Countable
            } else {
                IndexCountability::NotCountable
            },
            range_countable,
            summable: range_summable.then(|| "score".to_string()),
            range_summable,
            ranked_countable,
            ranked_summable,
            ranked_averageable,
            terminal: None,
            preallocated: false,
            outlives_delete: false,
            flat: false,
            skip_if_absent_properties: Vec::new(),
            summable_off_count_index: None,
        }
    }

    /// No ranking flags ⇒ byte-identical to the pre-PV14 4-way dispatch. This
    /// is the arm every existing contract takes.
    #[test]
    fn unranked_levels_keep_their_range_flag_tree_type() {
        for (rc, rs, expected) in [
            (true, true, TreeType::ProvableCountProvableSumTree),
            (true, false, TreeType::ProvableCountTree),
            (false, true, TreeType::ProvableSumTree),
            (false, false, TreeType::NormalTree),
        ] {
            let (tree_type, axes) =
                property_name_tree_type_and_ranked_axes(Some(&info(rc, rs, false, false, false)))
                    .expect("unranked resolution must succeed");
            assert_eq!(tree_type, expected);
            assert!(axes.is_empty());
        }
    }

    #[test]
    fn prefix_levels_resolve_to_normal_tree_without_axes() {
        let (tree_type, axes) =
            property_name_tree_type_and_ranked_axes(None).expect("prefix resolution must succeed");
        assert_eq!(tree_type, TreeType::NormalTree);
        assert!(axes.is_empty());
    }

    /// Count-only ranking on a count-only range layout is the one shape that
    /// gets the dedicated single-axis PCIT element.
    #[test]
    fn count_only_ranking_on_count_only_range_layout_is_pcit() {
        let (tree_type, axes) =
            property_name_tree_type_and_ranked_axes(Some(&info(true, false, true, false, false)))
                .expect("resolution must succeed");
        assert_eq!(tree_type, TreeType::ProvableCountIndexedTree);
        assert_eq!(axes, vec![IndexAxis::Count]);
    }

    #[test]
    fn sum_only_ranking_on_sum_only_range_layout_is_psit() {
        let (tree_type, axes) =
            property_name_tree_type_and_ranked_axes(Some(&info(false, true, false, true, false)))
                .expect("resolution must succeed");
        assert_eq!(tree_type, TreeType::ProvableSumIndexedTree);
        assert_eq!(axes, vec![IndexAxis::Sum]);
    }

    /// Ranking by count only, but the index also opts into `rangeSummable`:
    /// the base layout is PCPS, so the upgrade is the multi-axis PCPSIT with a
    /// single-entry TLV — NOT the PCIT element (which cannot carry a
    /// provable-sum primary).
    #[test]
    fn count_only_ranking_on_a_pcps_layout_is_pcpsit_with_one_axis() {
        let (tree_type, axes) =
            property_name_tree_type_and_ranked_axes(Some(&info(true, true, true, false, false)))
                .expect("resolution must succeed");
        assert_eq!(tree_type, TreeType::ProvableCountProvableSumIndexedTree);
        assert_eq!(axes, vec![IndexAxis::Count]);
    }

    /// `rankedAverageable` on its own is legal and must work.
    #[test]
    fn averageable_only_ranking_is_pcpsit_with_the_avg_axis() {
        let (tree_type, axes) =
            property_name_tree_type_and_ranked_axes(Some(&info(true, true, false, false, true)))
                .expect("resolution must succeed");
        assert_eq!(tree_type, TreeType::ProvableCountProvableSumIndexedTree);
        assert_eq!(axes, vec![IndexAxis::Avg]);
    }

    #[test]
    fn all_three_axes_come_out_in_canonical_tlv_order() {
        let (tree_type, axes) =
            property_name_tree_type_and_ranked_axes(Some(&info(true, true, true, true, true)))
                .expect("resolution must succeed");
        assert_eq!(tree_type, TreeType::ProvableCountProvableSumIndexedTree);
        assert_eq!(axes, vec![IndexAxis::Count, IndexAxis::Sum, IndexAxis::Avg]);
        let tlv = ranked_axes_tlv(&axes);
        assert_eq!(tlv, vec![(0u8, None), (1u8, None), (2u8, None)]);
        grovedb::Element::validate_pcpsit_axes(&tlv)
            .expect("the TLV we emit must satisfy grovedb's canonical-axes invariant");
    }

    /// A ranked axis without its range axis is a grammar invariant violation —
    /// fail closed rather than emit a tree grovedb will not maintain.
    #[test]
    fn ranked_axis_without_its_range_axis_is_rejected() {
        let result =
            property_name_tree_type_and_ranked_axes(Some(&info(false, false, true, false, false)));
        assert!(result.is_err());
    }

    #[test]
    fn indexed_trees_collapse_onto_their_non_indexed_mirrors() {
        assert_eq!(
            non_indexed_mirror_tree_type(TreeType::ProvableCountIndexedTree),
            TreeType::ProvableCountTree
        );
        assert_eq!(
            non_indexed_mirror_tree_type(TreeType::ProvableSumIndexedTree),
            TreeType::ProvableSumTree
        );
        assert_eq!(
            non_indexed_mirror_tree_type(TreeType::ProvableCountProvableSumIndexedTree),
            TreeType::ProvableCountProvableSumTree
        );
        assert_eq!(
            non_indexed_mirror_tree_type(TreeType::CountSumTree),
            TreeType::CountSumTree
        );
    }
}
