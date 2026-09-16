use crate::drive::contract_groups::estimated_costs::{
    CONTRACT_GROUP_BACKWARDS_REFERENCE_SIZE, CONTRACT_GROUP_INFO_ESTIMATED_SIZE,
    DOCUMENT_TYPE_NAME_ESTIMATED_KEY_SIZE,
};
use crate::drive::contract_groups::paths::{
    contract_group_contracts_path, contract_group_document_types_for_contract_path,
    contract_group_document_types_path, contract_group_path,
    contract_group_tokens_for_contract_path, contract_group_tokens_path,
    contract_groups_groups_path, contract_groups_members_path,
    contract_memberships_document_type_path, contract_memberships_document_types_path,
    contract_memberships_groups_path, contract_memberships_path, contract_memberships_token_path,
    contract_memberships_tokens_path,
};
use crate::drive::Drive;
use crate::util::type_constants::DEFAULT_HASH_SIZE_U8;
use dpp::contract_group::{ContractGroupMember, ContractGroupMembership};
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::{ApproximateElements, EstimatedLevel};
use grovedb::EstimatedLayerSizes::{AllItems, AllReference, AllSubtrees, Mix};
use grovedb::EstimatedSumTrees::NoSumTrees;
use grovedb::{EstimatedLayerInformation, TreeType};
use std::collections::HashMap;

