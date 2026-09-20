use crate::drive::contract::moderation::types::estimated_entry_value_size;
use crate::drive::contract::paths::contract_moderation_list_path;
use crate::drive::Drive;
use crate::error::Error;
use crate::util::storage_flags::StorageFlags;
use crate::util::type_constants::DEFAULT_HASH_SIZE_U8;
use dpp::data_contract::config::moderation::ContractModerationList;
use dpp::version::drive_versions::DriveVersion;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::PotentiallyAtMaxElements;
use grovedb::EstimatedLayerSizes::AllItems;
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

        // The contract's other tree (`[64, id, 2]`): the version item and up to two list trees.
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
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        Self::add_estimation_costs_for_contract_moderation_trees_v0(
            contract_id,
            estimated_costs_only_with_layer_info,
            &platform_version.drive,
        )?;

        // The list itself: one item per barred identity, keyed by identity id. An entry holds a
        // reason of any length up to the limit, so every entry is estimated at the largest.
        let value_size = estimated_entry_value_size(list, platform_version);

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
