use crate::data_contract::document_type::DocumentTypeRef;
use crate::state_transition::batch_transition::batched_transition::document_delete_transition::validate_structure::v0::DocumentDeleteTransitionStructureValidationV0;
use crate::state_transition::batch_transition::batched_transition::DocumentDeleteTransition;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

mod v0;

pub(crate) trait DocumentDeleteTransitionStructureValidation {
    fn validate_structure(
        &self,
        document_type: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}

impl DocumentDeleteTransitionStructureValidation for DocumentDeleteTransition {
    fn validate_structure(
        &self,
        document_type: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        match platform_version
            .dpp
            .state_transitions
            .documents
            .documents_batch_transition
            .validation
            .document_delete_transition_structure_validation
        {
            0 => self.validate_structure_v0(document_type),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DocumentDeleteTransition::validate_structure".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
