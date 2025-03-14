use crate::drive::tokens::paths::{
    token_distributions_root_path_vec, token_perpetual_distributions_path_vec,
    token_root_perpetual_distributions_path_vec,
};
use crate::drive::Drive;
use crate::util::type_constants::{DEFAULT_HASH_SIZE_U8, U8_SIZE_U8};
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::EstimatedLevel;
use grovedb::EstimatedLayerSizes::AllSubtrees;
use grovedb::EstimatedSumTrees::NoSumTrees;
use grovedb::{EstimatedLayerInformation, TreeType};
use std::collections::HashMap;

impl Drive {
    /// Version 0 of the estimation function for perpetual distributions.
    ///
    /// This function adds estimation cost entries for:
    ///   1. The root perpetual distributions tree.
    ///   2. The token-specific subtree (using `token_id`).
    ///   3. The subtree tracking identities' last claim time.
    ///
    /// # Parameters
    /// - `token_id`: The identifier for the token.
    /// - `estimated_costs_only_with_layer_info`: A mutable hashmap that holds estimated layer information.
    pub(crate) fn add_estimation_costs_for_token_perpetual_distribution_v0(
        token_id: Option<[u8; 32]>,
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    ) {
        // 1. Add estimation for the root distributions tree.
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(token_distributions_root_path_vec()),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: EstimatedLevel(1, false),
                estimated_layer_sizes: AllSubtrees(U8_SIZE_U8, NoSumTrees, None),
            },
        );

        // 2. Add estimation for the root perpetual distributions tree.
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(token_root_perpetual_distributions_path_vec()),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: EstimatedLevel(10, false), // Estimated depth
                estimated_layer_sizes: AllSubtrees(DEFAULT_HASH_SIZE_U8, NoSumTrees, None),
            },
        );

        if let Some(token_id) = token_id {
            // 3. Add estimation for the token-specific perpetual distribution subtree.
            estimated_costs_only_with_layer_info.insert(
                KeyInfoPath::from_known_owned_path(token_perpetual_distributions_path_vec(
                    token_id,
                )),
                EstimatedLayerInformation {
                    tree_type: TreeType::NormalTree,
                    estimated_layer_count: EstimatedLevel(2, false),
                    estimated_layer_sizes: AllSubtrees(U8_SIZE_U8, NoSumTrees, None),
                },
            );
        }
    }
}
