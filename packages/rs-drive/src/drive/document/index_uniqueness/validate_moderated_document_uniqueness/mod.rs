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
use std::collections::BTreeSet;

impl Drive {
    /// Validates that a document a moderator writes would be unique in the state: no other
    /// document of its type holds a value of one of its unique indexes. The document is
    /// written with its own timestamps and heights, so they are read off the document rather
    /// than off the block.
    ///
    /// A restore brings back a document that is absent while its removal record stands
    /// unrestored, so it is checked as a new document, its own id not allowed as the original
    /// (`changed_fields` is `None`). A field change rewrites a live document, so only the
    /// unique indexes reading one of `changed_fields` are checked, and the document's own
    /// entries do not count as a clash.
    ///
    /// # Parameters
    ///
    /// * `contract`: The contract of the document.
    /// * `document_type`: The document's type, whose unique indexes are checked.
    /// * `document`: The document as the moderator writes it, with its timestamps.
    /// * `changed_fields`: For a field change, the top-level properties it sets; `None` for a
    ///   restore.
    /// * `transaction`: The GroveDB transaction.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok(SimpleConsensusValidationResult)`: valid when no other document holds a value of
    ///   one of the document's unique indexes, otherwise carrying the duplicate unique index
    ///   error.
    /// * `Err(Error)` when the method version is unknown or a unique index query fails.
    pub fn validate_moderated_document_uniqueness(
        &self,
        contract: &DataContract,
        document_type: DocumentTypeRef,
        document: &Document,
        changed_fields: Option<&BTreeSet<String>>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match platform_version
            .drive
            .methods
            .document
            .index_uniqueness
            .validate_moderated_document_uniqueness
        {
            0 => self.validate_moderated_document_uniqueness_v0(
                contract,
                document_type,
                document,
                changed_fields,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "validate_moderated_document_uniqueness".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
