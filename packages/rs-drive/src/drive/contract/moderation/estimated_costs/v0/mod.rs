use crate::drive::constants::{
    ESTIMATED_AVERAGE_DOCUMENT_TYPE_NAME_SIZE, ESTIMATED_DOCUMENT_TYPES_DELETABLE_BY_MODERATORS,
};
use crate::drive::contract::moderation::types::CONTRACT_MODERATION_ACTION_COUNT_SIZE;
use crate::drive::contract::moderation::types::{
    estimated_contract_team_action_value_size, estimated_document_removal_value_size,
    estimated_entry_value_size,
};
use crate::drive::contract::paths::{
    contract_document_removals_path, contract_document_type_removals_path,
    contract_moderation_action_counts_path, contract_moderation_list_path,
    contract_team_action_path, contract_team_action_signers_path, contract_team_action_status_path,
    contract_team_actions_path,
};
use crate::drive::Drive;
use crate::error::Error;
use crate::util::storage_flags::StorageFlags;
use crate::util::type_constants::DEFAULT_HASH_SIZE_U8;
use dpp::data_contract::config::moderation::ContractModerationList;
use dpp::group::group_action_status::GroupActionStatus;
use dpp::version::drive_versions::DriveVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::{ApproximateElements, EstimatedLevel, PotentiallyAtMaxElements};
use grovedb::EstimatedLayerSizes::{AllItems, AllSubtrees, Mix};
use grovedb::EstimatedSumTrees::{AllSumTrees, NoSumTrees};
use grovedb::{EstimatedLayerInformation, TreeType};
use std::collections::HashMap;

impl Drive {
    pub(super) fn add_estimation_costs_for_contract_moderation_trees_v0(
        contract_id: [u8; 32],
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        Self::add_estimation_costs_for_levels_up_to_contract(
            estimated_costs_only_with_layer_info,
            drive_version,
        )?;

        // The contract's other tree (`[64, id, 2]`): the version item and up to three list trees.
        Drive::add_estimation_costs_for_contract_other_tree(
            contract_id,
            estimated_costs_only_with_layer_info,
        );

        Ok(())
    }

