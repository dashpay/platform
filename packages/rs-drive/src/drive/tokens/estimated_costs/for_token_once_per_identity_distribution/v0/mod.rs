use crate::drive::tokens::paths::{
    token_distributions_root_path_vec, token_once_per_identity_distributions_path_vec,
    token_root_once_per_identity_distributions_path_vec,
};
use crate::drive::Drive;
use crate::util::type_constants::{DEFAULT_HASH_SIZE_U8, U8_SIZE_U8};
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::EstimatedLevel;
use grovedb::EstimatedLayerSizes::{AllItems, AllSubtrees};
use grovedb::EstimatedSumTrees::NoSumTrees;
use grovedb::{EstimatedLayerInformation, TreeType};
use std::collections::HashMap;

/// A claim item holds the claim's block time as 8 big-endian bytes.
const CLAIM_ITEM_VALUE_SIZE: u32 = 8;

/// Single-epoch storage flags owned by the claimant: one type byte, two epoch bytes and the
/// 32-byte owner id.
const CLAIM_ITEM_FLAGS_SIZE: u32 = 35;

impl Drive {
    pub(crate) fn add_estimation_costs_for_token_once_per_identity_distribution_v0(
        token_id: Option<[u8; 32]>,
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    ) {
        // 1. The token distributions root tree, holding the timed, perpetual, pre-programmed and
        //    once-per-identity subtrees under one-byte keys.
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(token_distributions_root_path_vec()),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: EstimatedLevel(1, false),
                estimated_layer_sizes: AllSubtrees(U8_SIZE_U8, NoSumTrees, None),
            },
        );

        // 2. The once-per-identity root tree: one subtree per token, keyed by token id.
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(
                token_root_once_per_identity_distributions_path_vec(),
            ),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: EstimatedLevel(10, false),
                estimated_layer_sizes: AllSubtrees(DEFAULT_HASH_SIZE_U8, NoSumTrees, None),
            },
        );

        if let Some(token_id) = token_id {
            // 3. The token's claims subtree: one item per identity that already claimed. Any
            //    identity may claim, so this tree can grow large; estimate it as deep as the
            //    evonode perpetual claims tree.
            estimated_costs_only_with_layer_info.insert(
                KeyInfoPath::from_known_owned_path(token_once_per_identity_distributions_path_vec(
                    token_id,
                )),
                EstimatedLayerInformation {
                    tree_type: TreeType::NormalTree,
                    estimated_layer_count: EstimatedLevel(10, false),
                    estimated_layer_sizes: AllItems(
                        DEFAULT_HASH_SIZE_U8,
                        CLAIM_ITEM_VALUE_SIZE,
                        Some(CLAIM_ITEM_FLAGS_SIZE),
                    ),
                },
            );
        }
    }
}
