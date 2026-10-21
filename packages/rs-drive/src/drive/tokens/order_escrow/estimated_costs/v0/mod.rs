use crate::drive::constants::AVERAGE_BALANCE_SIZE;
use crate::drive::prefunded_specialized_balances::{
    prefunded_specialized_balances_for_token_orders_path_vec, prefunded_specialized_balances_path,
};
use crate::drive::tokens::paths::{
    token_order_escrow_path, token_order_escrow_root_path, tokens_root_path,
};
use crate::drive::Drive;
use crate::util::type_constants::DEFAULT_HASH_SIZE_U8;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::{EstimatedLevel, PotentiallyAtMaxElements};
use grovedb::EstimatedLayerSizes::{AllItems, AllSubtrees};
use grovedb::EstimatedSumTrees::{AllBigSumTrees, AllSumTrees, NoSumTrees, SomeSumTrees};
use grovedb::{EstimatedLayerInformation, TreeType};
use std::collections::HashMap;

/// The level of the Tokens Merk the escrow key sits on.
const TOKEN_ORDER_ESCROW_LEVEL_IN_TOKENS: u32 = 3;

impl Drive {
    /// The layers a write to one listing's escrow entry goes through: the root, the Tokens
    /// tree, where the escrow BigSumTree is a leaf on the third level, the escrow BigSumTree of
    /// one SumTree per token, and the token's SumTree of one entry per open listing.
    pub(super) fn add_estimation_costs_for_token_order_escrow_v0(
        token_id: [u8; 32],
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    ) {
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path([]),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: EstimatedLevel(2, false),
                estimated_layer_sizes: AllSubtrees(1, NoSumTrees, None),
            },
        );

        // The escrow key sits deeper in the Tokens Merk than the balances key the token balance
        // estimation describes at level 0, and a batch moving tokens into or out of escrow writes
        // both. One description covers the whole layer, so the deeper one is kept whichever is
        // registered first: this estimation never lowers what is there. The balance estimation
        // overwrites what it finds and belongs to every earlier protocol version's fees, so the
        // escrow operations register theirs after it (`TokenPlaceInOrderEscrow`,
        // `TokenReleaseFromOrderEscrow`).
        let tokens_layer = EstimatedLayerInformation {
            tree_type: TreeType::NormalTree,
            estimated_layer_count: EstimatedLevel(TOKEN_ORDER_ESCROW_LEVEL_IN_TOKENS, false),
            estimated_layer_sizes: AllSubtrees(1, AllBigSumTrees, None),
        };
        match estimated_costs_only_with_layer_info
            .get(&KeyInfoPath::from_known_path(tokens_root_path()))
            .map(|existing| &existing.estimated_layer_count)
        {
            Some(EstimatedLevel(level, _)) if *level >= TOKEN_ORDER_ESCROW_LEVEL_IN_TOKENS => {}
            _ => {
                estimated_costs_only_with_layer_info.insert(
                    KeyInfoPath::from_known_path(tokens_root_path()),
                    tokens_layer,
                );
            }
        }

        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(token_order_escrow_root_path()),
            EstimatedLayerInformation {
                tree_type: TreeType::BigSumTree,
                estimated_layer_count: EstimatedLevel(10, false),
                estimated_layer_sizes: AllSubtrees(DEFAULT_HASH_SIZE_U8, AllSumTrees, None),
            },
        );

        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(token_order_escrow_path(&token_id)),
            EstimatedLayerInformation {
                tree_type: TreeType::SumTree,
                estimated_layer_count: PotentiallyAtMaxElements,
                estimated_layer_sizes: AllItems(DEFAULT_HASH_SIZE_U8, AVERAGE_BALANCE_SIZE, None),
            },
        );
    }

    /// The layers a write to one offer's escrowed credits goes through: the root, the prefunded
    /// specialized balances, described as the prefunded balance and contract fee pot estimations
    /// describe it so a batch writing to several of its trees gets one answer, and the SumTree of
    /// one entry per open offer.
    pub(super) fn add_estimation_costs_for_credit_order_escrow_v0(
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    ) {
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path([]),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: EstimatedLevel(3, false),
                estimated_layer_sizes: AllSubtrees(
                    1,
                    SomeSumTrees {
                        sum_trees_weight: 1,
                        big_sum_trees_weight: 0,
                        count_trees_weight: 0,
                        count_sum_trees_weight: 0,
                        non_sum_trees_weight: 1,
                        provable_sum_trees_weight: 0,
                        provable_count_trees_weight: 0,
                        provable_count_sum_trees_weight: 0,
                        provable_count_provable_sum_trees_weight: 0,
                    },
                    None,
                ),
            },
        );

        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(prefunded_specialized_balances_path()),
            EstimatedLayerInformation {
                tree_type: TreeType::SumTree,
                estimated_layer_count: EstimatedLevel(1, false),
                estimated_layer_sizes: AllSubtrees(1, AllSumTrees, None),
            },
        );

        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(
                prefunded_specialized_balances_for_token_orders_path_vec(),
            ),
            EstimatedLayerInformation {
                tree_type: TreeType::SumTree,
                estimated_layer_count: PotentiallyAtMaxElements,
                estimated_layer_sizes: AllItems(DEFAULT_HASH_SIZE_U8, AVERAGE_BALANCE_SIZE, None),
            },
        );
    }
}
