use dpp::validation::{SimpleConsensusValidationResult};
use drive::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::{TokenBaseTransitionAction, TokenBaseTransitionActionAccessorsV0};
use dpp::version::PlatformVersion;
use crate::error::Error;

pub(in crate::execution::validation::state_transition::state_transitions::batch::action_validation) trait TokenBaseTransitionActionStructureValidationV0 {
    fn validate_structure_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}
impl TokenBaseTransitionActionStructureValidationV0 for TokenBaseTransitionAction {
    fn validate_structure_v0(
        &self,
        _platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        // The rule lives with the action, where a transformer that needs the token's
        // configuration earlier reads it too.
        Ok(match self.token_configuration_or_position_error() {
            Ok(_) => SimpleConsensusValidationResult::default(),
            Err(error) => SimpleConsensusValidationResult::new_with_error(error),
        })
    }
}
