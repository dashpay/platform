use crate::drive::contract::moderation::CONTRACT_SUSPENSION_VALUE_SIZE;
use crate::drive::contract::paths::{contract_moderation_list_path, contract_root_path};
use crate::drive::Drive;
use crate::error::Error;
use crate::util::storage_flags::StorageFlags;
use crate::util::type_constants::DEFAULT_HASH_SIZE_U8;
use dpp::data_contract::config::moderation::ContractModerationList;
use dpp::version::drive_versions::DriveVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::{EstimatedLevel, PotentiallyAtMaxElements};
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

        // The contract's own subtree: the contract, the documents, the version item and up to
        // two list trees.
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(contract_root_path(&contract_id)),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: EstimatedLevel(2, false),
                estimated_layer_sizes: AllSubtrees(
                    1,
                    NoSumTrees,
                    Some(StorageFlags::approximate_size(true, None)),
                ),
            },
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

        let value_size = match list {
            ContractModerationList::Banlist => 0,
            ContractModerationList::Suspensions => CONTRACT_SUSPENSION_VALUE_SIZE,
        };

        // The list itself: one small item per barred identity, keyed by identity id.
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
}
