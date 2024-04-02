mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;

use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::DataContract;

use dpp::identifier::Identifier;
use dpp::platform_value::Value;
use dpp::prelude::{BlockHeight, CoreBlockHeight, TimestampMillis};

use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;
use std::collections::BTreeMap;

/// Represents a request to determine the uniqueness of data.
/// This structure is defined to handle index uniqueness within a document.
/// The purpose is to encapsulate all the required parameters to determine
/// if a particular data is unique or not.
///
/// **Note**: Modifications to this structure are discouraged due to its close coupling
/// with index uniqueness methods. Any change here might necessitate changes across
/// all those methods. Given the likely infrequent need for changes, this design choice
/// is deemed acceptable.
pub(in crate::drive::document::index_uniqueness) struct UniquenessOfDataRequest<'a> {
    /// Reference to the associated data contract.
    pub contract: &'a DataContract,
    /// Reference of the document type.
    pub document_type: DocumentTypeRef<'a>,
    /// The ID representing the owner.
    pub owner_id: Identifier,
    /// The ID of the document in question.
    pub document_id: Identifier,
    /// A flag indicating if the original (existing) document is considered permissible.
    pub allow_original: bool,
    /// Optional timestamp indicating when the document was created.
    pub created_at: Option<TimestampMillis>,
    /// Optional timestamp indicating the last time the document was updated.
    pub updated_at: Option<TimestampMillis>,
    /// Optional timestamp indicating the block height at which the document was created.
    pub created_at_block_height: Option<BlockHeight>,
    /// Optional timestamp indicating the last block height the document was updated.
    pub updated_at_block_height: Option<BlockHeight>,
    /// Optional timestamp indicating the core height at which the document was created.
    pub created_at_core_block_height: Option<CoreBlockHeight>,
    /// Optional timestamp indicating the last core block height the document was updated.
    pub updated_at_core_block_height: Option<CoreBlockHeight>,
    /// The actual data to be checked for uniqueness, represented as a mapping.
    pub data: &'a BTreeMap<String, Value>,
}

impl Drive {
    /// Internal method validating uniqueness
    ///
    /// # Arguments
    ///
    /// * `request` - A `UniquenessOfDataRequest` object representing the request.
    /// * `transaction` - A `TransactionArg` object representing the transaction.
    /// * `drive_version` - A `DriveVersion` object representing the version of the Drive.
    ///
    /// # Returns
    ///
    /// * `Result<SimpleConsensusValidationResult, Error>` - If successful, returns a `SimpleConsensusValidationResult` object representing the result of the validation.
    ///   If an error occurs during the operation, returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function will return an error if the version of the Drive is unknown.
    pub(in crate::drive::document::index_uniqueness) fn validate_uniqueness_of_data(
        &self,
        request: UniquenessOfDataRequest,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match platform_version
            .drive
            .methods
            .document
            .index_uniqueness
            .validate_uniqueness_of_data
        {
            0 => self.validate_uniqueness_of_data_v0(request, transaction, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "validate_uniqueness_of_data".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
