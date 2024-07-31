//! Implements in Drive a function which adds estimated costs to a hashmap for adding an asset lock (version 0).

use crate::drive::Drive;

use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::{EstimatedLevel, PotentiallyAtMaxElements};
use grovedb::EstimatedLayerInformation;
use grovedb::EstimatedLayerSizes::{AllItems, AllSubtrees};

use crate::drive::asset_lock::asset_lock_storage_path;
use grovedb::EstimatedSumTrees::SomeSumTrees;
use std::collections::HashMap;

impl Drive {
    /// Add estimated costs to a hashmap for adding an asset lock (version 0).
    ///
    /// This function modifies the provided hashmap, `estimated_costs_only_with_layer_info`,
    /// by inserting two sets of key-value pairs related to the estimation costs for adding an asset lock.
    ///
    /// The function assumes:
    /// - The top layer has already been constructed so that contract/documents tree are at the top.
    /// - The balance is on layer 2, so updating this will mean updating 1 sum tree and 1 normal tree.
    ///   This is why an equal weight is given to both types of trees in the estimation.
    ///
    /// # Arguments
    ///
    /// * `estimated_costs_only_with_layer_info` - A mutable reference to a hashmap that will store
    ///   the estimated layer information associated with the key info paths.
    ///
    /// # KeyInfoPath Definitions:
    ///
    /// * First insertion: An empty key info path.
    /// * Second insertion: Uses the `asset_lock_storage_path()` function to derive the path.
    ///
    /// # Example Layer Information:
    ///
    /// * For the first insertion, it is assumed that:
    ///   - The layer is not a sum tree.
    ///   - There's an estimated level of 1 which is not a sum tree.
    ///   - There's equal weight given to sum trees and non-sum trees.
    ///
    /// * For the second insertion, it is assumed that:
    ///   - The layer is not a sum tree.
    ///   - The layer might potentially have max elements.
    ///   - Each item in this layer has a size of 36, which represents the size of an outpoint.
    ///
    pub(crate) fn add_estimation_costs_for_adding_asset_lock_v0(
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    ) {
        //                                                      DataContract_Documents 64
        //                                 /                                                                         \
        //                       Identities 32                                                                        Balances 96
        //             /                            \                                              /                                               \
        //   Token_Balances 16                    Pools 48                    WithdrawalTransactions 80                                        Votes  112
        //       /      \                           /                                      /                                                    /                          \
        //     NUPKH->I 8 UPKH->I 24   PreFundedSpecializedBalances 40       -> SpentAssetLockTransactions 72  <-                           Misc 104                          Versions 120

        // we have constructed the top layer so contract/documents tree are at the top
        // since SpentAssetLockTransactions will be on layer 3, updating will mean we will update 1 sum tree
        // and 2 normal tree, hence we should give an equal weight to both
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path([]),
            EstimatedLayerInformation {
                is_sum_tree: false,
                estimated_layer_count: EstimatedLevel(3, false),
                estimated_layer_sizes: AllSubtrees(
                    12, // 32 + 1 + 1 / 3
                    SomeSumTrees {
                        sum_trees_weight: 1,
                        non_sum_trees_weight: 2,
                    },
                    None,
                ),
            },
        );

        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(asset_lock_storage_path()),
            EstimatedLayerInformation {
                is_sum_tree: false,
                estimated_layer_count: PotentiallyAtMaxElements,
                estimated_layer_sizes: AllItems(
                    36, //The size of an outpoint
                    0, None,
                ),
            },
        );
    }
}
