use crate::drive::Drive;

use crate::drive::group::paths::{
    group_action_signers_path, group_active_action_path, group_active_action_root_path,
    group_closed_action_path, group_closed_action_root_path, group_closed_action_signers_path,
    group_contract_path, group_path, group_root_path,
};
use crate::util::type_constants::DEFAULT_HASH_SIZE_U8;
use dpp::data_contract::GroupContractPosition;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::EstimatedLevel;
use grovedb::EstimatedLayerSizes::{AllItems, AllSubtrees, Mix};
use grovedb::EstimatedSumTrees::{AllSumTrees, NoSumTrees, SomeSumTrees};
use grovedb::{EstimatedLayerInformation, TreeType};
use std::collections::HashMap;

impl Drive {
    /// Adds estimated costs for the layer information of group action updates in a contract.
    ///
    /// This function updates the `estimated_costs_only_with_layer_info` map with the layer information for
    /// the trees involved in adding or updating a group action in the context of a contract. The trees are
    /// organized hierarchically based on their role in the system, such as "Group Actions", "Withdrawal Transactions",
    /// "Balances", and "Contract/Documents". This estimation is used to determine the computational costs associated
    /// with updating these trees, considering whether they are sum trees or normal trees and their expected layer counts.
    ///
    /// The function breaks down the tree layers and their corresponding costs as follows:
    /// 1. **Group Actions Tree**: A normal tree that holds information about group actions in the contract.
    /// 2. **Withdrawal Transactions Tree**: A normal tree that holds withdrawal transaction data.
    /// 3. **Balances Tree**: A sum tree that holds balance information, which is crucial for cost estimation.
    /// 4. **Contract/Documents Tree**: A normal tree that holds contract and document-related data.
    ///
    /// Each tree's cost is estimated based on its depth and whether it's a sum tree or not. The function inserts the
    /// estimated layer information for each relevant tree in the `estimated_costs_only_with_layer_info` map, where
    /// the key represents the path to the specific tree and the value represents its estimated layer information.
    ///
    /// # Parameters
    ///
    /// - `contract_id`: The unique identifier of the contract being updated. Used to construct paths for the trees.
    /// - `group_contract_position`: The position of the group contract in the system, used to further specify paths.
    /// - `action_id`: An optional identifier for the specific action being added. If provided, additional paths for
    ///   the action and its associated signers will be included.
    /// - `estimated_costs_only_with_layer_info`: A mutable reference to a `HashMap` where the estimated layer information
    ///   will be inserted. The keys represent paths to the trees, and the values represent their estimated layer information.
    ///
    /// # Logic Breakdown
    ///
    /// - **Top Layer (Contract/Documents)**: The contract and documents tree is at the top level, with a weight of 2.
    /// - **Balance Tree (Sum Tree)**: The balance tree is a sum tree with a weight of 1.
    /// - **Withdrawal Transactions**: This tree is a normal tree, and it is expected to have a weight of 2.
    /// - **Group Action Tree**: The group action tree is also a normal tree, with an expected weight of 2.
    /// - **Additional Layer Costs**: For specific paths related to actions, signers, etc., further estimations are added with
    ///   appropriate layer counts and subtree size estimations.
    ///
    /// The function constructs the paths based on the contract ID, group contract position, and action ID (if provided).
    /// It then populates the `estimated_costs_only_with_layer_info` map with the estimated costs for each relevant tree
    /// involved in the group action update.
    pub(super) fn add_estimation_costs_for_add_group_action_v0(
        contract_id: [u8; 32],
        group_contract_position: GroupContractPosition,
        action_id: Option<[u8; 32]>,
        also_add_closed_tree: bool,
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

        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(group_active_action_root_path(
                contract_id.as_slice(),
                &group_contract_position.to_be_bytes(),
            )),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: EstimatedLevel(10, false),
                estimated_layer_sizes: AllSubtrees(DEFAULT_HASH_SIZE_U8, NoSumTrees, None),
            },
        );

        if let Some(action_id) = action_id {
            estimated_costs_only_with_layer_info.insert(
                KeyInfoPath::from_known_path(group_active_action_path(
                    contract_id.as_slice(),
                    &group_contract_position.to_be_bytes(),
                    action_id.as_slice(),
                )),
                EstimatedLayerInformation {
                    tree_type: TreeType::NormalTree,
                    estimated_layer_count: EstimatedLevel(1, false),
                    estimated_layer_sizes: Mix {
                        subtrees_size: Some((1, AllSumTrees, None, 1)),
                        items_size: Some((1, 8, Some(36), 1)),
                        references_size: None,
                    },
                },
            );

            estimated_costs_only_with_layer_info.insert(
                KeyInfoPath::from_known_path(group_action_signers_path(
                    contract_id.as_slice(),
                    &group_contract_position.to_be_bytes(),
                    action_id.as_slice(),
                )),
                EstimatedLayerInformation {
                    tree_type: TreeType::SumTree,
                    estimated_layer_count: EstimatedLevel(1, false),
                    estimated_layer_sizes: AllItems(8, 1, None),
                },
            );
        }

        if also_add_closed_tree {
            estimated_costs_only_with_layer_info.insert(
                KeyInfoPath::from_known_path(group_closed_action_root_path(
                    contract_id.as_slice(),
                    &group_contract_position.to_be_bytes(),
                )),
                EstimatedLayerInformation {
                    tree_type: TreeType::NormalTree,
                    estimated_layer_count: EstimatedLevel(10, false),
                    estimated_layer_sizes: AllSubtrees(DEFAULT_HASH_SIZE_U8, NoSumTrees, None),
                },
            );

            if let Some(action_id) = action_id {
                estimated_costs_only_with_layer_info.insert(
                    KeyInfoPath::from_known_path(group_closed_action_path(
                        contract_id.as_slice(),
                        &group_contract_position.to_be_bytes(),
                        action_id.as_slice(),
                    )),
                    EstimatedLayerInformation {
                        tree_type: TreeType::NormalTree,
                        estimated_layer_count: EstimatedLevel(1, false),
                        estimated_layer_sizes: Mix {
                            subtrees_size: Some((1, AllSumTrees, None, 1)),
                            items_size: Some((1, 8, Some(36), 1)),
                            references_size: None,
                        },
                    },
                );

                estimated_costs_only_with_layer_info.insert(
                    KeyInfoPath::from_known_path(group_closed_action_signers_path(
                        contract_id.as_slice(),
                        &group_contract_position.to_be_bytes(),
                        action_id.as_slice(),
                    )),
                    EstimatedLayerInformation {
                        tree_type: TreeType::SumTree,
                        estimated_layer_count: EstimatedLevel(1, false),
                        estimated_layer_sizes: AllItems(8, 1, None),
                    },
                );
            }
        }
    }
}
