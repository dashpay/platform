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
    ///
    /// # Parameters
    ///
    /// * `contract_id`: The contract's id.
    /// * `transaction`: The GroveDB transaction.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok(ContractGroupMembershipsForContract)` with the groups of the whole contract, of
    ///   each document type and of each token; empty when it belongs to no group.
    /// * `Err(Error)` when the method version is unknown, a read fails, or a stored membership
    ///   is malformed.
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
    ///
    /// # Parameters
    ///
    /// * `contract_id`: The contract's id.
    /// * `epoch`: The epoch the reads are priced in.
    /// * `transaction`: The GroveDB transaction.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok((FeeResult, ContractGroupMembershipsForContract))`: the fee of the reads and the
    ///   contract's memberships, empty when it belongs to no group.
    /// * `Err(Error)` when the method version is unknown, a read fails, a stored membership is
    ///   malformed, or the fee cannot be calculated.
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
    ///
    /// # Parameters
    ///
    /// * `contract_id`: The contract's id.
    /// * `transaction`: The GroveDB transaction.
    /// * `drive_operations`: The operations accumulator the reads are appended to, for billing.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok(ContractGroupMembershipsForContract)` with the groups of the whole contract, of
    ///   each document type and of each token; empty when it belongs to no group.
    /// * `Err(Error)` when the method version is unknown, a read fails, or a stored membership
    ///   is malformed.
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
