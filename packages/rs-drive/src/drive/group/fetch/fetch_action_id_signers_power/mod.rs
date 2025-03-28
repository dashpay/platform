use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::data_contract::group::GroupSumPower;
use dpp::data_contract::GroupContractPosition;
use dpp::identifier::Identifier;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

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
    ) -> Result<Option<GroupSumPower>, Error> {
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

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn fetch_action_id_signers_power_and_add_operations(
        &self,
        contract_id: Identifier,
        group_contract_position: GroupContractPosition,
        action_id: Identifier,
        estimate_costs_only: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<GroupSumPower>, Error> {
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
                estimate_costs_only,
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
