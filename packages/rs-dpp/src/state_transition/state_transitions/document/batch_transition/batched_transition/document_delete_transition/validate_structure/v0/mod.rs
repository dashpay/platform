use crate::consensus::basic::document::InvalidDocumentTransitionActionError;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::DocumentTypeRef;
use crate::state_transition::batch_transition::batched_transition::DocumentDeleteTransition;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;

pub(super) trait DocumentDeleteTransitionStructureValidationV0 {
    fn validate_structure_v0(
        &self,
        document_type: DocumentTypeRef,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}

impl DocumentDeleteTransitionStructureValidationV0 for DocumentDeleteTransition {
    fn validate_structure_v0(
        &self,
        document_type: DocumentTypeRef,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        // Mirrors the drive-abci action validator deletability check;
        // contract-local and safe pre-sign.
        if !document_type.documents_can_be_deleted() {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                InvalidDocumentTransitionActionError::new(format!(
                    "documents of type {} can not be deleted",
                    document_type.name()
                ))
                .into(),
            ));
        }

        Ok(SimpleConsensusValidationResult::default())
    }
}
