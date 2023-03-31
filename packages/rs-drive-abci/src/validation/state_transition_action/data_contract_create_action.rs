use crate::validation::state_transition_action::StateTransitionActionValidation;
use dpp::data_contract::state_transition::data_contract_create_transition::DataContractCreateTransitionAction;
use dpp::validation::SimpleValidationResult;
use dpp::ProtocolError;
use drive::drive::Drive;

impl StateTransitionActionValidation for DataContractCreateTransitionAction {
    fn validate_fee(&self, drive: &Drive) -> Result<SimpleValidationResult, ProtocolError> {
        drive
    }
}
