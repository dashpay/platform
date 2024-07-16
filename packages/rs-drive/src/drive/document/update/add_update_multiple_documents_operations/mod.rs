mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::util::batch::DriveOperation;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::DataContract;
use dpp::document::Document;
use dpp::version::drive_versions::DriveVersion;

impl Drive {
    /// Adds multiple document update operations to the drive.
    ///
    ///
    /// # Arguments
    ///
    /// * `documents`: A slice of references to documents that are to be updated.
    /// * `data_contract`: Reference to the data contract of the documents.
    /// * `document_type`: Reference to the type of the document.
    /// * `drive_operation_types`: A mutable reference to a vector of drive operations.
    ///   The new document update operations are pushed to this vector.
    /// * `drive_version`: Reference to the drive version that informs which version of the method to use.
    ///
    /// # Returns
    ///
    /// This function returns an empty Result if the operation is successful.
    /// If the version is unknown or not implemented, it returns a `DriveError` with
    /// a description of the error.
    ///
    /// # Errors
    ///
    /// This function returns an error variant of `DriveError::UnknownVersionMismatch`
    /// if the method version received does not match with any of the known versions.
    ///
    pub fn add_update_multiple_documents_operations<'a>(
        &self,
        documents: &'a [Document],
        data_contract: &'a DataContract,
        document_type: DocumentTypeRef<'a>,
        drive_operation_types: &mut Vec<DriveOperation<'a>>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        match drive_version
            .methods
            .document
            .update
            .add_update_multiple_documents_operations
        {
            0 => {
                Self::add_update_multiple_documents_operations_v0(
                    documents,
                    data_contract,
                    document_type,
                    drive_operation_types,
                );
                Ok(())
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_update_multiple_documents_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
