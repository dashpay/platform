use crate::drive::constants::AVERAGE_BALANCE_SIZE;

use crate::drive::Drive;

use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::{EstimatedLevel, PotentiallyAtMaxElements};
use grovedb::EstimatedLayerSizes::{AllItems, AllSubtrees};
use grovedb::{EstimatedLayerInformation, TreeType};

use crate::drive::tokens::paths::{
    token_balances_path, token_balances_root_path, tokens_root_path,
};
use crate::util::type_constants::DEFAULT_HASH_SIZE_U8;
use grovedb::EstimatedSumTrees::{AllBigSumTrees, AllSumTrees, NoSumTrees};
use std::collections::HashMap;

impl Drive {
    /// Adds estimation costs for token balances in Drive for version 0.
    ///
    /// This function provides a mechanism to estimate the costs of token balances
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
    /// The function estimates costs for three layers:
    ///
    /// 1. **Top Layer**:
    ///    - Contains general balance information and is assumed to be located on
    ///      layer 3 (third level in the hierarchy). The update will involve modifying:
    ///      - 1 normal tree for token balances.
    ///      - 1 normal tree for identities.
    ///      - 1 normal tree for contract/documents.
    ///
    /// 2. **Token Balances or Info Layer**:
    ///    - This layer contains two nodes, either for representing the token balances or info.
    ///
    /// 3. **All Token Balances Layer**:
    ///    - This layer contains the token balance root path and is considered to be
    ///      a normal tree that will require updates to an average of 10 nodes.
    ///
    /// 3. **Token Layer**:
    ///    - This layer contains the contract-specific token balance information, which
    ///      is a sum tree structure with all token amounts for all identities that have a balance.
    /// ```
    pub(super) fn add_estimation_costs_for_token_balances_v0(
        token_id: [u8; 32],
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
                estimated_layer_count: EstimatedLevel(0, false), // this should be at the top
                estimated_layer_sizes: AllSubtrees(1, AllBigSumTrees, None),
            },
        );

        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(token_balances_root_path()),
            EstimatedLayerInformation {
                tree_type: TreeType::BigSumTree,
                estimated_layer_count: EstimatedLevel(10, false), // we can estimate 10 levels deep
                estimated_layer_sizes: AllSubtrees(DEFAULT_HASH_SIZE_U8, AllSumTrees, None),
            },
        );

        // this is where the balances are
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(token_balances_path(&token_id)),
            EstimatedLayerInformation {
                tree_type: TreeType::SumTree,
                estimated_layer_count: PotentiallyAtMaxElements,
                estimated_layer_sizes: AllItems(DEFAULT_HASH_SIZE_U8, AVERAGE_BALANCE_SIZE, None),
            },
        );
    }
}
