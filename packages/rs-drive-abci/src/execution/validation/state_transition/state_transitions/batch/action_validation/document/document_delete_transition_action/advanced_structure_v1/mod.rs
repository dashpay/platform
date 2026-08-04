use dpp::consensus::basic::document::{InvalidDocumentTransitionActionError, InvalidDocumentTypeError};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::validation::SimpleConsensusValidationResult;
use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_delete_transition_action::DocumentDeleteTransitionAction;
use drive::state_transition_action::batch::batched_transition::document_transition::document_delete_transition_action::v0::DocumentDeleteTransitionActionAccessorsV0;

use crate::error::Error;

pub(in crate::execution::validation::state_transition::state_transitions::batch::action_validation) trait DocumentDeleteTransitionActionStructureValidationV1 {
    fn validate_structure_v1(&self) -> Result<SimpleConsensusValidationResult, Error>;
}

impl DocumentDeleteTransitionActionStructureValidationV1 for DocumentDeleteTransitionAction {
    /// V1 adds the `documents_keep_history()` guard alongside the V0
    /// `documents_can_be_deleted()` guard.
    ///
    /// Pre-V1, a delete against a keep-history doctype passed structure
    /// validation, reached `force_delete_document_for_contract_operations_v0`,
    /// and returned `DriveError::InvalidDeletionOfDocumentThatKeepsHistory`.
    /// The batch processor reclassifies that drive-layer error as
    /// `ExecutionResult::InternalError` — the transition is neither valid nor
    /// invalid-paid, leaving the SDK with no clean accept/reject signal.
    ///
    /// Rejecting at the structure layer turns the contradiction into a normal
    /// invalid (paid) consensus error. Gated behind a new validation version
    /// (rather than mutating V0) so PROTOCOL_VERSION_12 and earlier chains —
    /// which historically classified these deletes as InternalError — replay
    /// bit-for-bit. See issue #3927.
    fn validate_structure_v1(&self) -> Result<SimpleConsensusValidationResult, Error> {
        let contract_fetch_info = self.base().data_contract_fetch_info();
        let data_contract = &contract_fetch_info.contract;
        let document_type_name = self.base().document_type_name();

        let Some(document_type) = data_contract.document_type_optional_for_name(document_type_name)
        else {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                InvalidDocumentTypeError::new(document_type_name.clone(), data_contract.id())
                    .into(),
            ));
        };

        if !document_type.documents_can_be_deleted() {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                InvalidDocumentTransitionActionError::new(format!(
                    "documents of type {} can not be deleted",
                    document_type_name
                ))
                .into(),
            ));
        }

        if document_type.documents_keep_history() {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                InvalidDocumentTransitionActionError::new(format!(
                    "documents of type {} keep history and therefore can not be deleted",
                    document_type_name
                ))
                .into(),
            ));
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}
