use crate::drive::constants::AVERAGE_BALANCE_SIZE;
use crate::drive::prefunded_specialized_balances::{
    prefunded_specialized_balances_for_token_orders_path_vec, prefunded_specialized_balances_path,
};
use crate::drive::tokens::paths::{
    token_order_escrow_path, token_order_escrow_root_path, tokens_root_path,
};
use crate::drive::Drive;
use crate::util::estimated_layers::insert_estimated_layer_keeping_deeper;
use crate::util::estimated_layers::EstimatedLayers;
use crate::util::type_constants::DEFAULT_HASH_SIZE_U8;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::{EstimatedLevel, PotentiallyAtMaxElements};
use grovedb::EstimatedLayerSizes::{AllItems, AllSubtrees};
use grovedb::EstimatedSumTrees::{AllBigSumTrees, AllSumTrees, NoSumTrees, SomeSumTrees};
use grovedb::{EstimatedLayerInformation, TreeType};

/// The level of the Tokens Merk the escrow key sits on.
const TOKEN_ORDER_ESCROW_LEVEL_IN_TOKENS: u32 = 3;

/// The level of the prefunded specialized balances Merk the offers' escrow key sits on: its root
/// is the voting balances, 128, over the fee pots, 64 and 192, and the escrow key, 32, sits below
/// 64.
const CREDIT_ORDER_ESCROW_LEVEL_IN_PREFUNDED_BALANCES: u32 = 2;

impl Drive {
    /// The layers a write to one listing's escrow entry goes through: the root, the Tokens
    /// tree, where the escrow BigSumTree is a leaf on the third level, the escrow BigSumTree of
    /// one SumTree per token, and the token's SumTree of one entry per open listing.
    pub(super) fn add_estimation_costs_for_token_order_escrow_v0(
        token_id: [u8; 32],
        estimated_costs_only_with_layer_info: &mut EstimatedLayers,
    ) {
        // Every description below keeps a deeper one another write of the batch registered.
        insert_estimated_layer_keeping_deeper(
            estimated_costs_only_with_layer_info,
            KeyInfoPath::from_known_path([]),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: EstimatedLevel(2, false),
                estimated_layer_sizes: AllSubtrees(1, NoSumTrees, None),
            },
        );

        // The escrow key sits deeper in the Tokens Merk than the balances key the token balance
        // estimation describes at level 0, and a batch moving tokens into or out of escrow writes
        // both. From protocol version 15 the token balance estimation keeps this deeper
        // description too (`for_token_balances` 1), whichever registers first.
        insert_estimated_layer_keeping_deeper(
            estimated_costs_only_with_layer_info,
            KeyInfoPath::from_known_path(tokens_root_path()),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: EstimatedLevel(TOKEN_ORDER_ESCROW_LEVEL_IN_TOKENS, false),
                estimated_layer_sizes: AllSubtrees(1, AllBigSumTrees, None),
            },
        );

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
    /// specialized balances, where the escrow key is a leaf on the third level, and the SumTree
    /// of one entry per open offer. The prefunded balance and contract fee pot estimations
    /// describe the prefunded balances layer at level 1, their keys' depth: from protocol version
    /// 15 they keep a deeper description too, so a batch writing to the escrow and to one of
    /// them is priced at the escrow's depth whichever registers first.
    pub(super) fn add_estimation_costs_for_credit_order_escrow_v0(
        estimated_costs_only_with_layer_info: &mut EstimatedLayers,
    ) {
        insert_estimated_layer_keeping_deeper(
            estimated_costs_only_with_layer_info,
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

        insert_estimated_layer_keeping_deeper(
            estimated_costs_only_with_layer_info,
            KeyInfoPath::from_known_path(prefunded_specialized_balances_path()),
            EstimatedLayerInformation {
                tree_type: TreeType::SumTree,
                estimated_layer_count: EstimatedLevel(
                    CREDIT_ORDER_ESCROW_LEVEL_IN_PREFUNDED_BALANCES,
                    false,
                ),
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
