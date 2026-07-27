/// The estimated costs for a contract insert
mod add_estimation_costs_for_contract_insertion;

use grovedb::EstimatedSumTrees::{NoSumTrees, SomeSumTrees};
use grovedb::TreeType;

/// Per-tree-type weight tally used by contract-scoped estimation paths.
///
/// Mirrors every `TreeType` variant the contract apply / update paths
/// can write at a doctype's children layer: the primary-key tree (`[0]`)
/// plus each top-level index's terminator tree. Lives one module level
/// above the individual estimation entrypoints (`add_estimation_costs_*`)
/// so future estimation surfaces (delete / update) can reuse the tally
/// without duplicating the saturating-add bookkeeping and the
/// `to_estimated_sum_trees()` zero-detection logic.
#[derive(Debug, Default, Clone, Copy)]
pub(super) struct TreeTypeWeights {
    pub(super) normal: u8,
    pub(super) count: u8,
    pub(super) provable_count: u8,
    pub(super) sum: u8,
    pub(super) big_sum: u8,
    pub(super) provable_sum: u8,
    pub(super) count_sum: u8,
    pub(super) provable_count_sum: u8,
    pub(super) provable_count_provable_sum: u8,
}

impl TreeTypeWeights {
    pub(super) fn tally(&mut self, tree_type: TreeType) {
        match tree_type {
            TreeType::NormalTree => self.normal = self.normal.saturating_add(1),
            TreeType::CountTree => self.count = self.count.saturating_add(1),
            TreeType::ProvableCountTree => {
                self.provable_count = self.provable_count.saturating_add(1)
            }
            TreeType::SumTree => self.sum = self.sum.saturating_add(1),
            TreeType::BigSumTree => self.big_sum = self.big_sum.saturating_add(1),
            TreeType::ProvableSumTree => self.provable_sum = self.provable_sum.saturating_add(1),
            TreeType::CountSumTree => self.count_sum = self.count_sum.saturating_add(1),
            TreeType::ProvableCountSumTree => {
                self.provable_count_sum = self.provable_count_sum.saturating_add(1)
            }
            TreeType::ProvableCountProvableSumTree => {
                self.provable_count_provable_sum =
                    self.provable_count_provable_sum.saturating_add(1)
            }
            // Other tree variants (e.g. CommitmentTree) don't appear at
            // the doctype's children layer — they're shielded-pool-only.
            // Bucket them with `normal` (zero per-node aggregate cost) so
            // the tally stays exhaustive.
            _ => self.normal = self.normal.saturating_add(1),
        }
    }

    /// Convert the tally into an [`grovedb::EstimatedSumTrees`]. Returns
    /// `NoSumTrees` only when every aggregate-bearing tally is zero
    /// (preserving the existing v0/v1-output contract for purely-normal
    /// doctypes — see the regression test in the v1 module).
    pub(super) fn to_estimated_sum_trees(self) -> grovedb::EstimatedSumTrees {
        let any_aggregate = self.count
            | self.provable_count
            | self.sum
            | self.big_sum
            | self.provable_sum
            | self.count_sum
            | self.provable_count_sum
            | self.provable_count_provable_sum;
        if any_aggregate == 0 {
            NoSumTrees
        } else {
            SomeSumTrees {
                sum_trees_weight: self.sum,
                big_sum_trees_weight: self.big_sum,
                count_trees_weight: self.count,
                count_sum_trees_weight: self.count_sum,
                non_sum_trees_weight: self.normal,
                provable_sum_trees_weight: self.provable_sum,
                provable_count_trees_weight: self.provable_count,
                provable_count_sum_trees_weight: self.provable_count_sum,
                provable_count_provable_sum_trees_weight: self.provable_count_provable_sum,
            }
        }
    }
}

/// Derive the property-name tree type of a top-level index from its
/// terminator's flags. Mirror of the selection in
/// `add_indices_for_index_level_for_contract_operations` — when the
/// terminator opts into a range axis (`range_countable` or
/// `range_summable`) the property-name level gets a `Provable*` tree;
/// otherwise it gets the root-only-aggregate variant. The combinations
/// follow the same matrix the document-storage primary-key dispatcher
/// uses, see
/// [`crate::drive::document::primary_key_tree_type::DocumentTypePrimaryKeyTreeType::primary_key_tree_type`].
pub(super) fn property_name_tree_type_from_flags(
    info: &dpp::data_contract::document_type::IndexLevelTypeInfo,
) -> TreeType {
    let summable = info.summable.is_some();
    let countable = info.countable.is_countable();
    match (
        info.range_summable,
        info.range_countable,
        summable,
        countable,
    ) {
        (true, true, _, _) => TreeType::ProvableCountProvableSumTree,
        (true, false, _, _) => TreeType::ProvableSumTree,
        (false, true, _, _) => TreeType::ProvableCountTree,
        (false, false, true, true) => TreeType::CountSumTree,
        (false, false, true, false) => TreeType::SumTree,
        (false, false, false, true) => TreeType::CountTree,
        (false, false, false, false) => TreeType::NormalTree,
    }
}
