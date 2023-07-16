use dpp::data_contract::DataContract;

use crate::drive::Drive;

use crate::drive::document::index_uniqueness::internal::validate_uniqueness_of_data::UniquenessOfDataRequestV0;
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
use dpp::state_transition_action::document::documents_batch::document_transition::document_replace_transition_action::DocumentReplaceTransitionAction;

impl Drive {
    /// Validate that a document replace transition action would be unique in the state
    pub(super) fn validate_document_replace_transition_action_uniqueness_v0(
        &self,
        contract: &DataContract,
        document_type: &DocumentTypeRef,
        document_replace_transition: &DocumentReplaceTransitionAction,
        owner_id: &Identifier,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let request = UniquenessOfDataRequestV0 {
            contract,
            document_type,
            owner_id,
            document_id: &document_replace_transition.base.id,
            allow_original: true,
            created_at: &document_replace_transition.created_at,
            updated_at: &document_replace_transition.updated_at,
            data: &document_replace_transition.data,
        };
        self.validate_uniqueness_of_data(request, transaction, drive_version)
    }
}
