mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::DataContract;
use dpp::fee::fee_result::FeeResult;

use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Inserts a contract into the drive.
    ///
    /// # Arguments
    ///
    /// * `contract` - A reference to the `DataContract` to be inserted.
    /// * `block_info` - Information about the current block.
    /// * `apply` - A boolean indicating whether the insertion should be applied.
    /// * `transaction` - A `TransactionArg` object representing the transaction for the insertion.
    /// * `platform_version` - The version of the Platform.
    ///
    /// # Returns
    ///
    /// * `Result<FeeResult, Error>` - If successful, returns a `FeeResult` containing the
    ///   calculated fee for the insertion. If an error occurs during the operation, returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function may return an `Error` if the serialization or insertion process fails, or if
    /// the drive version does not match any of the implemented method versions.
    pub fn insert_contract(
        &self,
        contract: &DataContract,
        block_info: BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        match platform_version
            .drive
            .methods
            .contract
            .insert
            .insert_contract
        {
            0 => {
                self.insert_contract_v0(contract, block_info, apply, transaction, platform_version)
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "insert_contract".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Adds the operations for inserting a contract to the `drive_operations` vector.
    ///
    /// # Arguments
    ///
    /// * `contract_element` - The contract data encapsulated in an `Element`.
    /// * `contract` - A reference to the `DataContract` to be inserted.
    /// * `block_info` - A reference to information about the current block.
    /// * `estimated_costs_only_with_layer_info` - A mutable reference to an optional `HashMap` for estimated layer information.
    /// * `drive_operations` - A mutable reference to a `Vec` of `LowLevelDriveOperation` objects to perform.
    /// * `platform_version` - The version of Platform.
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - If successful, returns `Ok(())`. If an error occurs during the operation, returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function may return an `Error` if the addition process fails or if the drive version does not match any of the implemented method versions.
    pub(crate) fn insert_contract_add_operations(
        &self,
        contract_element: Element,
        contract: &DataContract,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .contract
            .insert
            .insert_contract
        {
            0 => self.insert_contract_add_operations_v0(
                contract_element,
                contract,
                block_info,
                estimated_costs_only_with_layer_info,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "insert_contract_add_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
