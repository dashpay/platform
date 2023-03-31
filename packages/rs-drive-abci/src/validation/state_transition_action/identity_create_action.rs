use crate::validation::state_transition_action::StateTransitionActionValidation;
use dpp::identity::state_transition::identity_create_transition::IdentityCreateTransitionAction;
use dpp::validation::SimpleValidationResult;
use dpp::ProtocolError;
use drive::drive::Drive;

impl StateTransitionActionValidation for IdentityCreateTransitionAction {
    fn validate_fee(&self, drive: &Drive) -> Result<SimpleValidationResult, ProtocolError> {
        todo!()
    }
}
