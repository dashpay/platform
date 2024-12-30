use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::data_contract::group::GroupSumPower;
use dpp::data_contract::GroupContractPosition;
use dpp::identifier::Identifier;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use platform_version::version::PlatformVersion;
use std::collections::HashMap;

mod v0;

impl Drive {
    /// Fetches the action id signers power
    pub fn fetch_action_id_signers_power(
        &self,
        contract_id: Identifier,
        group_contract_position: GroupContractPosition,
        action_id: Identifier,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<GroupSumPower, Error> {
        match platform_version
            .drive
            .methods
            .group
            .fetch
            .fetch_action_id_signers_power
        {
            0 => self.fetch_action_id_signers_power_v0(
                contract_id,
                group_contract_position,
                action_id,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_action_id_signers_power".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    pub(crate) fn fetch_action_id_signers_power_and_add_operations(
        &self,
        contract_id: Identifier,
        group_contract_position: GroupContractPosition,
        action_id: Identifier,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<GroupSumPower, Error> {
        match platform_version
            .drive
            .methods
            .group
            .fetch
            .fetch_action_id_signers_power
        {
            0 => self.fetch_action_id_signers_power_and_add_operations_v0(
                contract_id,
                group_contract_position,
                action_id,
                estimated_costs_only_with_layer_info,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_action_id_signers_and_add_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
