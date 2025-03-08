use platform_version::version::PlatformVersion;
use crate::ProtocolError;
use crate::state_transition::batch_transition::token_emergency_action_transition::validate_structure::v0::TokenEmergencyActionTransitionStructureValidationV0;
use crate::state_transition::batch_transition::TokenEmergencyActionTransition;
use crate::validation::SimpleConsensusValidationResult;

mod v0;

pub trait TokenEmergencyActionTransitionStructureValidation {
    fn validate_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}

impl TokenEmergencyActionTransitionStructureValidation for TokenEmergencyActionTransition {
    fn validate_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .batch_state_transition
            .token_emergency_action_transition_structure_validation
        {
            0 => self.validate_structure_v0(),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "TokenEmergencyActionTransition::validate_structure".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
