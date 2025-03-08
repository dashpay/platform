use crate::state_transition::batch_transition::token_transfer_transition::validate_structure::v0::TokenTransferTransitionActionStructureValidationV0;
use crate::state_transition::batch_transition::TokenTransferTransition;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_value::Identifier;
use platform_version::version::PlatformVersion;

mod v0;

pub trait TokenTransferTransitionStructureValidation {
    fn validate_structure(
        &self,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}
impl TokenTransferTransitionStructureValidation for TokenTransferTransition {
    fn validate_structure(
        &self,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .batch_state_transition
            .token_transfer_transition_structure_validation
        {
            0 => self.validate_structure_v0(owner_id),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "TokenTransferTransition::validate_structure".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
