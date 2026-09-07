use dpp::consensus::basic::document::{InvalidDocumentTransitionActionError, InvalidDocumentTypeError};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters};
use dpp::validation::SimpleConsensusValidationResult;
use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_delete_transition_action::DocumentDeleteTransitionAction;
use drive::state_transition_action::batch::batched_transition::document_transition::document_delete_transition_action::v0::DocumentDeleteTransitionActionAccessorsV0;

use crate::error::Error;

pub(in crate::execution::validation::state_transition::state_transitions::batch::action_validation) trait DocumentDeleteTransitionActionStructureValidationV0 {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, Error>;
}
impl DocumentDeleteTransitionActionStructureValidationV0 for DocumentDeleteTransitionAction {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, Error> {
        let contract_fetch_info = self.base().data_contract_fetch_info();
        let data_contract = &contract_fetch_info.contract;
        let document_type_name = self.base().document_type_name();

        // Make sure that the document type is defined in the contract
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

        // Pair the delete KIND with the doctype's storage mode: an
        // indexOnly document has no primary-storage row a by-id delete
        // could fetch, so its deletes must come as the indexOnlyDelete
        // (delete-by-values) kind — its structure validation enforces the
        // mirror rule. `index_only()` can only be true on a PV14+
        // contract, so this branch is unreachable for every historical
        // transition.
        if document_type.index_only() {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                InvalidDocumentTransitionActionError::new(format!(
                    "documents of indexOnly type {} must be deleted with an indexOnlyDelete \
                     (delete-by-values) transition carrying the document's values",
                    document_type_name
                ))
                .into(),
            ));
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}
