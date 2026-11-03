use crate::drive::constants::AVERAGE_BALANCE_SIZE;
use crate::drive::contract::paths::contract_fee_pots_path_vec;
use crate::drive::prefunded_specialized_balances::prefunded_specialized_balances_path;
use crate::drive::Drive;
use crate::util::type_constants::DEFAULT_HASH_SIZE_U8;
use dpp::data_contract::document_type::action_fees::ContractFeePot;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::{EstimatedLevel, PotentiallyAtMaxElements};
use grovedb::EstimatedLayerInformation;
use grovedb::EstimatedLayerSizes::{AllItems, AllSubtrees};
use grovedb::EstimatedSumTrees::{AllSumTrees, SomeSumTrees};
use grovedb::TreeType;
use std::collections::HashMap;

impl Drive {
    #[inline(always)]
    pub(super) fn add_estimation_costs_for_contract_fee_pot_update_v0(
        pot: ContractFeePot,
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    ) {
        // The root layer is left alone when the batch already describes it: a document
        // operation of the same batch describes it as well as this does.
        estimated_costs_only_with_layer_info
            .entry(KeyInfoPath::from_known_path([]))
            .or_insert(EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                // The prefunded specialized balances tree is on the 3rd level
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

        // Three sum trees: the voting balances on top, the two kinds of fee pots below.
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(prefunded_specialized_balances_path()),
            EstimatedLayerInformation {
                tree_type: TreeType::SumTree,
                estimated_layer_count: EstimatedLevel(1, false),
                estimated_layer_sizes: AllSubtrees(1, AllSumTrees, None),
            },
        );

        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(contract_fee_pots_path_vec(pot)),
            EstimatedLayerInformation {
                tree_type: TreeType::SumTree,
                estimated_layer_count: PotentiallyAtMaxElements,
                estimated_layer_sizes: AllItems(DEFAULT_HASH_SIZE_U8, AVERAGE_BALANCE_SIZE, None),
            },
        );
    }
}
