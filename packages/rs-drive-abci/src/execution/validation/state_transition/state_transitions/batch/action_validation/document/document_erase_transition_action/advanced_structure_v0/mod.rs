use crate::error::Error;
use dpp::consensus::basic::document::{
    InvalidDocumentTransitionActionError, InvalidDocumentTypeError,
};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters};
use dpp::validation::SimpleConsensusValidationResult;
use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_erase_transition_action::v0::DocumentEraseTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_erase_transition_action::DocumentEraseTransitionAction;

pub(in crate::execution::validation::state_transition::state_transitions::batch::action_validation) trait DocumentEraseTransitionActionStructureValidationV0 {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, Error>;
}

impl DocumentEraseTransitionActionStructureValidationV0 for DocumentEraseTransitionAction {
    /// Checks what the contract alone decides: whether this type has retained
    /// revisions at all, whether it allows them to be purged, and whether it is
    /// one of the types the lifecycle keeps out. Whether the particular document
    /// is in a state that can be erased is a question about committed state and
    /// belongs to state validation.
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, Error> {
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

        if !document_type.documents_keep_history() {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                InvalidDocumentTransitionActionError::new(format!(
                    "documents of type {} do not keep history and can not be erased",
                    document_type_name
                ))
                .into(),
            ));
        }

        if !document_type.documents_can_be_erased() {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                InvalidDocumentTransitionActionError::new(format!(
                    "documents of type {} can not be erased",
                    document_type_name
                ))
                .into(),
            ));
        }

        // Defensive, for a contract stored before protocol 14: the parser keeps
        // the two apart for every contract registered since.
        if document_type
            .indexes()
            .values()
            .any(|index| index.contested_index.is_some())
        {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                InvalidDocumentTransitionActionError::new(format!(
                    "documents of keep-history type {} carry a contested index and can not take \
                     part in the document lifecycle",
                    document_type_name
                ))
                .into(),
            ));
        }

        // Erase has no token cost of its own: the deletion it follows was paid
        // for when the document was deleted. Offering one would let a
        // continuation, which anyone may submit, move tokens.
        if self.base().token_cost().is_some() {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                InvalidDocumentTransitionActionError::new(format!(
                    "an erase of a document of type {} must not carry token payment information",
                    document_type_name
                ))
                .into(),
            ));
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}
