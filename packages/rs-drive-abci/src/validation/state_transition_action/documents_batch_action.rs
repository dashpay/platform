use crate::validation::state_transition_action::StateTransitionActionValidation;
use dpp::document::state_transition::documents_batch_transition::DocumentsBatchTransitionAction;
use dpp::validation::SimpleValidationResult;
use dpp::ProtocolError;
use drive::drive::Drive;

impl StateTransitionActionValidation for DocumentsBatchTransitionAction {
    fn validate_fee(&self, drive: &Drive) -> Result<SimpleValidationResult, ProtocolError> {
        todo!()
    }
}
