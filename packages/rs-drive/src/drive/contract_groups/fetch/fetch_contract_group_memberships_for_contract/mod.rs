mod v0;

use crate::drive::contract_groups::types::ContractGroupMembershipsForContract;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::epoch::Epoch;
use dpp::fee::fee_result::FeeResult;
use dpp::identifier::Identifier;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Fetches the contract groups a contract belongs to, as a whole, through its document
    /// types and through its tokens. Empty when the contract belongs to no group.
    pub fn fetch_contract_group_memberships_for_contract(
        &self,
        contract_id: Identifier,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ContractGroupMembershipsForContract, Error> {
        match platform_version
            .drive
            .methods
            .contract_group
            .fetch
            .fetch_contract_group_memberships_for_contract
        {
            0 => self.fetch_contract_group_memberships_for_contract_v0(
                contract_id,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_contract_group_memberships_for_contract".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Fetches the contract groups a contract belongs to and the fee of the lookup, so that
    /// consensus validation can bill it.
    pub fn fetch_contract_group_memberships_for_contract_with_fee(
        &self,
        contract_id: Identifier,
        epoch: &Epoch,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(FeeResult, ContractGroupMembershipsForContract), Error> {
        match platform_version
            .drive
            .methods
            .contract_group
            .fetch
            .fetch_contract_group_memberships_for_contract
        {
            0 => self.fetch_contract_group_memberships_for_contract_with_fee_v0(
                contract_id,
                epoch,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_contract_group_memberships_for_contract_with_fee".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Fetches the contract groups a contract belongs to, recording the reads in
    /// `drive_operations`.
    pub fn fetch_contract_group_memberships_for_contract_add_to_operations(
        &self,
        contract_id: Identifier,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<ContractGroupMembershipsForContract, Error> {
        match platform_version
            .drive
            .methods
            .contract_group
            .fetch
            .fetch_contract_group_memberships_for_contract
        {
            0 => self.fetch_contract_group_memberships_for_contract_add_to_operations_v0(
                contract_id,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_contract_group_memberships_for_contract_add_to_operations"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
