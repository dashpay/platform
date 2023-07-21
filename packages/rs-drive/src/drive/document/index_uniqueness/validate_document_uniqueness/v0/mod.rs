use dpp::data_contract::DataContract;

use crate::drive::Drive;

use crate::drive::document::index_uniqueness::internal::validate_uniqueness_of_data::UniquenessOfDataRequest;
use crate::error::Error;
use crate::query::{DriveQuery, InternalClauses, WhereClause, WhereOperator};
use dpp::consensus::state::document::duplicate_unique_index_error::DuplicateUniqueIndexError;
use dpp::consensus::state::state_error::StateError;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::document::Document;
use dpp::identifier::Identifier;
use dpp::platform_value::{platform_value, Value};
use dpp::prelude::TimestampMillis;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::drive_versions::DriveVersion;
use grovedb::TransactionArg;
use std::collections::BTreeMap;

impl Drive {
    /// Validate that a document would be unique in the state
    pub(super) fn validate_document_uniqueness_v0(
        &self,
        contract: &DataContract,
        document_type: DocumentTypeRef,
        document: &Document,
        owner_id: &Identifier,
        allow_original: bool,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let request = UniquenessOfDataRequest {
            contract,
            document_type,
            owner_id,
            document_id: &document.id,
            allow_original,
            created_at: &document.created_at,
            updated_at: &document.updated_at,
            data: &document.properties,
        };
        self.validate_uniqueness_of_data(request, transaction, drive_version)
    }
}
