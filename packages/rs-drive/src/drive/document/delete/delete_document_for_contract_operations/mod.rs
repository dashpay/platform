mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;

use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::DataContract;

use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Prepares the operations for deleting a document.
    ///
    /// # Parameters
    /// * `document_id`: The ID of the document to delete.
    /// * `contract`: The contract that contains the document.
    /// * `document_type`: The type of the document.
    /// * `previous_batch_operations`: Previous batch operations to include.
    /// * `estimated_costs_only_with_layer_info`: Estimated costs with layer info.
    /// * `transaction`: The transaction argument.
    /// * `drive_version`: The drive version to select the correct function version to run.
    ///
    /// # Returns
    /// * `Ok(Vec<LowLevelDriveOperation>)` if the operation was successful.
    /// * `Err(DriveError::UnknownVersionMismatch)` if the drive version does not match known versions.
    pub(crate) fn delete_document_for_contract_operations(
        &self,
        document_id: [u8; 32],
        contract: &DataContract,
        document_type: DocumentTypeRef,
        previous_batch_operations: Option<&mut Vec<LowLevelDriveOperation>>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version
            .drive
            .methods
            .document
            .delete
            .delete_document_for_contract_operations
        {
            0 => self.delete_document_for_contract_operations_v0(
                document_id,
                contract,
                document_type,
                previous_batch_operations,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "delete_document_for_contract_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
