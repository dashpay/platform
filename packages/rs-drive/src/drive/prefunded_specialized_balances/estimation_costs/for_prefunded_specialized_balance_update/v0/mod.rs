use crate::drive::Drive;

use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::{EstimatedLevel, PotentiallyAtMaxElements};
use grovedb::EstimatedLayerInformation;
use grovedb::EstimatedLayerSizes::{AllItems, AllSubtrees};

use crate::drive::constants::AVERAGE_BALANCE_SIZE;
use crate::drive::prefunded_specialized_balances::{
    prefunded_specialized_balances_for_voting_path_vec, prefunded_specialized_balances_path,
};
use crate::util::type_constants::DEFAULT_HASH_SIZE_U8;
use grovedb::EstimatedSumTrees::{AllSumTrees, SomeSumTrees};
use std::collections::HashMap;

impl Drive {
    /// Adds estimation costs for total system credits update.
    ///
    /// This method operates on the provided HashMap, `estimated_costs_only_with_layer_info`, and adds
    /// new entries to it, representing the estimated costs for the total system credits update.
    #[inline(always)]
    pub(super) fn add_estimation_costs_for_prefunded_specialized_balance_update_v0(
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    ) {
        // todo: this will be inserted at the same time as other estimated costs for documents,
        //  hence we add the full information, but it would be much better that estimated costs would
        //  be merged instead of overwritten
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path([]),
            EstimatedLayerInformation {
                is_sum_tree: false,
                // We are on the 3rd level
                estimated_layer_count: EstimatedLevel(3, false),
                estimated_layer_sizes: AllSubtrees(
                    1,
                    SomeSumTrees {
                        sum_trees_weight: 1,
                        non_sum_trees_weight: 1,
                    },
                    None,
                ),
            },
        );

        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(prefunded_specialized_balances_path()),
            EstimatedLayerInformation {
                is_sum_tree: true,
                estimated_layer_count: EstimatedLevel(0, false),
                estimated_layer_sizes: AllSubtrees(1, AllSumTrees, None),
            },
        );

        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(prefunded_specialized_balances_for_voting_path_vec()),
            EstimatedLayerInformation {
                is_sum_tree: true,
                estimated_layer_count: PotentiallyAtMaxElements,
                estimated_layer_sizes: AllItems(DEFAULT_HASH_SIZE_U8, AVERAGE_BALANCE_SIZE, None),
            },
        );
    }
}
