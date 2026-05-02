use crate::state_transition::batch_transition::document_create_transition::validate_structure::v0::DocumentCreateTransitionStructureValidationV0;
use crate::state_transition::batch_transition::DocumentCreateTransition;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_value::Identifier;
use platform_version::version::PlatformVersion;

mod v0;

pub trait DocumentCreateTransitionStructureValidation {
    fn validate_structure(
        &self,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}

impl DocumentCreateTransitionStructureValidation for DocumentCreateTransition {
    fn validate_structure(
        &self,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        // Dispatch via the DPP-owned version field. The drive-abci action
        // validator has a separate field under
        // `drive_abci.validation_and_processing.state_transitions.batch_state_transition`
        // and is intentionally decoupled from this DPP/SDK pre-sign helper.
        match platform_version
            .dpp
            .state_transitions
            .documents
            .documents_batch_transition
            .validation
            .document_create_transition_structure_validation
        {
            0 => self.validate_structure_v0(owner_id),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DocumentCreateTransition::validate_structure".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
