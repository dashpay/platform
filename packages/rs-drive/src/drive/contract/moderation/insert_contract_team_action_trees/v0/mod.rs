use crate::drive::contract::paths::{
    contract_other_path, contract_team_actions_path, CONTRACT_TEAM_ACTIONS_KEY,
    CONTRACT_TEAM_ACTIVE_ACTIONS_KEY, CONTRACT_TEAM_CLOSED_ACTIONS_KEY,
};
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::object_size_info::DriveKeyInfo;
use crate::util::storage_flags::StorageFlags;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Unconditional, like the list trees: only a contract's insertion creates them, and it
    /// (re)creates the contract's root subtree in the same batch.
    #[inline(always)]
    pub(super) fn insert_contract_team_action_trees_operations_v0(
        &self,
        contract_id: [u8; 32],
        storage_flags: Option<&StorageFlags>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        _transaction: TransactionArg,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Drive::add_estimation_costs_for_contract_team_action_trees(
                contract_id,
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
        }

        self.batch_insert_empty_tree(
            contract_other_path(&contract_id),
            DriveKeyInfo::KeyRef(&[CONTRACT_TEAM_ACTIONS_KEY]),
            storage_flags,
            batch_operations,
            &platform_version.drive,
        )?;
        for status_key in [
            CONTRACT_TEAM_ACTIVE_ACTIONS_KEY,
            CONTRACT_TEAM_CLOSED_ACTIONS_KEY,
        ] {
            self.batch_insert_empty_tree(
                contract_team_actions_path(&contract_id),
                DriveKeyInfo::KeyRef(status_key),
                storage_flags,
                batch_operations,
                &platform_version.drive,
            )?;
        }
        Ok(())
    }
}
