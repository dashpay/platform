use crate::state_transition::batch_transition::token_direct_purchase_to_pool_transition::validate_structure::v0::TokenDirectPurchaseToPoolTransitionActionStructureValidationV0;
use crate::state_transition::batch_transition::TokenDirectPurchaseToPoolTransition;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

mod v0;

pub trait TokenDirectPurchaseToPoolTransitionStructureValidation {
    fn validate_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}

impl TokenDirectPurchaseToPoolTransitionStructureValidation
    for TokenDirectPurchaseToPoolTransition
{
    fn validate_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .batch_state_transition
            .token_direct_purchase_to_pool_transition_structure_validation
        {
            0 => self.validate_structure_v0(platform_version),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "TokenDirectPurchaseToPoolTransition::validate_structure".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
