use crate::drive::Drive;

use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::EstimatedLevel;
use grovedb::EstimatedLayerSizes::{AllItems, AllSubtrees};
use grovedb::{EstimatedLayerInformation, TreeType};

use crate::drive::balances::total_tokens_root_supply_path;
use crate::drive::system::misc_path;
use crate::util::type_constants::{DEFAULT_HASH_SIZE_U8, U64_SIZE_U32};
use grovedb::EstimatedSumTrees::{NoSumTrees, SomeSumTrees};
use std::collections::HashMap;

impl Drive {
    /// Adds estimation costs for token total supply in Drive for version 0.
    ///
    /// This function estimates the storage and update costs associated with token total supply
    /// in Drive, providing detailed information about the layer structure and required updates
    /// for each relevant path. The provided `HashMap` will be updated with the estimated costs
    /// for each layer involved in the process.
    ///
    /// # Parameters
    ///
    /// * `estimated_costs_only_with_layer_info`: A mutable reference to a `HashMap`
    ///   that stores the estimated layer information for each key information path.
    ///   This map will be populated with the relevant layer information for the token supply
    ///   data and other associated trees.
    ///
    /// # Notes
    ///
    /// The function estimates costs for the following layers:
    ///
    /// 1. **Top Layer**:
    ///    - Contains general balance information and is assumed to be located on
    ///      level 3 of the hierarchy. It involves updating:
    ///      - 1 normal tree for contract/documents.
    ///      - 1 sum tree for balances.
    ///      - 1 normal for votes.
    ///      - 1 normal tree for misc.
    ///    - This layer has an equal weight distribution between normal and sum trees.
    ///
    /// 2. **Misc Layer**:
    ///    - A normal tree that contains miscellaneous data relevant to the total supply
    ///      process.
    ///
    /// 3. **Total Tokens Root Supply Path**:
    ///    - This path represents the root for the total tokens supply and is updated
    ///      with the corresponding token supply information. It is estimated to update
    ///      an average of 10 nodes in a normal tree structure.
    pub(super) fn add_estimation_costs_for_token_total_supply_v0(
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    ) {
        // we have constructed the top layer so contract/documents tree are at the top
        // since balance will be on layer 4 (level 3 on right, then right, then left)
        // updating will mean we will update:
        // 1 normal tree (misc)
        // 1 normal tree (votes)
        // 1 sum tree (balances)
        // 1 normal tree (contract/documents)
        // hence we should give an equal weight to both
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path([]),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: EstimatedLevel(3, false),
                // 17 because we have 2 layers at 32 and two layers at 2
                estimated_layer_sizes: AllSubtrees(
                    17,
                    SomeSumTrees {
                        sum_trees_weight: 1,
                        big_sum_trees_weight: 0,
                        count_trees_weight: 0,
                        count_sum_trees_weight: 0,
                        non_sum_trees_weight: 3,
                    },
                    None,
                ),
            },
        );

        // in the misc tree
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(misc_path()),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: EstimatedLevel(2, false),
                estimated_layer_sizes: AllSubtrees(1, NoSumTrees, None),
            },
        );

        // in the total tokens root supply path
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(total_tokens_root_supply_path()),
            EstimatedLayerInformation {
                tree_type: TreeType::BigSumTree,
                estimated_layer_count: EstimatedLevel(10, false),
                estimated_layer_sizes: AllItems(DEFAULT_HASH_SIZE_U8, U64_SIZE_U32, None),
            },
        );
    }
}
