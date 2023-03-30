use dpp::data_contract::state_transition::data_contract_update_transition::DataContractUpdateTransitionAction;
use dpp::ProtocolError;
use dpp::validation::SimpleValidationResult;
use drive::drive::Drive;
use crate::validation::state_transition_action::StateTransitionActionValidation;

impl StateTransitionActionValidation for DataContractUpdateTransitionAction {
    fn validate_fee(&self, drive: &Drive) -> Result<SimpleValidationResult, ProtocolError> {
        todo!()
    }
}