use crate::drive::constants::AVERAGE_BALANCE_SIZE;
use crate::drive::credit_pools::epochs::paths::EpochProposers;
use crate::drive::credit_pools::pools_vec_path;
use crate::drive::prefunded_specialized_balances::{
    prefunded_specialized_balances_for_readiness_path_vec, prefunded_specialized_balances_path,
};
use crate::drive::votes::paths::{
    readiness_contract_tree_path_vec, readiness_contracts_tree_path_vec,
    readiness_deadline_tree_path_vec, readiness_deadlines_tree_path_vec,
    readiness_retired_rounds_tree_path_vec, readiness_round_reports_tree_path_vec,
    readiness_round_tree_path_vec, readiness_tree_path_vec, vote_root_path_vec,
};
use crate::drive::Drive;
use crate::util::type_constants::{DEFAULT_HASH_SIZE_U8, U64_SIZE_U8, U8_SIZE_U8};
use dpp::block::epoch::Epoch;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::{ApproximateElements, EstimatedLevel, PotentiallyAtMaxElements};
use grovedb::EstimatedLayerSizes::{AllItems, AllSubtrees, Mix};
use grovedb::EstimatedSumTrees::{AllSumTrees, NoSumTrees, SomeSumTrees};
use grovedb::{EstimatedLayerInformation, TreeType};
use std::collections::HashMap;

/// The serialized size of a readiness round record, rounded up: a versioned enum around two
/// identifiers, two 32-byte digests, a handful of integers, the status with two timestamps,
/// the optional evaluation mark and the typed payer.
pub(crate) const ESTIMATED_READINESS_ROUND_RECORD_SIZE: u32 = 200;

/// The serialized size of a readiness report record: a versioned enum around a height and a
/// profile.
pub(crate) const ESTIMATED_READINESS_REPORT_RECORD_SIZE: u32 = 16;

/// The serialized size of a readiness scan cursor: a versioned enum around two counters, an
/// optional key and three counts.
pub(crate) const ESTIMATED_READINESS_SCAN_CURSOR_SIZE: u32 = 56;

/// The number of readiness rounds we expect to be pending at once (one per contract with a
/// pending bundle).
const ESTIMATED_PENDING_ROUNDS: u32 = 1_024;

/// The number of reports we expect a round to hold at most: every evonode of the network.
const ESTIMATED_REPORTS_PER_ROUND: u32 = 4_096;

/// The number of entries of the pools tree: one epoch tree per epoch of the perpetual
/// storage window (50 eras of 40 epochs at the default era length) beside the pool items.
const ESTIMATED_POOL_ENTRIES: u32 = 2_048;

/// The number of entries of an epoch's tree: the proposers tree beside the pool, start and
/// multiplier items.
const ESTIMATED_EPOCH_ENTRIES: u32 = 9;

