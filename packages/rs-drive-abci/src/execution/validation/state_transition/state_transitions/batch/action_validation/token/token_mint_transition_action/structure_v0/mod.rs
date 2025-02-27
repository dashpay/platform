use dpp::validation::{SimpleConsensusValidationResult};
use drive::state_transition_action::batch::batched_transition::token_transition::token_mint_transition_action::{TokenMintTransitionAction, TokenMintTransitionActionAccessorsV0};
use dpp::version::PlatformVersion;
use crate::error::Error;
use crate::execution::validation::state_transition::batch::action_validation::token::token_base_transition_action::TokenBaseTransitionActionValidation;

pub(in crate::execution::validation::state_transition::state_transitions::batch::action_validation) trait TokenMintTransitionActionStructureValidationV0 {
    fn validate_structure_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}
impl TokenMintTransitionActionStructureValidationV0 for TokenMintTransitionAction {
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
