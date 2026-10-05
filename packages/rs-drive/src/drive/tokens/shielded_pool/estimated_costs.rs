// A token pool stores the same five items as the credit pool, so it is sized from the same
// constants: a divergence here would price one pool's writes wrong while the other stays right.
use crate::drive::shielded::estimated_costs::{
    ANCHOR_KEY_SIZE, ANCHOR_VALUE_SIZE, AVERAGE_NOTE_VALUE_SIZE, NULLIFIER_KEY_SIZE,
};
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
use grovedb::EstimatedSumTrees::{AllBigSumTrees, AllSumTrees, SomeSumTrees};
use grovedb::{EstimatedLayerInformation, TreeType};
use std::collections::HashMap;

impl Drive {
    /// Registers the layer information for every tree a token shielded pool write can touch,
    /// from the root down to the pool's five children, so a batch touching the pool can be
    /// priced without state.
    pub(crate) fn add_estimation_costs_for_token_shielded_pool_operations(
        token_id: [u8; 32],
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    ) {
        // Root level: []. This MUST stay identical to the credit pool's root registration in
        // `crate::drive::shielded::estimated_costs`, because the three identity-less token pool
        // transitions write both into one estimation map under this one key, and the later write
        // wins. A divergence would make the surviving estimate depend on the order the operations
        // happen to be converted in.
        //
        // `NoSumTrees` was also untrue of the root, whoever is asking: `Balances`, `Tokens` and
        // `PreFundedSpecializedBalances` are sum trees directly under it. Of the two descriptions
        // this one is the larger, which is the safe direction — `validate_fees_of_event` prices a
        // batch with this estimated model during block execution and requires `estimated >= actual`.
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path([]),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: EstimatedLevel(3, false),
                estimated_layer_sizes: AllSubtrees(
                    1,
                    SomeSumTrees {
                        sum_trees_weight: 2,
                        big_sum_trees_weight: 0,
                        count_trees_weight: 0,
                        count_sum_trees_weight: 0,
                        non_sum_trees_weight: 2,
                        provable_sum_trees_weight: 0,
                        provable_count_trees_weight: 0,
                        provable_count_sum_trees_weight: 0,
                        provable_count_provable_sum_trees_weight: 0,
                    },
                    None,
                ),
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
        //
        // The sixteen levels model roughly 65 000 keys, and the estimate stays at or above the
        // real cost only while the tree is shallower than that. It fills about twice as fast as
        // the depth was chosen for: spends contribute one nullifier each, but every inflow
        // bundle now also records a dummy nullifier per action, up to sixteen. A pool that
        // outgrows the model does not merely misprice — the estimate falls below the actual
        // cost and fee validation refuses the block, so the ceiling is a halting condition and
        // not a rounding one. Raise the level before a pool approaches it.
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
