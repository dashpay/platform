mod v0;

use crate::contract::Contract;
use crate::drive::Drive;
use crate::error::Error;
use crate::query::{DriveQuery, InternalClauses, WhereClause, WhereOperator};
use dpp::consensus::state::document::duplicate_unique_index_error::DuplicateUniqueIndexError;
use dpp::consensus::state::state_error::StateError;
use dpp::data_contract::document_type::DocumentType;
use dpp::document::Document;
use dpp::identifier::Identifier;
use dpp::platform_value::{platform_value, Value};
use dpp::prelude::TimestampMillis;
use dpp::validation::SimpleConsensusValidationResult;
use grovedb::TransactionArg;
use std::collections::BTreeMap;
use dpp::data_contract::DataContract;
use dpp::state_transition::documents_batch_transition::document_transition::{DocumentCreateTransitionAction, DocumentReplaceTransitionAction};
use dpp::version::drive_versions::DriveVersion;
use crate::error::drive::DriveError;

// We don't create an enum version of this
// If this would ever need to be changed all index uniqueness methods would need to be changed
// Which is an okay trade off as this should seldom ever be changed
pub(in crate::drive::document::index_uniqueness) struct UniquenessOfDataRequestV0<'a> {
    pub contract: &'a DataContract,
    pub document_type: &'a DocumentType<'a>,
    pub owner_id: &'a Identifier,
    pub document_id: &'a Identifier,
    pub allow_original: bool,
    pub created_at: &'a Option<TimestampMillis>,
    pub updated_at: &'a Option<TimestampMillis>,
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
        drive_version: &DriveVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match drive_version.methods.document.index_uniqueness.validate_uniqueness_of_data {
            0 => self.validate_uniqueness_of_data_v0(request, transaction, drive_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "validate_uniqueness_of_data".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
