mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::data_contract::document_type::DocumentTypeRef;

use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Removes the document from primary storage.
    ///
    /// # Parameters
    /// * `document_id`: The ID of the document to delete.
    /// * `document_type`: The document type to delete.
    /// * `contract_documents_primary_key_path`: The primary key path of the document.
    /// * `estimated_costs_only_with_layer_info`: Estimated costs with layer info.
    /// * `transaction`: The transaction argument.
    /// * `batch_operations`: Batch operations to perform.
    /// * `drive_version`: The drive version to select the correct function version to run.
    ///
    /// # Returns
    /// * `Ok(())` if the operation was successful.
    /// * `Err(DriveError::UnknownVersionMismatch)` if the drive version does not match known versions.
    pub(in crate::drive::document) fn remove_document_from_primary_storage(
        &self,
        document_id: Identifier,
        document_type: DocumentTypeRef,
        contract_documents_primary_key_path: [&[u8]; 5],
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .document
            .delete
            .remove_document_from_primary_storage
        {
            0 => self.remove_document_from_primary_storage_v0(
                document_id,
                document_type,
                contract_documents_primary_key_path,
                estimated_costs_only_with_layer_info,
                transaction,
                batch_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "remove_document_from_primary_storage".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
