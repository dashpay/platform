use dpp::consensus::ConsensusError;
use dpp::consensus::state::document::document_not_found_error::DocumentNotFoundError;
use dpp::consensus::state::document::document_owner_id_mismatch_error::DocumentOwnerIdMismatchError;
use dpp::consensus::state::state_error::StateError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::document::{Document, DocumentV0Getters};
use dpp::identifier::Identifier;
use dpp::prelude::ConsensusValidationResult;
use dpp::validation::SimpleConsensusValidationResult;
use drive::state_transition_action::document::documents_batch::document_transition::document_delete_transition_action::DocumentDeleteTransitionAction;
use dpp::version::PlatformVersion;
use drive::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use drive::state_transition_action::document::documents_batch::document_transition::document_delete_transition_action::v0::DocumentDeleteTransitionActionAccessorsV0;
use crate::error::Error;

pub(super) trait DocumentDeleteTransitionActionStateValidationV0 {
    fn validate_state_v0(
        &self,
        fetched_documents: &[Document],
        owner_id: Identifier,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}
impl DocumentDeleteTransitionActionStateValidationV0 for DocumentDeleteTransitionAction {
    fn validate_state_v0(
        &self,
        fetched_documents: &[Document],
        owner_id: Identifier,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let validation_result =
            check_if_document_can_be_found(self, fetched_documents);

        if !validation_result.is_valid_with_data() {
            return Ok(SimpleConsensusValidationResult::new_with_errors(validation_result.errors));
        }

        let original_document = validation_result.into_data()?;

        Ok(check_ownership(self, original_document, &owner_id))
    }
}

pub fn check_if_document_can_be_found<'a>(
    document_transition: &'a DocumentDeleteTransitionAction,
    fetched_documents: &'a [Document],
) -> ConsensusValidationResult<&'a Document> {
    let maybe_fetched_document = fetched_documents
        .iter()
        .find(|d| d.id() == document_transition.base().id());

    if let Some(document) = maybe_fetched_document {
        ConsensusValidationResult::new_with_data(document)
    } else {
        ConsensusValidationResult::new_with_error(ConsensusError::StateError(
            StateError::DocumentNotFoundError(DocumentNotFoundError::new(
                document_transition.base().id(),
            )),
        ))
    }
}

fn check_ownership(
    document_transition: &DocumentDeleteTransitionAction,
    fetched_document: &Document,
    owner_id: &Identifier,
) -> SimpleConsensusValidationResult {
    let mut result = SimpleConsensusValidationResult::default();
    if fetched_document.owner_id() != owner_id {
        result.add_error(ConsensusError::StateError(
            StateError::DocumentOwnerIdMismatchError(DocumentOwnerIdMismatchError::new(
                document_transition.base().id(),
                owner_id.to_owned(),
                fetched_document.owner_id(),
            )),
        ));
    }
    result
}