mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::contract_group::ContractGroupInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Registers a contract group: its tree under `Groups`, the info item and the three empty
    /// member subtrees. Applies the operations when `apply` is true, otherwise only estimates.
    ///
    /// The caller must have checked that no group with this id exists.
    ///
    /// # Parameters
    ///
    /// * `contract_group_id`: The new group's id.
    /// * `info`: The group's information, stored as its info item.
    /// * `block_info`: The block being executed; its epoch prices the fee.
    /// * `apply`: Whether to apply the operations or only estimate their cost.
    /// * `transaction`: The GroveDB transaction.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok(FeeResult)` with the fee of the operations, applied or estimated.
    /// * `Err(Error)` when the method version is unknown, the group already exists, the info
    ///   does not serialize, applying the batch fails, or the fee cannot be calculated.
    pub fn insert_contract_group(
        &self,
        contract_group_id: Identifier,
        info: &ContractGroupInfo,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        match platform_version
            .drive
            .methods
            .contract_group
            .insert
            .insert_contract_group
        {
            0 => self.insert_contract_group_v0(
                contract_group_id,
                info,
                block_info,
                apply,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "insert_contract_group".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// The low level operations registering a contract group. With layer information the
    /// operations are built for estimation only.
    ///
    /// # Parameters
    ///
    /// * `contract_group_id`: The new group's id.
    /// * `info`: The group's information, stored as its info item.
    /// * `estimated_costs_only_with_layer_info`: The estimation map, when only estimating costs.
    /// * `transaction`: The GroveDB transaction.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<LowLevelDriveOperation>)` with the inserts of the group's tree, its info item
    ///   and its three member subtrees.
    /// * `Err(Error)` when the method version is unknown, the group already exists, the info
    ///   does not serialize, or building an operation fails.
    pub fn insert_contract_group_operations(
        &self,
        contract_group_id: Identifier,
        info: &ContractGroupInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version
            .drive
            .methods
            .contract_group
            .insert
            .insert_contract_group
        {
            0 => self.insert_contract_group_operations_v0(
                contract_group_id,
                info,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "insert_contract_group_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
