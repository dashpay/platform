mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::DataContract;
use dpp::document::Document;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Validates that a document a moderator restores would be unique in the state: no other
    /// document of its type holds a value of one of its unique indexes. The document comes
    /// back as it was, timestamps and heights included, so they are read off the document
    /// rather than off the block, and its own id is not allowed as the original: the
    /// document is absent while its removal record stands unrestored.
    pub fn validate_restored_document_uniqueness(
        &self,
        contract: &DataContract,
        document_type: DocumentTypeRef,
        document: &Document,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match platform_version
            .drive
            .methods
            .document
            .index_uniqueness
            .validate_restored_document_uniqueness
        {
            0 => self.validate_restored_document_uniqueness_v0(
                contract,
                document_type,
                document,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "validate_restored_document_uniqueness".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
