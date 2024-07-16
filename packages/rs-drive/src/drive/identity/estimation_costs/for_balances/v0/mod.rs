use crate::drive::constants::AVERAGE_BALANCE_SIZE;

use crate::drive::Drive;

use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::{EstimatedLevel, PotentiallyAtMaxElements};
use grovedb::EstimatedLayerInformation;
use grovedb::EstimatedLayerSizes::{AllItems, AllSubtrees};

use crate::drive::balances::balance_path_vec;

use crate::util::type_constants::DEFAULT_HASH_SIZE_U8;
use grovedb::EstimatedSumTrees::SomeSumTrees;
use std::collections::HashMap;
// we need to construct the reference from the split height of the key
// type which is at 4
// 0 represents document storage
// Then we add document id
// Then we add 0 if the document type keys history
// vec![vec![0], Vec::from(key.id)];
// 1 (vec size) + 1 (subvec size) + 1 (0) + 1 (subvec size) + 32 (key id size)
// 1 for type reference
// 1 for reference type
// 1 for root height offset
// reference path size
// 1 reference_hops options
// 1 reference_hops count
// 1 element flags option

impl Drive {
    /// Adds estimation costs for balances in Drive for version 0.
    ///
    /// This function provides a mechanism to estimate the costs of balances
    /// in the drive by updating the provided `HashMap` with layer information
    /// relevant to balances.
    ///
    /// # Parameters
    ///
    /// * `estimated_costs_only_with_layer_info`: A mutable reference to a `HashMap`
    ///   that stores estimated layer information based on the key information path.
    ///
    /// # Notes
    ///
    /// The function estimates costs for two key layers:
    ///
    /// 1. A top layer with balance information, assumed to be on layer 2. Updates to
    ///    this layer are estimated to involve updating one sum tree and one normal tree.
    /// 2. A contract layer for the balance. This layer is considered as a sum tree.
    /// ```
    pub(super) fn add_estimation_costs_for_balances_v0(
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    ) {
        // we have constructed the top layer so contract/documents tree are at the top
        // since balance will be on layer 2, updating will mean we will update 1 sum tree
        // and 1 normal tree, hence we should give an equal weight to both
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path([]),
            EstimatedLayerInformation {
                is_sum_tree: false,
                estimated_layer_count: EstimatedLevel(1, false),
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

        // we then need to insert the contract layer
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(balance_path_vec()),
            EstimatedLayerInformation {
                is_sum_tree: true,
                estimated_layer_count: PotentiallyAtMaxElements,
                estimated_layer_sizes: AllItems(DEFAULT_HASH_SIZE_U8, AVERAGE_BALANCE_SIZE, None),
            },
        );
    }
}
