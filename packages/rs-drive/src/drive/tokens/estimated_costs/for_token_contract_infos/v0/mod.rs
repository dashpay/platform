use crate::drive::Drive;

use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::EstimatedLevel;
use grovedb::EstimatedLayerSizes::{AllItems, AllSubtrees};
use grovedb::{EstimatedLayerInformation, TreeType};

use crate::drive::tokens::paths::{token_contract_infos_root_path, tokens_root_path};
use crate::util::type_constants::DEFAULT_HASH_SIZE_U8;
use grovedb::EstimatedSumTrees::{NoSumTrees, SomeSumTrees};
use std::collections::HashMap;

impl Drive {
    /// Adds estimated storage costs related to token contract infos (v0).
    ///
    /// This function populates the provided `estimated_costs_only_with_layer_info` map with
    /// `EstimatedLayerInformation` entries, which represent cost estimation hints for GroveDB
    /// during query planning and fee calculations. It is specific to version 0 of the token
    /// selling price schema.
    ///
    /// # Parameters
    ///
    /// * `estimated_costs_only_with_layer_info` - A mutable reference to a map storing
    ///   estimation data keyed by GroveDB paths.
    ///
    /// # Estimation Structure
    ///
    /// The following layers are estimated:
    ///
    /// 1. **Top-Level Structure **:
    ///    - Represents the root of the document structure.
    ///    - Contains 1 subtree (contracts/documents).
    ///    - Tree type: Normal.
    ///    - Depth: 2 levels deep.
    ///
    /// 2. **Token Root Path **:
    ///    - Represents the root layer under which token-related structures reside.
    ///    - Includes a mixture of normal trees and sum trees (e.g., for token balances).
    ///    - Estimated as 1 subtree with specific weights for different tree types.
    ///    - Tree type: Normal.
    ///    - Depth: 2 levels deep.
    ///
    /// 3. **Token Contract Infos Root Path **:
    ///    - Contains token contract infos.
    ///    - Expected to hold a relatively flat key-value structure with bincode encoded versioned token contract infos.
    ///    - Tree type: Normal.
    ///    - Estimated to contain values of 36 bytes (u64), each with hash-sized keys and no flags.
    ///    - Estimated depth: 10 levels.
    ///
    /// This setup provides a balanced estimation model for token contract info operations,
    /// which helps GroveDB optimize storage behavior and fee prediction.
    pub(super) fn add_estimation_costs_for_token_contract_infos_v0(
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    ) {
        // we have constructed the top layer so contract/documents tree are at the top
        // since balance will be on layer 3 (level 2 on left then left)
        // updating will mean we will update:
        // 1 normal tree (token balances)
        // 1 normal tree (identities)
        // 1 normal tree (contract/documents)
        // hence we should give an equal weight to both
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path([]),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: EstimatedLevel(2, false),
                estimated_layer_sizes: AllSubtrees(1, NoSumTrees, None),
            },
        );

        // there is one tree for the root path
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(tokens_root_path()),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: EstimatedLevel(2, false),
                estimated_layer_sizes: AllSubtrees(
                    1,
                    SomeSumTrees {
                        sum_trees_weight: 0,
                        big_sum_trees_weight: 1,
                        count_trees_weight: 0,
                        count_sum_trees_weight: 0,
                        non_sum_trees_weight: 2,
                    },
                    None,
                ),
            },
        );

        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(token_contract_infos_root_path()),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: EstimatedLevel(10, false), // we can estimate 10 levels deep
                estimated_layer_sizes: AllItems(DEFAULT_HASH_SIZE_U8, 36, None),
            },
        );
    }
}
