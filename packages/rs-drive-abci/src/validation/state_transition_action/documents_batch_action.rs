use dpp::document::state_transition::documents_batch_transition::DocumentsBatchTransitionAction;
use dpp::ProtocolError;
use dpp::validation::SimpleValidationResult;
use drive::drive::Drive;
use crate::validation::state_transition_action::StateTransitionActionValidation;

impl StateTransitionActionValidation for DocumentsBatchTransitionAction {
    fn validate_fee(&self, drive: &Drive) -> Result<SimpleValidationResult, ProtocolError> {
        todo!()
    }
}