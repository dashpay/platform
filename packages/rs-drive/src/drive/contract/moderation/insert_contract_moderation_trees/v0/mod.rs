use crate::drive::contract::paths::{contract_moderation_list_key, contract_other_path};
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::object_size_info::DriveKeyInfo;
use crate::util::storage_flags::StorageFlags;
use dpp::data_contract::config::moderation::ContractModerationConfig;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn insert_contract_moderation_trees_operations_v0(
        &self,
        contract_id: [u8; 32],
        moderation: &ContractModerationConfig,
        storage_flags: Option<&StorageFlags>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        _transaction: TransactionArg,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Drive::add_estimation_costs_for_contract_moderation_trees(
                contract_id,
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
        }

        let contract_other_path = contract_other_path(&contract_id);

        for list in moderation.lists() {
            // Unconditional, like the contract's other tree and its documents tree: a contract
            // insertion (re)creates the contract's root subtree in the same batch, so whatever
            // state holds under it is gone, and a check against that state would skip a tree
            // the batch has just wiped.
            self.batch_insert_empty_tree(
                contract_other_path,
                DriveKeyInfo::KeyRef(contract_moderation_list_key(list)),
                storage_flags,
                batch_operations,
                &platform_version.drive,
            )?;
        }

        Ok(())
    }
}