    pub(super) fn add_estimation_costs_for_contract_moderation_entry_v0(
        contract_id: [u8; 32],
        list: ContractModerationList,
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        Self::add_estimation_costs_for_contract_moderation_trees_v0(
            contract_id,
            estimated_costs_only_with_layer_info,
            drive_version,
        )?;

        // The list itself: one item per barred identity, keyed by identity id. An entry holds a
        // reason of any length up to the limit, estimated at a typical one.
        let value_size = estimated_entry_value_size(list);

        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(contract_moderation_list_path(&contract_id, list)),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: PotentiallyAtMaxElements,
                estimated_layer_sizes: AllItems(
                    DEFAULT_HASH_SIZE_U8,
                    value_size,
                    Some(StorageFlags::approximate_size(true, None)),
                ),
            },
        );

        Ok(())
    }

    pub(super) fn add_estimation_costs_for_contract_moderation_action_counts_v0(
        contract_id: [u8; 32],
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        Self::add_estimation_costs_for_contract_moderation_trees_v0(
            contract_id,
            estimated_costs_only_with_layer_info,
            drive_version,
        )?;

        // The counts (`[64, id, 2, 48]`): one item per member of the seated team who acted
        // since the last settle, keyed by identity id, a four-byte count without storage flags.
        // A team is at most the leader, the elected members and the additions, so the tree is
        // at most a few levels deep.
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(contract_moderation_action_counts_path(&contract_id)),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: EstimatedLevel(4, false),
                estimated_layer_sizes: AllItems(
                    DEFAULT_HASH_SIZE_U8,
                    CONTRACT_MODERATION_ACTION_COUNT_SIZE as u32,
                    None,
                ),
            },
        );

        Ok(())
    }

    pub(super) fn add_estimation_costs_for_contract_document_removal_trees_v0(
        contract_id: [u8; 32],
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        Self::add_estimation_costs_for_contract_moderation_trees_v0(
            contract_id,
            estimated_costs_only_with_layer_info,
            drive_version,
        )?;

        // The tree of all the records (`[64, id, 2, 16]`): one subtree per document type
        // moderators may delete documents of, keyed by the type's name. A contract has few.
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(contract_document_removals_path(&contract_id)),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: ApproximateElements(
                    ESTIMATED_DOCUMENT_TYPES_DELETABLE_BY_MODERATORS,
                ),
                estimated_layer_sizes: AllSubtrees(
                    ESTIMATED_AVERAGE_DOCUMENT_TYPE_NAME_SIZE,
                    NoSumTrees,
                    Some(StorageFlags::approximate_size(true, None)),
                ),
            },
        );

        Ok(())
    }

    pub(super) fn add_estimation_costs_for_contract_document_removal_v0(
        contract_id: [u8; 32],
        document_type_name: &str,
        estimated_kept_fields_size: u32,
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        Self::add_estimation_costs_for_contract_document_removal_trees_v0(
            contract_id,
            estimated_costs_only_with_layer_info,
            drive_version,
        )?;

        // The records of one document type: one item per removed document, keyed by document
        // id. The records a write walks past are sized like a typical one, keeping what the
        // type's records are estimated to keep (the same paths, at their middle sizes); the
        // record being written is priced by its own size.
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(contract_document_type_removals_path(
                &contract_id,
                document_type_name,
            )),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: PotentiallyAtMaxElements,
                estimated_layer_sizes: AllItems(
                    DEFAULT_HASH_SIZE_U8,
                    estimated_document_removal_value_size(estimated_kept_fields_size),
                    Some(StorageFlags::approximate_size(true, None)),
                ),
            },
        );

        Ok(())
    }

    pub(super) fn add_estimation_costs_for_contract_team_action_trees_v0(
        contract_id: [u8; 32],
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        Self::add_estimation_costs_for_contract_moderation_trees_v0(
            contract_id,
            estimated_costs_only_with_layer_info,
            drive_version,
        )?;

        // The team actions tree (`[64, id, 2, 24]`): the active and the closed actions.
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(contract_team_actions_path(&contract_id)),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: EstimatedLevel(1, false),
                estimated_layer_sizes: AllSubtrees(
                    1,
                    NoSumTrees,
                    Some(StorageFlags::approximate_size(true, None)),
                ),
            },
        );

        Ok(())
    }

    pub(super) fn add_estimation_costs_for_contract_team_action_v0(
        contract_id: [u8; 32],
        action_id: [u8; 32],
        closes: bool,
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        Self::add_estimation_costs_for_contract_team_action_trees_v0(
            contract_id,
            estimated_costs_only_with_layer_info,
            drive_version,
        )?;

        let statuses: &[(GroupActionStatus, Option<u32>)] = if closes {
            &[
                (
                    GroupActionStatus::ActionActive,
                    Some(StorageFlags::approximate_size(true, None)),
                ),
                (GroupActionStatus::ActionClosed, None),
            ]
        } else {
            &[(
                GroupActionStatus::ActionActive,
                Some(StorageFlags::approximate_size(true, None)),
            )]
        };
        for (status, flags_size) in statuses {
            // The actions of one status, keyed by action id: as many as the team ever
            // proposed, estimated as a token group's are.
            estimated_costs_only_with_layer_info.insert(
                KeyInfoPath::from_known_path(contract_team_action_status_path(
                    &contract_id,
                    *status,
                )),
                EstimatedLayerInformation {
                    tree_type: TreeType::NormalTree,
                    estimated_layer_count: EstimatedLevel(10, false),
                    estimated_layer_sizes: AllSubtrees(DEFAULT_HASH_SIZE_U8, NoSumTrees, None),
                },
            );
            // One action: its info and the sum tree of its approvals.
            estimated_costs_only_with_layer_info.insert(
                KeyInfoPath::from_known_path(contract_team_action_path(
                    &contract_id,
                    *status,
                    &action_id,
                )),
                EstimatedLayerInformation {
                    tree_type: TreeType::NormalTree,
                    estimated_layer_count: EstimatedLevel(1, false),
                    estimated_layer_sizes: Mix {
                        subtrees_size: Some((1, AllSumTrees, None, 1)),
                        items_size: Some((
                            1,
                            estimated_contract_team_action_value_size(),
                            *flags_size,
                            1,
                        )),
                        references_size: None,
                        items_with_sum_item_size: None,
                        references_with_sum_item_size: None,
                    },
                },
            );
            // Its approvals: one sum item per member who approved, at most the team, a few
            // levels deep.
            estimated_costs_only_with_layer_info.insert(
                KeyInfoPath::from_known_path(contract_team_action_signers_path(
                    &contract_id,
                    *status,
                    &action_id,
                )),
                EstimatedLayerInformation {
                    tree_type: TreeType::SumTree,
                    estimated_layer_count: EstimatedLevel(4, false),
                    estimated_layer_sizes: AllItems(DEFAULT_HASH_SIZE_U8, 8, *flags_size),
                },
            );
        }

        Ok(())
    }
}
