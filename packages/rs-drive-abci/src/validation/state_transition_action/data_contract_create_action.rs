use dpp::data_contract::state_transition::data_contract_create_transition::DataContractCreateTransitionAction;
use dpp::ProtocolError;
use dpp::validation::SimpleValidationResult;
use drive::drive::Drive;
use crate::validation::state_transition_action::StateTransitionActionValidation;

impl StateTransitionActionValidation for DataContractCreateTransitionAction {
    fn validate_fee(&self, drive: &Drive) -> Result<SimpleValidationResult, ProtocolError> {
        todo!()
    }
}