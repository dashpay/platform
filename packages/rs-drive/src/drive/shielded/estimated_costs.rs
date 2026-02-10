use crate::drive::shielded::paths::{
    shielded_anchors_credit_pool_path, shielded_anchors_path, shielded_credit_pool_commitments_path,
    shielded_credit_pool_encrypted_notes_path, shielded_credit_pool_nullifiers_path,
    shielded_credit_pool_path,
};
use crate::drive::Drive;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::EstimatedLevel;
use grovedb::EstimatedLayerSizes::{AllItems, AllSubtrees, Mix};
use grovedb::EstimatedSumTrees::{NoSumTrees, SomeSumTrees};
use grovedb::{EstimatedLayerInformation, TreeType};
use std::collections::HashMap;

/// Average size of an encrypted note (~692 bytes: 32 epk + 580 enc + 80 out)
const AVERAGE_ENCRYPTED_NOTE_SIZE: u32 = 692;

/// Size of a commitment or nullifier key (32 bytes)
const COMMITMENT_KEY_SIZE: u8 = 32;

/// Size of an anchor key (32 bytes)
const ANCHOR_KEY_SIZE: u8 = 32;

impl Drive {
    /// Adds estimation costs for shielded pool operations.
    ///
    /// Registers all shielded pool tree paths in the estimation cache so that
    /// the fee estimation system can calculate costs for shielded operations.
    pub(crate) fn add_estimation_costs_for_shielded_pool_operations(
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    ) {
        // Shielded credit pool: [AddressBalances, "s"]
        // SumTree containing: commitments tree (CommitmentTree), nullifiers tree (NormalTree),
        // encrypted notes tree (NormalTree), params item, total balance sum item
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(shielded_credit_pool_path()),
            EstimatedLayerInformation {
                tree_type: TreeType::SumTree,
                estimated_layer_count: EstimatedLevel(5, false),
                estimated_layer_sizes: Mix {
                    subtrees_size: Some((
                        1,
                        SomeSumTrees {
                            sum_trees_weight: 0,
                            big_sum_trees_weight: 0,
                            count_trees_weight: 0,
                            count_sum_trees_weight: 0,
                            non_sum_trees_weight: 3,
                        },
                        None,
                        3, // 3 subtrees: commitments, nullifiers, encrypted notes
                    )),
                    items_size: Some((1, 100, None, 2)), // 2 items: params and total balance
                    references_size: None,
                },
            },
        );

        // Commitments tree: [AddressBalances, "s", 1]
        // CommitmentTree - stores Orchard note commitments
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(shielded_credit_pool_commitments_path()),
            EstimatedLayerInformation {
                tree_type: TreeType::CommitmentTree,
                estimated_layer_count: EstimatedLevel(10, false),
                estimated_layer_sizes: AllItems(COMMITMENT_KEY_SIZE, 32, None),
            },
        );

        // Nullifiers tree: [AddressBalances, "s", 2]
        // NormalTree - stores spent nullifiers (32-byte key → empty item)
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(shielded_credit_pool_nullifiers_path()),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: EstimatedLevel(10, false),
                estimated_layer_sizes: AllItems(COMMITMENT_KEY_SIZE, 0, None),
            },
        );

        // Encrypted notes tree: [AddressBalances, "s", 3]
        // NormalTree - stores encrypted notes (32-byte key → ~692-byte item)
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(shielded_credit_pool_encrypted_notes_path()),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: EstimatedLevel(10, false),
                estimated_layer_sizes: AllItems(
                    COMMITMENT_KEY_SIZE,
                    AVERAGE_ENCRYPTED_NOTE_SIZE,
                    None,
                ),
            },
        );

        // Anchors tree: [AddressBalances, "a"]
        // NormalTree - contains subtrees per pool type
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(shielded_anchors_path()),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: EstimatedLevel(1, false),
                estimated_layer_sizes: AllSubtrees(1, NoSumTrees, None),
            },
        );

        // Anchors credit pool tree: [AddressBalances, "a", "s"]
        // NormalTree - stores anchor hashes (32-byte key → empty item)
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(shielded_anchors_credit_pool_path()),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: EstimatedLevel(10, false),
                estimated_layer_sizes: AllItems(ANCHOR_KEY_SIZE, 0, None),
            },
        );
    }
}
