use crate::validation::state_transition_action::StateTransitionActionValidation;
use dpp::identity::state_transition::identity_topup_transition::IdentityTopUpTransitionAction;
use dpp::validation::SimpleValidationResult;
use dpp::ProtocolError;
use drive::drive::Drive;

impl StateTransitionActionValidation for IdentityTopUpTransitionAction {
    fn validate_fee(&self, drive: &Drive) -> Result<SimpleValidationResult, ProtocolError> {}
}
