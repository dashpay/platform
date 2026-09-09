use versioned_feature_core::FeatureVersion;

pub mod v1;

#[derive(Clone, Debug, Default)]
pub struct DriveGroveMethodVersions {
    pub basic: DriveGroveBasicMethodVersions,
    pub batch: DriveGroveBatchMethodVersions,
    pub apply: DriveGroveApplyMethodVersions,
    pub costs: DriveGroveCostMethodVersions,
}

#[derive(Clone, Debug, Default)]
pub struct DriveGroveBasicMethodVersions {
    pub grove_insert: FeatureVersion,
    pub grove_insert_empty_tree: FeatureVersion,
    pub grove_insert_if_not_exists: FeatureVersion,
    pub grove_insert_if_not_exists_return_existing_element: FeatureVersion,
    pub grove_clear: FeatureVersion,
    pub grove_delete: FeatureVersion,
    pub grove_get_raw: FeatureVersion,
    pub grove_get_raw_optional: FeatureVersion,
    pub grove_get_raw_value_u64_from_encoded_var_vec: FeatureVersion,
    pub grove_get: FeatureVersion,
    pub grove_get_path_query_serialized_results: FeatureVersion,
    pub grove_get_path_query_serialized_or_sum_results: FeatureVersion,
    pub grove_get_path_query: FeatureVersion,
    pub grove_get_path_query_with_optional: FeatureVersion,
    pub grove_get_raw_path_query_with_optional: FeatureVersion,
    pub grove_get_raw_path_query: FeatureVersion,
    pub grove_get_proved_path_query: FeatureVersion,
    pub grove_get_proved_path_query_with_conditional: FeatureVersion,
    pub grove_get_proved_branch_chunk_query: FeatureVersion,
    pub grove_get_proved_trunk_chunk_query: FeatureVersion,
    pub grove_get_sum_tree_total_value: FeatureVersion,
    pub grove_has_raw: FeatureVersion,
    pub grove_get_raw_item: FeatureVersion,
    pub grove_get_optional_sum_tree_total_value: FeatureVersion,
    pub grove_get_raw_optional_item: FeatureVersion,
    pub grove_get_big_sum_tree_total_value: FeatureVersion,
    pub grove_get_proved_path_query_v1: FeatureVersion,
    pub grove_commitment_tree_count: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveGroveBatchMethodVersions {
    pub batch_insert_empty_tree: FeatureVersion,
    pub batch_insert_empty_tree_if_not_exists: FeatureVersion,
    pub batch_insert_empty_tree_if_not_exists_check_existing_operations: FeatureVersion,
    pub batch_insert_sum_item_if_not_exists: FeatureVersion,
    pub batch_insert_sum_item_or_add_to_if_already_exists: FeatureVersion,
    pub batch_insert: FeatureVersion,
    pub batch_insert_if_not_exists: FeatureVersion,
    pub batch_insert_if_changed_value: FeatureVersion,
    pub batch_replace: FeatureVersion,
    pub batch_delete: FeatureVersion,
    pub batch_delete_items_in_path_query: FeatureVersion,
    pub batch_move_items_in_path_query: FeatureVersion,
    pub batch_remove_raw: FeatureVersion,
    pub batch_delete_up_tree_while_empty: FeatureVersion,
    pub batch_refresh_reference: FeatureVersion,
    pub batch_insert_empty_sum_tree: FeatureVersion,
    pub batch_insert_empty_count_tree: FeatureVersion,
    pub batch_insert_empty_count_sum_tree: FeatureVersion,
    pub batch_insert_empty_provable_count_tree: FeatureVersion,
    /// Provable sum tree (range-summable). Mirrors
    /// [`Self::batch_insert_empty_provable_count_tree`] for the sum
    /// surface — added alongside the v3 sum-tree feature.
    pub batch_insert_empty_provable_sum_tree: FeatureVersion,
    /// Combined provable count + sum tree. Used when an index opts into
    /// both `rangeCountable: true` and `rangeSummable: true`. Activates
    /// when grovedb PR 670 lands the `Element::ProvableCountSumTree`
    /// callable empty-tree variant.
    pub batch_insert_empty_provable_count_sum_tree: FeatureVersion,
    /// Fully-provable combined count + sum tree (PCPS). Used when an
    /// index opts into BOTH `rangeCountable: true` AND `rangeSummable:
    /// true`: per-node counts AND per-node sums are committed to every
    /// internal merk node, so range queries can answer
    /// `AggregateCountOnRange` / `AggregateSumOnRange` (and the
    /// combined variant once grovedb PR 670 ships) over a single tree.
    pub batch_insert_empty_provable_count_provable_sum_tree: FeatureVersion,
    /// Provable count-indexed tree (grovedb PR 657). The count-only
    /// ranked variant: a `ProvableCountTree`-shaped primary Merk plus a
    /// single count-ordered secondary Merk, letting an index answer
    /// "top k groups by count" in O(log n + k) with a proof.
    pub batch_insert_empty_provable_count_indexed_tree: FeatureVersion,
    /// Provable sum-indexed tree (grovedb PR 657). Sum-only ranked
    /// counterpart of
    /// [`Self::batch_insert_empty_provable_count_indexed_tree`].
    pub batch_insert_empty_provable_sum_indexed_tree: FeatureVersion,
    /// Provable count + provable sum indexed tree (grovedb PR 657).
    /// Carries a TLV list of 1..=3 ranked axes (count / sum / avg), one
    /// secondary Merk each, so a single tree can be ranked on any
    /// declared axis. Used whenever more than one axis is requested, or
    /// when the ranked axis is `avg` (which needs both count and sum).
    pub batch_insert_empty_provable_count_provable_sum_indexed_tree: FeatureVersion,
    pub batch_move: FeatureVersion,
    pub batch_insert_item_with_sum_item_if_not_exists: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveGroveApplyMethodVersions {
    pub grove_apply_operation: FeatureVersion,
    pub grove_apply_batch: FeatureVersion,
    pub grove_apply_partial_batch: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveGroveCostMethodVersions {
    pub grove_batch_operations_costs: FeatureVersion,
}
