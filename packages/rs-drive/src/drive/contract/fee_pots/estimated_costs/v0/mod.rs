use crate::drive::constants::AVERAGE_BALANCE_SIZE;
use crate::drive::contract::paths::contract_fee_pots_path_vec;
use crate::drive::prefunded_specialized_balances::prefunded_specialized_balances_path;
use crate::drive::Drive;
use crate::util::estimated_layers::EstimatedLayers;
use crate::util::type_constants::DEFAULT_HASH_SIZE_U8;
use dpp::data_contract::document_type::action_fees::ContractFeePot;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::{EstimatedLevel, PotentiallyAtMaxElements};
use grovedb::EstimatedLayerInformation;
use grovedb::EstimatedLayerSizes::{AllItems, AllSubtrees};
use grovedb::EstimatedSumTrees::{AllSumTrees, SomeSumTrees};
use grovedb::TreeType;

impl Drive {
    #[inline(always)]
    pub(super) fn add_estimation_costs_for_contract_fee_pot_update_v0(
        pot: ContractFeePot,
        estimated_costs_only_with_layer_info: &mut EstimatedLayers,
    ) {
        // The root is written only when nothing described it yet; otherwise this description is
        // kept among the root's other descriptions. An estimate that is not resolved (protocol
        // version 14 and earlier) prices the earlier description: in an action fee's batch, the
        // identity balance write's level 1, as the pot write comes after it, so the pot write's
        // climb through the root is priced two levels short. A resolved estimate
        // (`apply_drive_operations` 2, protocol version 15) prices the root at the costliest of
        // all its descriptions, this one included.
        estimated_costs_only_with_layer_info.insert_if_absent(
            KeyInfoPath::from_known_path([]),
            EstimatedLayerInformation {
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
            },
        );

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

#[cfg(test)]
mod tests {
    use crate::drive::Drive;
    use crate::util::estimated_layers::EstimatedLayers;
    use dpp::data_contract::document_type::action_fees::ContractFeePot;
    use dpp::version::PlatformVersion;
    use grovedb::batch::KeyInfoPath;
    use grovedb::EstimatedLayerCount::EstimatedLevel;
    use grovedb::EstimatedLayerInformation;
    use grovedb::EstimatedLayerSizes::AllSubtrees;
    use grovedb::EstimatedSumTrees::NoSumTrees;
    use grovedb::TreeType;

    /// A root an earlier write described keeps that description in an estimate that is not
    /// resolved, as before protocol version 15; a resolved estimate prices the root at the pot
    /// write's level 3.
    #[test]
    fn should_leave_a_described_root_unless_the_estimate_is_resolved() {
        let root = KeyInfoPath::from_known_path([]);
        let balance_root = EstimatedLayerInformation {
            tree_type: TreeType::NormalTree,
            estimated_layer_count: EstimatedLevel(1, false),
            estimated_layer_sizes: AllSubtrees(1, NoSumTrees, None),
        };
        let mut estimate = EstimatedLayers::new();
        estimate.insert(root.clone(), balance_root);
        Drive::add_estimation_costs_for_contract_fee_pot_update_v0(
            ContractFeePot::Owner,
            &mut estimate,
        );
        assert_eq!(estimate.get(&root), Some(&balance_root));
        assert_eq!(
            estimate.clone().into_layers().get(&root),
            Some(&balance_root)
        );

        let resolved = estimate
            .resolve(PlatformVersion::latest())
            .expect("expected to resolve the estimate");
        assert_eq!(
            resolved
                .get(&root)
                .expect("expected the root to be described")
                .estimated_layer_count
                .estimate_levels(),
            3
        );
    }
}
