use crate::drive::Drive;

use crate::drive::group::paths::{group_contract_path, group_path, group_root_path};
use crate::util::type_constants::DEFAULT_HASH_SIZE_U8;
use dpp::data_contract::group::Group;
use dpp::data_contract::GroupContractPosition;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::EstimatedLevel;
use grovedb::EstimatedLayerSizes::AllSubtrees;
use grovedb::EstimatedSumTrees::{NoSumTrees, SomeSumTrees};
use grovedb::{EstimatedLayerInformation, TreeType};
use std::collections::{BTreeMap, HashMap};

impl Drive {
    pub(super) fn add_estimation_costs_for_add_groups_v0(
        contract_id: [u8; 32],
        groups: &BTreeMap<GroupContractPosition, Group>,
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    ) {
        //                                                                                DataContract_Documents 64
        //                                 /                                                                                                       \
        //                       Identities 32                                                                                                 Balances 96
        //             /                            \                                                                        /                                                       \
        //   Token_Balances 16                    Pools 48                                                    WithdrawalTransactions 80                                                Votes  112
        //       /      \                           /                     \                                         /                           \                            /                          \
        //     NUPKH->I 8 UPKH->I 24   PreFundedSpecializedBalances 40  Masternode Lists 56 (reserved)     SpentAssetLockTransactions 72    GroupActions 88             Misc 104                        Versions 120

        // we have constructed the top layer so contract/documents tree are at the top
        // since balance will be on layer 3 (level 2 on left then left)
        // updating will mean we will update:
        // 1 normal tree (group actions)
        // 1 normal tree (withdrawal transactions)
        // 1 sum tree (balances)
        // 1 normal tree (contract/documents)
        // hence we should give an equal weight to both
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
                        non_sum_trees_weight: 2,
                    },
                    None,
                ),
            },
        );

        // there is one tree for the root path
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(group_root_path()),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: EstimatedLevel(10, false), // We estimate that on average we need to update 10 nodes
                estimated_layer_sizes: AllSubtrees(DEFAULT_HASH_SIZE_U8, NoSumTrees, None),
            },
        );

        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(group_contract_path(contract_id.as_slice())),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: EstimatedLevel(1, false),
                estimated_layer_sizes: AllSubtrees(2, NoSumTrees, None),
            },
        );

        for group_contract_position in groups.keys() {
            estimated_costs_only_with_layer_info.insert(
                KeyInfoPath::from_known_path(group_path(
                    contract_id.as_slice(),
                    &group_contract_position.to_be_bytes(),
                )),
                EstimatedLayerInformation {
                    tree_type: TreeType::NormalTree,
                    estimated_layer_count: EstimatedLevel(1, false),
                    estimated_layer_sizes: AllSubtrees(1, NoSumTrees, None),
                },
            );
        }
    }
}
