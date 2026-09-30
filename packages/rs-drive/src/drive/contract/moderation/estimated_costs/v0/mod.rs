use crate::drive::constants::{
    ESTIMATED_AVERAGE_DOCUMENT_TYPE_NAME_SIZE, ESTIMATED_DOCUMENT_TYPES_DELETABLE_BY_MODERATORS,
};
use crate::drive::contract::moderation::types::{
    estimated_entry_value_size, ContractDocumentRecords, CONTRACT_MODERATION_ACTION_COUNT_SIZE,
};
use crate::drive::contract::paths::{
    contract_document_records_path, contract_document_type_records_path,
    contract_moderation_action_counts_path, contract_moderation_list_path,
};
use crate::drive::Drive;
use crate::error::Error;
use crate::util::storage_flags::StorageFlags;
use crate::util::type_constants::DEFAULT_HASH_SIZE_U8;
use dpp::data_contract::config::moderation::ContractModerationList;
use dpp::version::drive_versions::DriveVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::{ApproximateElements, EstimatedLevel, PotentiallyAtMaxElements};
use grovedb::EstimatedLayerSizes::{AllItems, AllSubtrees};
use grovedb::EstimatedSumTrees::NoSumTrees;
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

    pub(super) fn add_estimation_costs_for_contract_document_record_trees_v0(
        contract_id: [u8; 32],
        records: ContractDocumentRecords,
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        Self::add_estimation_costs_for_contract_moderation_trees_v0(
            contract_id,
            estimated_costs_only_with_layer_info,
            drive_version,
        )?;

        // The tree of all the records of the kind (`[64, id, 2, 16]` for removal records,
        // `[64, id, 2, 24]` for settled-deletion approvals): one subtree per document type that
        // keeps them, keyed by the type's name. A contract has few: the types moderators may
        // delete documents of, and fewer that set `deleteSettled`.
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(contract_document_records_path(&contract_id, records)),
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

    pub(super) fn add_estimation_costs_for_contract_document_record_v0(
        contract_id: [u8; 32],
        records: ContractDocumentRecords,
        document_type_name: &str,
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        Self::add_estimation_costs_for_contract_document_record_trees_v0(
            contract_id,
            records,
            estimated_costs_only_with_layer_info,
            drive_version,
        )?;

        // The records of one document type: one item per document, keyed by document id. The
        // records a write walks past are sized like a typical one; the record being written is
        // priced by its own size.
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(contract_document_type_records_path(
                &contract_id,
                records,
                document_type_name,
            )),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: PotentiallyAtMaxElements,
                estimated_layer_sizes: AllItems(
                    DEFAULT_HASH_SIZE_U8,
                    records.estimated_value_size(),
                    Some(StorageFlags::approximate_size(true, None)),
                ),
            },
        );

        Ok(())
    }
}
