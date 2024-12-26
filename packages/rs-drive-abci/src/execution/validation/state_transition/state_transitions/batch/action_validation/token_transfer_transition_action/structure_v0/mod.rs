use dpp::identifier::Identifier;
use dpp::validation::{SimpleConsensusValidationResult};
use drive::state_transition_action::document::documents_batch::document_transition::token_transfer_transition_action::{TokenTransferTransitionAction, TokenTransferTransitionActionAccessors};
use dpp::version::PlatformVersion;
use drive::state_transition_action::document::documents_batch::document_transition::token_base_transition_action::TokenBaseTransitionActionAccessorsV0;
use crate::error::Error;

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
        let token_configuration = self.base().token_configuration()?;

        Ok(SimpleConsensusValidationResult::default())
    }
}
