use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::storage_flags::StorageFlags;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::DataContract;
use dpp::fee::fee_result::FeeResult;

use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::borrow::Cow;
use std::collections::HashMap;

mod v0;

/// Drive contract application methods.
impl Drive {
    /// Applies a contract with its serialization.
    ///
    /// This method delegates the contract application to the appropriate versioned method
    /// according to the drive version.
    ///
    /// # Arguments
    ///
    /// * `contract` - A reference to the `DataContract` to be applied.
    /// * `contract_serialization` - The serialized data of the contract.
    /// * `block_info` - A `BlockInfo` object containing information about the block where
    ///   the contract is being applied.
    /// * `apply` - A boolean indicating whether the contract should be applied (`true`) or not (`false`).
    /// * `storage_flags` - An optional `Cow<StorageFlags>` containing the storage flags for the contract.
    /// * `transaction` - A `TransactionArg` object representing the transaction to be used
    ///   for applying the contract.
    /// * `drive_version` - The `DriveVersion` to determine which version of contract application to use.
    ///
    /// # Returns
    ///
    /// * `Result<FeeResult, Error>` - If successful, returns a `FeeResult` representing the fee
    ///   for applying the contract. If an error occurs during the contract application or fee calculation,
    ///   returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function returns an error if the contract application or fee calculation fails, or if
    /// the provided drive version does not match any known versions.
    pub fn apply_contract_with_serialization(
        &self,
        contract: &DataContract,
        contract_serialization: Vec<u8>,
        block_info: BlockInfo,
        apply: bool,
        storage_flags: Option<Cow<StorageFlags>>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        match platform_version
            .drive
            .methods
            .contract
            .apply
            .apply_contract_with_serialization
        {
            0 => self.apply_contract_with_serialization_v0(
                contract,
                contract_serialization,
                block_info,
                apply,
                storage_flags,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "apply_contract_with_serialization".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Gets the operations for applying a contract with its serialization.
    ///
    /// This method delegates the operation gathering for applying a contract to the appropriate versioned method
    /// according to the drive version.
    ///
    /// # Arguments
    ///
    /// * `contract` - A reference to the `DataContract` to be applied.
    /// * `contract_serialization` - The serialized data of the contract.
    /// * `block_info` - A `BlockInfo` object containing information about the block where
    ///   the contract is being applied.
    /// * `estimated_costs_only_with_layer_info` - A mutable reference to an `Option` of a `HashMap`
    ///   containing estimated layer information. If provided (`Some`), only the estimated costs will be updated.
    /// * `storage_flags` - An optional `Cow<StorageFlags>` containing the storage flags for the contract.
    /// * `transaction` - A `TransactionArg` object representing the transaction to be used
    ///   for applying the contract.
    /// * `drive_version` - The `DriveVersion` to determine which version of contract operations application to use.
    ///
    /// # Returns
    ///
    /// * `Result<Vec<LowLevelDriveOperation>, Error>` - If successful, returns a vector of `LowLevelDriveOperation`
    ///   containing the operations for applying the contract. If an error occurs during the operation gathering,
    ///   returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function returns an error if the operation gathering fails, or if the provided drive version does not
    /// match any known versions.
    pub(crate) fn apply_contract_with_serialization_operations(
        &self,
        contract: &DataContract,
        contract_serialization: Vec<u8>,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        storage_flags: Option<Cow<StorageFlags>>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version
            .drive
            .methods
            .contract
            .apply
            .apply_contract_with_serialization
        {
            0 => self.apply_contract_with_serialization_operations_v0(
                contract,
                contract_serialization,
                block_info,
                estimated_costs_only_with_layer_info,
                storage_flags,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "apply_contract_with_serialization_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