impl Drive {
    /// Describes every layer a readiness round write can touch: the root, the votes tree, the
    /// readiness tree, the contracts tree, the contract's tree (pointer item beside round
    /// trees), the round tree (record item, count tree, cursor item), the reports count tree,
    /// the deadlines tree and the retired rounds tree.
    pub(super) fn add_estimation_costs_for_readiness_v0(
        contract_id: [u8; 32],
        round_id: [u8; 32],
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    ) {
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path([]),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: EstimatedLevel(3, false),
                estimated_layer_sizes: AllSubtrees(1, NoSumTrees, None),
            },
        );

        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(vote_root_path_vec()),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: EstimatedLevel(2, false),
                estimated_layer_sizes: AllSubtrees(1, NoSumTrees, None),
            },
        );

        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(readiness_tree_path_vec()),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: EstimatedLevel(2, false),
                estimated_layer_sizes: Mix {
                    subtrees_size: Some((1, NoSumTrees, None, 3)),
                    items_size: Some((1, DEFAULT_HASH_SIZE_U8 as u32, None, 1)),
                    references_size: None,
                    items_with_sum_item_size: None,
                    references_with_sum_item_size: None,
                },
            },
        );

        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(readiness_contracts_tree_path_vec()),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: ApproximateElements(ESTIMATED_PENDING_ROUNDS),
                estimated_layer_sizes: AllSubtrees(DEFAULT_HASH_SIZE_U8, NoSumTrees, None),
            },
        );

        // A contract's tree holds the pointer item and normally one round tree, two during
        // the batch that replaces a round.
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(readiness_contract_tree_path_vec(contract_id)),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: ApproximateElements(3),
                estimated_layer_sizes: Mix {
                    subtrees_size: Some((DEFAULT_HASH_SIZE_U8, NoSumTrees, None, 2)),
                    items_size: Some((U8_SIZE_U8, DEFAULT_HASH_SIZE_U8 as u32, None, 1)),
                    references_size: None,
                    items_with_sum_item_size: None,
                    references_with_sum_item_size: None,
                },
            },
        );

        // A round's tree holds the record item, the reports count tree and possibly the
        // cursor item.
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(readiness_round_tree_path_vec(
                contract_id,
                round_id,
            )),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: ApproximateElements(3),
                estimated_layer_sizes: Mix {
                    subtrees_size: Some((
                        U8_SIZE_U8,
                        SomeSumTrees {
                            sum_trees_weight: 0,
                            big_sum_trees_weight: 0,
                            count_trees_weight: 1,
                            count_sum_trees_weight: 0,
                            non_sum_trees_weight: 0,
                            provable_sum_trees_weight: 0,
                            provable_count_trees_weight: 0,
                            provable_count_sum_trees_weight: 0,
                            provable_count_provable_sum_trees_weight: 0,
                        },
                        None,
                        1,
                    )),
                    items_size: Some((U8_SIZE_U8, ESTIMATED_READINESS_ROUND_RECORD_SIZE, None, 2)),
                    references_size: None,
                    items_with_sum_item_size: None,
                    references_with_sum_item_size: None,
                },
            },
        );

        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(readiness_round_reports_tree_path_vec(
                contract_id,
                round_id,
            )),
            EstimatedLayerInformation {
                tree_type: TreeType::CountTree,
                estimated_layer_count: ApproximateElements(ESTIMATED_REPORTS_PER_ROUND),
                estimated_layer_sizes: AllItems(
                    DEFAULT_HASH_SIZE_U8,
                    ESTIMATED_READINESS_REPORT_RECORD_SIZE,
                    None,
                ),
            },
        );

        // Deadlines: one subtree per distinct deadline time holding contract id -> round id
        // items. The block time granularity makes distinct times plentiful.
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(readiness_deadlines_tree_path_vec()),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: ApproximateElements(ESTIMATED_PENDING_ROUNDS),
                estimated_layer_sizes: AllSubtrees(U64_SIZE_U8, NoSumTrees, None),
            },
        );

        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(readiness_retired_rounds_tree_path_vec()),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: ApproximateElements(ESTIMATED_PENDING_ROUNDS),
                estimated_layer_sizes: AllItems(
                    DEFAULT_HASH_SIZE_U8,
                    DEFAULT_HASH_SIZE_U8 as u32,
                    None,
                ),
            },
        );
    }

    /// Describes the per-time tree of one activation deadline: contract id keys with round
    /// id items, a few per distinct time.
    pub(super) fn add_estimation_costs_for_readiness_deadline_v0(
        deadline_ms: u64,
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    ) {
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(readiness_deadline_tree_path_vec(deadline_ms)),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: ApproximateElements(2),
                estimated_layer_sizes: AllItems(
                    DEFAULT_HASH_SIZE_U8,
                    DEFAULT_HASH_SIZE_U8 as u32,
                    None,
                ),
            },
        );
    }

    /// Describes the layers a readiness fund write touches, mirroring the voting fund
    /// estimation on the sibling tree.
    pub(super) fn add_estimation_costs_for_readiness_fund_update_v0(
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
                prefunded_specialized_balances_for_readiness_path_vec(),
            ),
            EstimatedLayerInformation {
                tree_type: TreeType::SumTree,
                estimated_layer_count: PotentiallyAtMaxElements,
                estimated_layer_sizes: AllItems(DEFAULT_HASH_SIZE_U8, AVERAGE_BALANCE_SIZE, None),
            },
        );
    }

    /// Describes the layers a credit to an epoch's processing fee pool touches: the root
    /// (only when no earlier estimation described it), the pools sum tree of epoch trees and
    /// the epoch's sum tree, whose pool is a sum item beside the other epoch items.
    pub(super) fn add_estimation_costs_for_readiness_pool_credit_v0(
        epoch: &Epoch,
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    ) {
        estimated_costs_only_with_layer_info
            .entry(KeyInfoPath::from_known_path([]))
            .or_insert(EstimatedLayerInformation {
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
            });

        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(pools_vec_path()),
            EstimatedLayerInformation {
                tree_type: TreeType::SumTree,
                estimated_layer_count: ApproximateElements(ESTIMATED_POOL_ENTRIES),
                estimated_layer_sizes: AllSubtrees(2, AllSumTrees, None),
            },
        );

        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(epoch.get_path_vec()),
            EstimatedLayerInformation {
                tree_type: TreeType::SumTree,
                estimated_layer_count: ApproximateElements(ESTIMATED_EPOCH_ENTRIES),
                estimated_layer_sizes: Mix {
                    subtrees_size: Some((U8_SIZE_U8, NoSumTrees, None, 1)),
                    items_size: Some((U8_SIZE_U8, U64_SIZE_U8 as u32, None, 8)),
                    references_size: None,
                    items_with_sum_item_size: None,
                    references_with_sum_item_size: None,
                },
            },
        );
    }
}
