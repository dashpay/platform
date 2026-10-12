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
    ///
    /// # Parameters
    ///
    /// * `contract_group_id`: The group's id.
    /// * `transaction`: The GroveDB transaction.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(ContractGroupInfo))` with the group's information, `Ok(None)` when no such
    ///   group exists.
    /// * `Err(Error)` when the method version is unknown, the read fails, or the stored
    ///   information does not deserialize.
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
    ///
    /// # Parameters
    ///
    /// * `contract_group_id`: The group's id.
    /// * `epoch`: The epoch the read is priced in.
    /// * `transaction`: The GroveDB transaction.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok((FeeResult, Option<ContractGroupInfo>))`: the fee of the read and the group's
    ///   information, `None` when no such group exists.
    /// * `Err(Error)` when the method version is unknown, the read fails, the stored
    ///   information does not deserialize, or the fee cannot be calculated.
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
    ///
    /// # Parameters
    ///
    /// * `contract_group_id`: The group's id.
    /// * `transaction`: The GroveDB transaction.
    /// * `drive_operations`: The operations accumulator the read is appended to, for billing.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(ContractGroupInfo))` with the group's information, `Ok(None)` when no such
    ///   group exists.
    /// * `Err(Error)` when the method version is unknown, the read fails, or the stored
    ///   information does not deserialize.
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
