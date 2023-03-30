use dpp::identity::state_transition::identity_create_transition::IdentityCreateTransitionAction;
use dpp::ProtocolError;
use dpp::validation::SimpleValidationResult;
use drive::drive::Drive;
use crate::validation::state_transition_action::StateTransitionActionValidation;

impl StateTransitionActionValidation for IdentityCreateTransitionAction {
    fn validate_fee(&self, drive: &Drive) -> Result<SimpleValidationResult, ProtocolError> {
        todo!()
    }
}