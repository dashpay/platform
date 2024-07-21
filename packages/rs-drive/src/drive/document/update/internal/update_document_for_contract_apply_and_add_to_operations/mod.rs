mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::object_size_info::DocumentAndContractInfo;
use dpp::block::block_info::BlockInfo;

use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Updates a document.
    ///
    /// # Parameters
    /// * `document_and_contract_info`: The document and contract info to be updated.
    /// * `block_info`: The block info.
    /// * `estimated_costs_only_with_layer_info`: An optional hashmap of key info paths to estimated layer information.
    /// * `transaction`: The transaction argument.
    /// * `drive_operations`: The mutable reference to the vector of drive operations to be added.
    /// * `drive_version`: The drive version to select the correct function version to run.
    ///
    /// # Returns
    /// * `Ok(())` if the operation was successful.
    /// * `Err(DriveError::UnknownVersionMismatch)` if the drive version does not match known versions.
    pub fn update_document_for_contract_apply_and_add_to_operations(
        &self,
        document_and_contract_info: DocumentAndContractInfo,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .document
            .update
            .update_document_for_contract_apply_and_add_to_operations
        {
            0 => self.update_document_for_contract_apply_and_add_to_operations_v0(
                document_and_contract_info,
                block_info,
                estimated_costs_only_with_layer_info,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "update_document_for_contract_apply_and_add_to_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