impl Drive {
    pub(super) fn add_estimation_costs_for_insert_contract_group_memberships_v0(
        contract_id: [u8; 32],
        memberships: &[ContractGroupMembership],
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    ) {
        Self::add_estimation_costs_for_contract_groups_root_layers(
            estimated_costs_only_with_layer_info,
        );

        // [ContractGroups, Groups] holds one subtree per contract group, keyed by id.
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(contract_groups_groups_path()),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: EstimatedLevel(10, false),
                estimated_layer_sizes: AllSubtrees(DEFAULT_HASH_SIZE_U8, NoSumTrees, None),
            },
        );

        // [ContractGroups, Members] holds one subtree per member contract, keyed by id.
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(contract_groups_members_path()),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: EstimatedLevel(10, false),
                estimated_layer_sizes: AllSubtrees(DEFAULT_HASH_SIZE_U8, NoSumTrees, None),
            },
        );

        // The new contract's backwards index: up to three subtrees.
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(contract_memberships_path(&contract_id)),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: ApproximateElements(3),
                estimated_layer_sizes: AllSubtrees(1, NoSumTrees, None),
            },
        );

        let mut has_contract_members = false;
        let mut has_document_type_members = false;
        let mut has_token_members = false;

        for membership in memberships {
            let contract_group_id = membership.contract_group_id.to_buffer();

            // The group's own tree: the info item and three member subtrees.
            estimated_costs_only_with_layer_info.insert(
                KeyInfoPath::from_known_path(contract_group_path(&contract_group_id)),
                EstimatedLayerInformation {
                    tree_type: TreeType::NormalTree,
                    estimated_layer_count: ApproximateElements(4),
                    estimated_layer_sizes: Mix {
                        subtrees_size: Some((1, NoSumTrees, None, 3)),
                        items_size: Some((1, CONTRACT_GROUP_INFO_ESTIMATED_SIZE, None, 1)),
                        references_size: None,
                        items_with_sum_item_size: None,
                        references_with_sum_item_size: None,
                    },
                },
            );

            match &membership.member {
                ContractGroupMember::Contract => {
                    has_contract_members = true;
                    // Whole-contract members: empty items keyed by contract id.
                    estimated_costs_only_with_layer_info.insert(
                        KeyInfoPath::from_known_path(contract_group_contracts_path(
                            &contract_group_id,
                        )),
                        EstimatedLayerInformation {
                            tree_type: TreeType::NormalTree,
                            estimated_layer_count: EstimatedLevel(6, false),
                            estimated_layer_sizes: AllItems(DEFAULT_HASH_SIZE_U8, 0, None),
                        },
                    );
                }
                ContractGroupMember::DocumentType(_) => {
                    has_document_type_members = true;
                    // Document type members: one subtree per contract, then empty items keyed
                    // by document type name.
                    estimated_costs_only_with_layer_info.insert(
                        KeyInfoPath::from_known_path(contract_group_document_types_path(
                            &contract_group_id,
                        )),
                        EstimatedLayerInformation {
                            tree_type: TreeType::NormalTree,
                            estimated_layer_count: EstimatedLevel(6, false),
                            estimated_layer_sizes: AllSubtrees(
                                DEFAULT_HASH_SIZE_U8,
                                NoSumTrees,
                                None,
                            ),
                        },
                    );
                    estimated_costs_only_with_layer_info.insert(
                        KeyInfoPath::from_known_path(
                            contract_group_document_types_for_contract_path(
                                &contract_group_id,
                                &contract_id,
                            ),
                        ),
                        EstimatedLayerInformation {
                            tree_type: TreeType::NormalTree,
                            estimated_layer_count: EstimatedLevel(2, false),
                            estimated_layer_sizes: AllItems(
                                DOCUMENT_TYPE_NAME_ESTIMATED_KEY_SIZE,
                                0,
                                None,
                            ),
                        },
                    );
                }
                ContractGroupMember::Token(_) => {
                    has_token_members = true;
                    // Token members: one subtree per contract, then empty items keyed by the
                    // two byte token position.
                    estimated_costs_only_with_layer_info.insert(
                        KeyInfoPath::from_known_path(contract_group_tokens_path(
                            &contract_group_id,
                        )),
                        EstimatedLayerInformation {
                            tree_type: TreeType::NormalTree,
                            estimated_layer_count: EstimatedLevel(6, false),
                            estimated_layer_sizes: AllSubtrees(
                                DEFAULT_HASH_SIZE_U8,
                                NoSumTrees,
                                None,
                            ),
                        },
                    );
                    estimated_costs_only_with_layer_info.insert(
                        KeyInfoPath::from_known_path(contract_group_tokens_for_contract_path(
                            &contract_group_id,
                            &contract_id,
                        )),
                        EstimatedLayerInformation {
                            tree_type: TreeType::NormalTree,
                            estimated_layer_count: EstimatedLevel(2, false),
                            estimated_layer_sizes: AllItems(2, 0, None),
                        },
                    );
                }
            }
        }

        if has_contract_members {
            // Backwards: the groups the whole contract belongs to, references keyed by group id.
            estimated_costs_only_with_layer_info.insert(
                KeyInfoPath::from_known_path(contract_memberships_groups_path(&contract_id)),
                EstimatedLayerInformation {
                    tree_type: TreeType::NormalTree,
                    estimated_layer_count: EstimatedLevel(2, false),
                    estimated_layer_sizes: AllReference(
                        DEFAULT_HASH_SIZE_U8,
                        CONTRACT_GROUP_BACKWARDS_REFERENCE_SIZE,
                        None,
                    ),
                },
            );
        }

        if has_document_type_members {
            // Backwards: one subtree per document type name, holding references keyed by
            // group id.
            estimated_costs_only_with_layer_info.insert(
                KeyInfoPath::from_known_path(contract_memberships_document_types_path(
                    &contract_id,
                )),
                EstimatedLayerInformation {
                    tree_type: TreeType::NormalTree,
                    estimated_layer_count: EstimatedLevel(2, false),
                    estimated_layer_sizes: AllSubtrees(
                        DOCUMENT_TYPE_NAME_ESTIMATED_KEY_SIZE,
                        NoSumTrees,
                        None,
                    ),
                },
            );
            for membership in memberships {
                if let ContractGroupMember::DocumentType(name) = &membership.member {
                    estimated_costs_only_with_layer_info.insert(
                        KeyInfoPath::from_known_path(contract_memberships_document_type_path(
                            &contract_id,
                            name.as_bytes(),
                        )),
                        EstimatedLayerInformation {
                            tree_type: TreeType::NormalTree,
                            estimated_layer_count: EstimatedLevel(2, false),
                            estimated_layer_sizes: AllReference(
                                DEFAULT_HASH_SIZE_U8,
                                CONTRACT_GROUP_BACKWARDS_REFERENCE_SIZE,
                                None,
                            ),
                        },
                    );
                }
            }
        }

        if has_token_members {
            // Backwards: one subtree per token position, holding references keyed by group id.
            estimated_costs_only_with_layer_info.insert(
                KeyInfoPath::from_known_path(contract_memberships_tokens_path(&contract_id)),
                EstimatedLayerInformation {
                    tree_type: TreeType::NormalTree,
                    estimated_layer_count: EstimatedLevel(2, false),
                    estimated_layer_sizes: AllSubtrees(2, NoSumTrees, None),
                },
            );
            for membership in memberships {
                if let ContractGroupMember::Token(position) = &membership.member {
                    estimated_costs_only_with_layer_info.insert(
                        KeyInfoPath::from_known_path(contract_memberships_token_path(
                            &contract_id,
                            &position.to_be_bytes(),
                        )),
                        EstimatedLayerInformation {
                            tree_type: TreeType::NormalTree,
                            estimated_layer_count: EstimatedLevel(2, false),
                            estimated_layer_sizes: AllReference(
                                DEFAULT_HASH_SIZE_U8,
                                CONTRACT_GROUP_BACKWARDS_REFERENCE_SIZE,
                                None,
                            ),
                        },
                    );
                }
            }
        }
    }
}
