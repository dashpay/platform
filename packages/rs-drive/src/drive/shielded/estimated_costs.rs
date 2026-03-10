use crate::drive::shielded::paths::{
    shielded_credit_pool_anchors_path, shielded_credit_pool_notes_path,
    shielded_credit_pool_nullifiers_path, shielded_credit_pool_path, SHIELDED_NOTES_CHUNK_POWER,
};
use crate::drive::{Drive, RootTree};
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::EstimatedLevel;
use grovedb::EstimatedLayerSizes::{AllItems, AllSubtrees, Mix};
use grovedb::EstimatedSumTrees::SomeSumTrees;
use grovedb::{EstimatedLayerInformation, TreeType};
use std::collections::HashMap;

/// Average size of a note value: 32 cmx + 32 rho + 216 encrypted note = 280 bytes
/// (encrypted note = 32 epk + 104 enc_ciphertext + 80 out_ciphertext, using DashMemo 36-byte memos)
/// The cmx and rho are prepended by GroveDB's commitment_tree_insert_op for client retrieval.
const AVERAGE_NOTE_VALUE_SIZE: u32 = 280;

/// Size of a nullifier key (32 bytes)
const NULLIFIER_KEY_SIZE: u8 = 32;

/// Size of an anchor block height key (u64 big-endian = 8 bytes)
const ANCHOR_KEY_SIZE: u8 = 8;

/// Size of an anchor value (32 bytes)
const ANCHOR_VALUE_SIZE: u32 = 32;

impl Drive {
    /// Adds estimation costs for shielded pool operations.
    ///
    /// Registers all shielded pool tree paths in the estimation cache so that
    /// the fee estimation system can calculate costs for shielded operations.
    /// Also registers parent paths (root and AddressBalances) needed by the
    /// GroveDB cost estimation system.
    pub(crate) fn add_estimation_costs_for_shielded_pool_operations(
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    ) {
        // Root level: [] — needed so GroveDB can estimate traversal to AddressBalances
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
                    },
                    None,
                ),
            },
        );

        // AddressBalances level: [[56]] — parent of shielded pool subtree
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(vec![vec![RootTree::AddressBalances as u8]]),
            EstimatedLayerInformation {
                tree_type: TreeType::SumTree,
                estimated_layer_count: EstimatedLevel(2, false),
                estimated_layer_sizes: AllSubtrees(
                    1,
                    SomeSumTrees {
                        sum_trees_weight: 1,
                        big_sum_trees_weight: 0,
                        count_trees_weight: 0,
                        count_sum_trees_weight: 1,
                        non_sum_trees_weight: 0,
                    },
                    None,
                ),
            },
        );

        // Shielded credit pool: [AddressBalances, "s"]
        // SumTree containing: notes (CommitmentTree), permanent nullifiers (ProvableCountTree),
        // total balance (SumItem), anchors (NormalTree), recent nullifiers (CountSumTree),
        // compacted nullifiers (NormalTree), expiration time (NormalTree)
        // 7 elements total (6 subtrees + 1 item) → balanced Merk depth = ceil(log2(8)) = 3
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(shielded_credit_pool_path()),
            EstimatedLayerInformation {
                tree_type: TreeType::SumTree,
                estimated_layer_count: EstimatedLevel(3, false),
                estimated_layer_sizes: Mix {
                    subtrees_size: Some((
                        1,
                        SomeSumTrees {
                            sum_trees_weight: 0,
                            big_sum_trees_weight: 0,
                            count_trees_weight: 1, // permanent nullifiers (ProvableCountTree)
                            count_sum_trees_weight: 1, // recent nullifiers (CountSumTree)
                            non_sum_trees_weight: 4, // notes (CommitmentTree), anchors, compacted nullifiers, expiration time
                        },
                        None,
                        6, // 6 subtrees: notes, permanent nullifiers, anchors, recent nullifiers, compacted nullifiers, expiration time
                    )),
                    items_size: Some((1, 8, None, 1)), // 1 item: total balance
                    references_size: None,
                },
            },
        );

        // Notes tree: [AddressBalances, "s", 1]
        // CommitmentTree - stores notes (cmx||encrypted_note items + Sinsemilla frontier)
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(shielded_credit_pool_notes_path()),
            EstimatedLayerInformation {
                tree_type: TreeType::CommitmentTree(SHIELDED_NOTES_CHUNK_POWER),
                estimated_layer_count: EstimatedLevel(16, false),
                estimated_layer_sizes: AllItems(8, AVERAGE_NOTE_VALUE_SIZE, None),
            },
        );

        // Nullifiers tree: [AddressBalances, "s", 2]
        // ProvableCountTree - stores spent nullifiers (32-byte key -> empty item)
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(shielded_credit_pool_nullifiers_path()),
            EstimatedLayerInformation {
                tree_type: TreeType::ProvableCountTree,
                estimated_layer_count: EstimatedLevel(16, false),
                estimated_layer_sizes: AllItems(NULLIFIER_KEY_SIZE, 0, None),
            },
        );

        // Anchors tree: [AddressBalances, "s", 6]
        // NormalTree - stores block_height_be -> anchor_bytes
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(shielded_credit_pool_anchors_path()),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: EstimatedLevel(7, false),
                estimated_layer_sizes: AllItems(ANCHOR_KEY_SIZE, ANCHOR_VALUE_SIZE, None),
            },
        );
    }
}
