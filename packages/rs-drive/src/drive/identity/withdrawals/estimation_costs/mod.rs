use crate::drive::identity::withdrawals::paths::{
    get_withdrawal_credit_inflows_sum_tree_path_vec, get_withdrawal_root_path_vec,
};
use crate::drive::Drive;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::{ApproximateElements, EstimatedLevel, PotentiallyAtMaxElements};
use grovedb::EstimatedLayerSizes::{AllItems, AllSubtrees};
use grovedb::EstimatedSumTrees::SomeSumTrees;
use grovedb::{EstimatedLayerInformation, TreeType};
use std::collections::HashMap;

impl Drive {
    /// Adds estimation costs for updating the credit inflows sum tree under the withdrawals
    /// tree. Not versioned on its own: it belongs to `record_credit_inflows` version 0, the
    /// recording of every credit mint that the net daily withdrawal limit reads back.
    ///
    /// The estimated batch must never be cheaper than the applied one (fee validation prices
    /// with the estimated model and execution enforces `estimated >= actual`), so the layer
    /// sizes here stay on the generous side.
    pub(crate) fn add_estimation_costs_for_withdrawal_credit_inflows_update(
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    ) {
        // The withdrawals tree sits on layer 3 of the root tree, like the spent asset lock
        // transactions tree next to it (see the layout sketch in
        // `add_estimation_costs_for_adding_asset_lock`).
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path([]),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: EstimatedLevel(3, false),
                estimated_layer_sizes: AllSubtrees(
                    12, // about 32 + 1 + 1 / 3
                    SomeSumTrees {
                        sum_trees_weight: 1,
                        big_sum_trees_weight: 0,
                        count_trees_weight: 0,
                        count_sum_trees_weight: 0,
                        non_sum_trees_weight: 2,
                        provable_sum_trees_weight: 0,
                        provable_count_trees_weight: 0,
                        provable_count_sum_trees_weight: 0,
                        provable_count_provable_sum_trees_weight: 0,
                    },
                    None,
                ),
            },
        );

        // The withdrawals tree itself holds six single-byte keys: the index counter item, the
        // queue, the sum of reserved amounts, the broadcasted queue, the total credits history
        // and the credit inflows sum tree.
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(get_withdrawal_root_path_vec()),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: ApproximateElements(6),
                estimated_layer_sizes: AllSubtrees(
                    1,
                    SomeSumTrees {
                        sum_trees_weight: 2,
                        big_sum_trees_weight: 0,
                        count_trees_weight: 0,
                        count_sum_trees_weight: 0,
                        non_sum_trees_weight: 4,
                        provable_sum_trees_weight: 0,
                        provable_count_trees_weight: 0,
                        provable_count_sum_trees_weight: 0,
                        provable_count_provable_sum_trees_weight: 0,
                    },
                    None,
                ),
            },
        );

        // One sum item per block that minted credits, keyed by the 8-byte expiration time,
        // pruned as entries expire; sized for the worst case anyway.
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(get_withdrawal_credit_inflows_sum_tree_path_vec()),
            EstimatedLayerInformation {
                tree_type: TreeType::SumTree,
                estimated_layer_count: PotentiallyAtMaxElements,
                estimated_layer_sizes: AllItems(8, 8, None),
            },
        );
    }
}
