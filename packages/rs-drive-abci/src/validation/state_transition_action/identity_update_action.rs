use dpp::identity::state_transition::identity_update_transition::IdentityUpdateTransitionAction;
use dpp::ProtocolError;
use dpp::validation::SimpleValidationResult;
use drive::drive::Drive;
use crate::validation::state_transition_action::StateTransitionActionValidation;

impl StateTransitionActionValidation for IdentityUpdateTransitionAction {
    fn validate_fee(&self, drive: &Drive) -> Result<SimpleValidationResult, ProtocolError> {

    }
}