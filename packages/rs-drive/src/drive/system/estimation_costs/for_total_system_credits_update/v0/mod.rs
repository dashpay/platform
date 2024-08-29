use crate::drive::Drive;

use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::{ApproximateElements, EstimatedLevel};
use grovedb::EstimatedLayerInformation;
use grovedb::EstimatedLayerSizes::{AllItems, AllSubtrees};

use crate::drive::system::misc_path_vec;

use grovedb::EstimatedSumTrees::SomeSumTrees;
use std::collections::HashMap;

//                                                      DataContract_Documents 64
//                                 /                                                                         \
//                       Identities 32                                                                        Balances 96
//             /                            \                                              /                                               \
//   Token_Balances 16                    Pools 48                    WithdrawalTransactions 80                                        Votes  112
//       /      \                           /                                      /                                                    /                          \
//     NUPKH->I 8 UPKH->I 24   PreFundedSpecializedBalances 40          SpentAssetLockTransactions 72                         ->    Misc 104   <-                       Versions 120

impl Drive {
    /// Adds estimation costs for total system credits update.
    ///
    /// This method operates on the provided HashMap, `estimated_costs_only_with_layer_info`, and adds
    /// new entries to it, representing the estimated costs for the total system credits update.
    #[inline(always)]
    pub(super) fn add_estimation_costs_for_total_system_credits_update_v0(
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    ) {
        // we have constructed the top layer so contract/documents tree are at the top
        // since balance will be on layer 3, updating will mean we will update 1 sum tree
        // and 2 normal trees, hence we should give an equal weight to both
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path([]),
            EstimatedLayerInformation {
                is_sum_tree: false,
                estimated_layer_count: EstimatedLevel(3, false),
                estimated_layer_sizes: AllSubtrees(
                    12, // about 32 + 1 + 1 / 3
                    SomeSumTrees {
                        sum_trees_weight: 1,
                        non_sum_trees_weight: 2,
                    },
                    None,
                ),
            },
        );

        // we then need to insert the contract layer
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(misc_path_vec()),
            EstimatedLayerInformation {
                is_sum_tree: false,
                estimated_layer_count: ApproximateElements(1),
                estimated_layer_sizes: AllItems(1, 8, None),
            },
        );
    }
}
