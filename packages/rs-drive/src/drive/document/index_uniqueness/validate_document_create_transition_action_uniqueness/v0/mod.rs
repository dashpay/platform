use dpp::data_contract::DataContract;

use crate::drive::Drive;

use crate::drive::document::index_uniqueness::internal::validate_uniqueness_of_data::UniquenessOfDataRequest;
use crate::error::Error;

use dpp::data_contract::document_type::DocumentTypeRef;

use dpp::identifier::Identifier;

use dpp::validation::SimpleConsensusValidationResult;

use grovedb::TransactionArg;

use crate::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use crate::state_transition_action::document::documents_batch::document_transition::document_create_transition_action::{DocumentCreateTransitionAction, DocumentCreateTransitionActionAccessorsV0};
use dpp::version::PlatformVersion;

impl Drive {
    /// Validate that a document create transition action would be unique in the state
    pub(super) fn validate_document_create_transition_action_uniqueness_v0(
        &self,
        contract: &DataContract,
        document_type: DocumentTypeRef,
        document_create_transition: &DocumentCreateTransitionAction,
        owner_id: Identifier,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let request = UniquenessOfDataRequest {
            contract,
            document_type,
            owner_id,
            document_id: document_create_transition.base().id(),
            allow_original: false,
            created_at: document_create_transition.created_at(),
            updated_at: document_create_transition.created_at(),
            data: document_create_transition.data(),
        };
        self.validate_uniqueness_of_data(request, transaction, platform_version)
    }
}
