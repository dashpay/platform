mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;

use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::DataContract;

use dpp::identifier::Identifier;
use dpp::platform_value::Value;
use dpp::prelude::TimestampMillis;
use dpp::validation::SimpleConsensusValidationResult;

use dpp::version::PlatformVersion;
use grovedb::TransactionArg;
use std::collections::BTreeMap;

/// Uniqueness of data request
// We don't create an enum version of this
// If this would ever need to be changed all index uniqueness methods would need to be changed
// Which is an okay trade off as this should seldom ever be changed
pub(in crate::drive::document::index_uniqueness) struct UniquenessOfDataRequest<'a> {
    /// Contract
    pub contract: &'a DataContract,
    /// Document type
    pub document_type: DocumentTypeRef<'a>,
    /// Owner ID
    pub owner_id: Identifier,
    /// Document ID
    pub document_id: Identifier,
    /// Allow original?
    pub allow_original: bool,
    /// Created at
    pub created_at: Option<TimestampMillis>,
    /// Updated at
    pub updated_at: Option<TimestampMillis>,
    /// Data
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
