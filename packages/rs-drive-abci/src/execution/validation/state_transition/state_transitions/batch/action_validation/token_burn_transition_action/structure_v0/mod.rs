use dpp::validation::{SimpleConsensusValidationResult};
use drive::state_transition_action::document::documents_batch::document_transition::token_burn_transition_action::{TokenBurnTransitionAction, TokenBurnTransitionActionAccessorsV0};
use dpp::version::PlatformVersion;
use crate::error::Error;
use crate::execution::validation::state_transition::batch::action_validation::token_base_transition_action::TokenBaseTransitionActionValidation;

pub(super) trait TokenBurnTransitionActionStructureValidationV0 {
    fn validate_structure_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}
impl TokenBurnTransitionActionStructureValidationV0 for TokenBurnTransitionAction {
    fn validate_structure_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let validation_result = self.base().validate_structure(platform_version)?;
        if !validation_result.is_valid() {
            return Ok(validation_result);
        }

        Ok(SimpleConsensusValidationResult::default())
    }
}
