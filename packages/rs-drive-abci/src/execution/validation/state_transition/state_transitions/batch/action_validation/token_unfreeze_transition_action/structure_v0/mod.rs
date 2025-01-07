use dpp::validation::{SimpleConsensusValidationResult};
use drive::state_transition_action::document::batch::batched_transition::document_transition::token_unfreeze_transition_action::{TokenUnfreezeTransitionAction, TokenUnfreezeTransitionActionAccessorsV0};
use dpp::version::PlatformVersion;
use crate::error::Error;
use crate::execution::validation::state_transition::batch::action_validation::token_base_transition_action::TokenBaseTransitionActionValidation;

pub(super) trait TokenUnfreezeTransitionActionStructureValidationV0 {
    fn validate_structure_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}
impl TokenUnfreezeTransitionActionStructureValidationV0 for TokenUnfreezeTransitionAction {
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
