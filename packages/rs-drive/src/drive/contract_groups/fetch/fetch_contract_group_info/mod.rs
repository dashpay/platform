mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::epoch::Epoch;
use dpp::contract_group::ContractGroupInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::identifier::Identifier;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Fetches the stored information of a contract group, or `None` when no such group exists.
    pub fn fetch_contract_group_info(
        &self,
        contract_group_id: Identifier,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<ContractGroupInfo>, Error> {
        match platform_version
            .drive
            .methods
            .contract_group
            .fetch
            .fetch_contract_group_info
        {
            0 => {
                self.fetch_contract_group_info_v0(contract_group_id, transaction, platform_version)
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_contract_group_info".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Fetches the stored information of a contract group and the fee of the lookup, so that
    /// consensus validation can bill it.
    pub fn fetch_contract_group_info_with_fee(
        &self,
        contract_group_id: Identifier,
        epoch: &Epoch,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(FeeResult, Option<ContractGroupInfo>), Error> {
        match platform_version
            .drive
            .methods
            .contract_group
            .fetch
            .fetch_contract_group_info
        {
            0 => self.fetch_contract_group_info_with_fee_v0(
                contract_group_id,
                epoch,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_contract_group_info_with_fee".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Fetches the stored information of a contract group, recording the read in
    /// `drive_operations`.
    pub fn fetch_contract_group_info_add_to_operations(
        &self,
        contract_group_id: Identifier,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<ContractGroupInfo>, Error> {
        match platform_version
            .drive
            .methods
            .contract_group
            .fetch
            .fetch_contract_group_info
        {
            0 => self.fetch_contract_group_info_add_to_operations_v0(
                contract_group_id,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_contract_group_info_add_to_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
