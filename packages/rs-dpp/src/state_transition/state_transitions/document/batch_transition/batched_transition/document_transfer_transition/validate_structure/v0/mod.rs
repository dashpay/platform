use crate::consensus::basic::document::InvalidDocumentTransitionActionError;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::DocumentTypeRef;
use crate::state_transition::batch_transition::batched_transition::DocumentTransferTransition;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;

pub(super) trait DocumentTransferTransitionStructureValidationV0 {
    fn validate_structure_v0(
        &self,
        document_type: DocumentTypeRef,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}

impl DocumentTransferTransitionStructureValidationV0 for DocumentTransferTransition {
    fn validate_structure_v0(
        &self,
        document_type: DocumentTypeRef,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        // Mirrors the drive-abci action validator transferability check;
        // contract-local and safe pre-sign.
        if !document_type.documents_transferable().is_transferable() {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                InvalidDocumentTransitionActionError::new(format!(
                    "{} is not a transferable document type",
                    document_type.name()
                ))
                .into(),
            ));
        }

        Ok(SimpleConsensusValidationResult::default())
    }
}
