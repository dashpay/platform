use dpp::data_contract::DataContract;

use crate::drive::Drive;

use crate::drive::document::index_uniqueness::internal::validate_uniqueness_of_data::UniquenessOfDataRequest;
use crate::error::Error;

use dpp::data_contract::document_type::DocumentTypeRef;

use dpp::identifier::Identifier;

use dpp::validation::SimpleConsensusValidationResult;

use grovedb::TransactionArg;

use crate::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use crate::state_transition_action::document::documents_batch::document_transition::document_replace_transition_action::{DocumentReplaceTransitionAction, DocumentReplaceTransitionActionAccessorsV0};
use dpp::version::PlatformVersion;

impl Drive {
    /// Validate that a document replace transition action would be unique in the state
    #[inline(always)]
    pub(super) fn validate_document_replace_transition_action_uniqueness_v0(
        &self,
        contract: &DataContract,
        document_type: DocumentTypeRef,
        document_replace_transition: &DocumentReplaceTransitionAction,
        owner_id: Identifier,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let request = UniquenessOfDataRequest {
            contract,
            document_type,
            owner_id,
            document_id: document_replace_transition.base().id(),
            allow_original: true,
            created_at: document_replace_transition.created_at(),
            updated_at: document_replace_transition.updated_at(),
            data: document_replace_transition.data(),
        };
        self.validate_uniqueness_of_data(request, transaction, platform_version)
    }
}
