use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;
use crate::state_transition::batch_transition::batched_transition::token_order_sell_limit_transition::validate_structure::v0::TokenOrderSellLimitTransitionStructureValidationV0;
use crate::state_transition::batch_transition::batched_transition::token_order_sell_limit_transition::transition::TokenOrderSellLimitTransition;

mod v0;

pub trait TokenOrderSellLimitTransitionStructureValidation {
    fn validate_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}

impl TokenOrderSellLimitTransitionStructureValidation for TokenOrderSellLimitTransition {
    fn validate_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .batch_state_transition
            .token_order_sell_limit_transition_structure_validation
        {
            0 => self.validate_structure_v0(),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "TokenOrderSellLimitTransition::validate_structure".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
