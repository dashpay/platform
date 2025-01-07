use dpp::identifier::Identifier;
use dpp::validation::{SimpleConsensusValidationResult};
use drive::state_transition_action::batch::batched_transition::token_transition::token_transfer_transition_action::TokenTransferTransitionAction;
use dpp::version::PlatformVersion;
use drive::state_transition_action::batch::batched_transition::token_transition::token_transfer_transition_action::v0::TokenTransferTransitionActionAccessorsV0;
use crate::error::Error;
use crate::execution::validation::state_transition::batch::action_validation::token_base_transition_action::TokenBaseTransitionActionValidation;

pub(super) trait TokenTransferTransitionActionStructureValidationV0 {
    fn validate_structure_v0(
        &self,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}
impl TokenTransferTransitionActionStructureValidationV0 for TokenTransferTransitionAction {
    fn validate_structure_v0(
        &self,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let validation_result = self.base().validate_structure(platform_version)?;
        if !validation_result.is_valid() {
            return Ok(validation_result);
        }

        Ok(SimpleConsensusValidationResult::default())
    }
}
