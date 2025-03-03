use crate::drive::Drive;

use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::{EstimatedLevel, PotentiallyAtMaxElements};
use grovedb::EstimatedLayerSizes::{AllItems, AllSubtrees};
use grovedb::{EstimatedLayerInformation, TreeType};

use crate::drive::tokens::paths::{
    token_identity_infos_path, token_identity_infos_root_path, tokens_root_path,
};
use crate::util::type_constants::DEFAULT_HASH_SIZE_U8;
use grovedb::EstimatedSumTrees::{NoSumTrees, SomeSumTrees};
use std::collections::HashMap;

pub const ESTIMATED_TOKEN_INFO_SIZE_BYTES: u32 = 32;

impl Drive {
    pub(super) fn add_estimation_costs_for_token_identity_infos_v0(
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
                estimated_layer_count: EstimatedLevel(1, false), // this should be at the top
                estimated_layer_sizes: AllSubtrees(
                    1,
                    SomeSumTrees {
                        sum_trees_weight: 0,
                        big_sum_trees_weight: 1,
                        count_trees_weight: 0,
                        count_sum_trees_weight: 0,
                        non_sum_trees_weight: 1,
                    },
                    None,
                ),
            },
        );

        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(token_identity_infos_root_path()),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: EstimatedLevel(10, false), // we can estimate 10 levels deep
                estimated_layer_sizes: AllSubtrees(DEFAULT_HASH_SIZE_U8, NoSumTrees, None),
            },
        );

        // this is where the balances are
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(token_identity_infos_path(&token_id)),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: PotentiallyAtMaxElements,
                estimated_layer_sizes: AllItems(
                    DEFAULT_HASH_SIZE_U8,
                    ESTIMATED_TOKEN_INFO_SIZE_BYTES,
                    None,
                ),
            },
        );
    }
}
