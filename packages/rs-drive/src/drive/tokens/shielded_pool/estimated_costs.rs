use crate::drive::shielded::paths::{
    token_shielded_pool_anchors_by_height_path_vec, token_shielded_pool_anchors_path_vec,
    token_shielded_pool_notes_path_vec, token_shielded_pool_nullifiers_path_vec,
    token_shielded_pool_path_vec, SHIELDED_NOTES_CHUNK_POWER,
};
use crate::drive::tokens::paths::{token_shielded_pools_root_path, tokens_root_path};
use crate::drive::Drive;
use crate::util::type_constants::DEFAULT_HASH_SIZE_U8;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::EstimatedLevel;
use grovedb::EstimatedLayerSizes::{AllItems, AllSubtrees, Mix};
use grovedb::EstimatedSumTrees::{AllBigSumTrees, AllSumTrees, NoSumTrees, SomeSumTrees};
use grovedb::{EstimatedLayerInformation, TreeType};
use std::collections::HashMap;

/// Average size of a note value: 32 cmx + 32 rho + 32 cv_net + 216 encrypted note = 312 bytes
/// (the same item the credit pool stores; see `crate::drive::shielded::estimated_costs`).
const AVERAGE_NOTE_VALUE_SIZE: u32 = 312;

/// Size of a nullifier key (32 bytes)
const NULLIFIER_KEY_SIZE: u8 = 32;

/// Size of an anchor key (32 bytes)
const ANCHOR_KEY_SIZE: u8 = 32;

/// Size of an anchor value (u64 big-endian block height = 8 bytes)
const ANCHOR_VALUE_SIZE: u32 = 8;

impl Drive {
    /// Registers the layer information for every tree a token shielded pool write can touch,
    /// from the root down to the pool's five children, so a batch touching the pool can be
    /// priced without state.
    pub(crate) fn add_estimation_costs_for_token_shielded_pool_operations(
        token_id: [u8; 32],
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    ) {
        // Root level: [] — the Tokens tree sits on layer 2 like the balances estimation assumes.
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path([]),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: EstimatedLevel(2, false),
                estimated_layer_sizes: AllSubtrees(1, NoSumTrees, None),
            },
        );

        // Tokens root: [16]
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(tokens_root_path()),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: EstimatedLevel(0, false),
                estimated_layer_sizes: AllSubtrees(1, AllBigSumTrees, None),
            },
        );

        // Token shielded pools root: [16, 224] — a BigSumTree of per-token pool SumTrees.
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(token_shielded_pools_root_path()),
            EstimatedLayerInformation {
                tree_type: TreeType::BigSumTree,
                estimated_layer_count: EstimatedLevel(10, false),
                estimated_layer_sizes: AllSubtrees(DEFAULT_HASH_SIZE_U8, AllSumTrees, None),
            },
        );

        // The token's pool: [16, 224, token_id] — SumTree of 4 subtrees + 1 SumItem, like the
        // credit pool (balanced Merk depth ceil(log2(5)) = 3).
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(token_shielded_pool_path_vec(token_id)),
            EstimatedLayerInformation {
                tree_type: TreeType::SumTree,
                estimated_layer_count: EstimatedLevel(3, false),
                estimated_layer_sizes: Mix {
                    subtrees_size: Some((
                        1,
                        SomeSumTrees {
                            sum_trees_weight: 0,
                            big_sum_trees_weight: 0,
                            count_trees_weight: 1, // nullifiers (ProvableCountTree)
                            count_sum_trees_weight: 0,
                            non_sum_trees_weight: 3, // notes, anchors, anchors-by-height
                            provable_sum_trees_weight: 0,
                            provable_count_trees_weight: 0,
                            provable_count_sum_trees_weight: 0,
                            provable_count_provable_sum_trees_weight: 0,
                        },
                        None,
                        4,
                    )),
                    items_size: Some((1, 8, None, 1)), // total balance SumItem
                    references_size: None,
                    items_with_sum_item_size: None,
                    references_with_sum_item_size: None,
                },
            },
        );

        // Notes tree: [16, 224, token_id, 128]
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(token_shielded_pool_notes_path_vec(token_id)),
            EstimatedLayerInformation {
                tree_type: TreeType::CommitmentTree(SHIELDED_NOTES_CHUNK_POWER),
                estimated_layer_count: EstimatedLevel(16, false),
                estimated_layer_sizes: AllItems(8, AVERAGE_NOTE_VALUE_SIZE, None),
            },
        );

        // Nullifiers tree: [16, 224, token_id, 64]
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(token_shielded_pool_nullifiers_path_vec(token_id)),
            EstimatedLayerInformation {
                tree_type: TreeType::ProvableCountTree,
                estimated_layer_count: EstimatedLevel(16, false),
                estimated_layer_sizes: AllItems(NULLIFIER_KEY_SIZE, 0, None),
            },
        );

        // Anchors tree: [16, 224, token_id, 192]
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(token_shielded_pool_anchors_path_vec(token_id)),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: EstimatedLevel(7, false),
                estimated_layer_sizes: AllItems(ANCHOR_KEY_SIZE, ANCHOR_VALUE_SIZE, None),
            },
        );

        // Anchors-by-height tree: [16, 224, token_id, 96]
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(token_shielded_pool_anchors_by_height_path_vec(
                token_id,
            )),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: EstimatedLevel(7, false),
                estimated_layer_sizes: AllItems(
                    ANCHOR_VALUE_SIZE as u8,
                    ANCHOR_KEY_SIZE as u32,
                    None,
                ),
            },
        );
    }
}
