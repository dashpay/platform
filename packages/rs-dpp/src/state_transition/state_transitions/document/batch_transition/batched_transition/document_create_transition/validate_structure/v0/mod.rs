use crate::consensus::basic::document::InvalidDocumentTransitionIdError;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::document::Document;
use crate::state_transition::batch_transition::document_base_transition::document_base_transition_trait::DocumentBaseTransitionAccessors;
use crate::state_transition::batch_transition::document_base_transition::v0::v0_methods::DocumentBaseTransitionV0Methods;
use crate::state_transition::batch_transition::document_create_transition::v0::v0_methods::DocumentCreateTransitionV0Methods;
use crate::state_transition::batch_transition::DocumentCreateTransition;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_value::Identifier;

pub(super) trait DocumentCreateTransitionStructureValidationV0 {
    fn validate_structure_v0(
        &self,
        owner_id: Identifier,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}

impl DocumentCreateTransitionStructureValidationV0 for DocumentCreateTransition {
    fn validate_structure_v0(
        &self,
        owner_id: Identifier,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        let (expected_id, invalid_id) = match self {
            DocumentCreateTransition::V0(transition) => (
                Document::generate_document_id_v0(
                    &transition.base().data_contract_id(),
                    &owner_id,
                    transition.base().document_type_name(),
                    &transition.entropy(),
                ),
                transition.base().id(),
            ),
        };

        if invalid_id != expected_id {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                ConsensusError::BasicError(BasicError::InvalidDocumentTransitionIdError(
                    InvalidDocumentTransitionIdError::new(expected_id, invalid_id),
                )),
            ));
        }

        Ok(SimpleConsensusValidationResult::default())
    }
}
