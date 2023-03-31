use crate::validation::state_transition_action::StateTransitionActionValidation;
use dpp::identity::state_transition::identity_update_transition::IdentityUpdateTransitionAction;
use dpp::validation::SimpleValidationResult;
use dpp::ProtocolError;
use drive::drive::Drive;

impl StateTransitionActionValidation for IdentityUpdateTransitionAction {
    fn validate_fee(&self, drive: &Drive) -> Result<SimpleValidationResult, ProtocolError> {}
}
