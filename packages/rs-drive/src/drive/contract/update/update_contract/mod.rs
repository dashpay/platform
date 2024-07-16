mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::DataContract;
use dpp::fee::fee_result::FeeResult;

use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Updates a data contract.
    ///
    /// This function updates a given data contract in the storage. The version of
    /// the contract update method is determined by the drive version.
    ///
    /// # Arguments
    ///
    /// * `contract` - A reference to the `DataContract` to be updated.
    /// * `block_info` - A `BlockInfo` object containing information about the block where
    ///   the contract is being updated.
    /// * `apply` - A boolean indicating whether the contract update should be applied (`true`) or not (`false`).
    /// * `transaction` - A `TransactionArg` object representing the transaction to be used
    ///   for updating the contract.
    /// * `drive_version` - The `DriveVersion` to determine which version of contract update to use.
    ///
    /// # Returns
    ///
    /// * `Result<FeeResult, Error>` - If successful, returns a `FeeResult` representing the fee
    ///   for updating the contract. If an error occurs during the contract update or fee calculation,
    ///   returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function returns an error if the contract update or fee calculation fails, or if
    /// the provided drive version does not match any known versions.
    pub fn update_contract(
        &self,
        contract: &DataContract,
        block_info: BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
        previous_fee_versions: Option<&CachedEpochIndexFeeVersions>,
    ) -> Result<FeeResult, Error> {
        match platform_version
            .drive
            .methods
            .contract
            .update
            .update_contract
        {
            0 => self.update_contract_v0(
                contract,
                block_info,
                apply,
                transaction,
                platform_version,
                previous_fee_versions,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "update_contract".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Updates a contract element.
    ///
    /// This function updates a given element in a contract. The version of
    /// the contract element update method is determined by the drive version.
    ///
    /// # Arguments
    ///
    /// * `contract_element` - The `Element` to be updated in the contract.
    /// * `contract` - A reference to the `Contract` containing the element to be updated.
    /// * `original_contract` - A reference to the original `Contract` before updates.
    /// * `block_info` - A `BlockInfo` object containing information about the block where
    ///   the contract is being updated.
    /// * `transaction` - A `TransactionArg` object representing the transaction to be used
    ///   for updating the contract.
    /// * `drive_operations` - A mutable reference to a vector of `LowLevelDriveOperation`.
    ///   This vector will be filled with operations needed to update the contract element.
    /// * `drive_version` - The `DriveVersion` to determine which version of contract element update to use.
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - If successful, returns `Ok(())`. If an error occurs during the contract element update,
    ///   returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function returns an error if the contract element update fails, or if
    /// the provided drive version does not match any known versions.
    pub fn update_contract_element(
        &self,
        contract_element: Element,
        contract: &DataContract,
        original_contract: &DataContract,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .contract
            .update
            .update_contract
        {
            0 => self.update_contract_element_v0(
                contract_element,
                contract,
                original_contract,
                block_info,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "update_contract_element".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Updates add operations for a contract.
    ///
    /// This function updates the add operations for a given contract. The version of
    /// the contract add operations update method is determined by the drive version.
    ///
    /// # Arguments
    ///
    /// * `contract_element` - The `Element` to be added to the contract.
    /// * `contract` - A reference to the `Contract` containing the operations to be updated.
    /// * `original_contract` - A reference to the original `Contract` before updates.
    /// * `block_info` - A `BlockInfo` object containing information about the block where
    ///   the contract is being updated.
    /// * `estimated_costs_only_with_layer_info` - A mutable reference to an `Option` of a `HashMap`
    ///   containing estimated layer information. If provided (`Some`), only the estimated costs will be updated.
    /// * `transaction` - A `TransactionArg` object representing the transaction to be used
    ///   for updating the contract.
    /// * `drive_operations` - A mutable reference to a vector of `LowLevelDriveOperation`.
    ///   This vector will be filled with operations needed to update the contract.
    /// * `drive_version` - The `DriveVersion` to determine which version of contract add operations update to use.
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - If successful, returns `Ok(())`. If an error occurs during the contract add operations update,
    ///   returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function returns an error if the contract add operations update fails, or if
    /// the provided drive version does not match any known versions.
    pub(crate) fn update_contract_add_operations(
        &self,
        contract_element: Element,
        contract: &DataContract,
        original_contract: &DataContract,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .contract
            .update
            .update_contract
        {
            0 => self.update_contract_add_operations_v0(
                contract_element,
                contract,
                original_contract,
                block_info,
                estimated_costs_only_with_layer_info,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "update_contract_add_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
