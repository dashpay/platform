use crate::data_contract::document_type::DocumentTypeRef;
use crate::state_transition::batch_transition::batched_transition::document_replace_transition::validate_structure::v0::DocumentReplaceTransitionStructureValidationV0;
use crate::state_transition::batch_transition::batched_transition::DocumentReplaceTransition;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

mod v0;

pub trait DocumentReplaceTransitionStructureValidation {
    fn validate_structure(
        &self,
        document_type: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}

impl DocumentReplaceTransitionStructureValidation for DocumentReplaceTransition {
    fn validate_structure(
        &self,
        document_type: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        // Constructor-only pre-sign helper. The drive-abci action validator
        // has its own version field under
        // `drive_abci.validation_and_processing.state_transitions.batch_state_transition`
        // and is intentionally decoupled from this DPP/SDK pre-sign helper.
        match platform_version
            .dpp
            .state_transitions
            .documents
            .documents_batch_transition
            .validation
            .document_replace_transition_structure_validation
        {
            0 => self.validate_structure_v0(document_type),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DocumentReplaceTransition::validate_structure".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
