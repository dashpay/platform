mod data_contract_create_action;
mod data_contract_update_action;
mod documents_batch_action;
mod identity_create_action;
mod identity_credit_withdrawal_action;
mod identity_top_up_action;
mod identity_update_action;

use dpp::state_transition::StateTransitionAction;
use dpp::validation::SimpleValidationResult;
use dpp::ProtocolError;
use drive::drive::Drive;

pub trait StateTransitionActionValidation {
    fn convert_to_execution_event(&self, drive: &Drive) -> Result<SimpleValidationResult, ProtocolError>;
    fn validate_fee(&self, drive: &Drive) -> Result<SimpleValidationResult, ProtocolError>;
}

impl StateTransitionActionValidation for StateTransitionAction {
    fn validate_fee(&self, drive: &Drive) -> Result<SimpleValidationResult, ProtocolError> {
        match self {
            StateTransitionAction::DataContractCreateAction(sta) => sta.validate_fee(drive),
            StateTransitionAction::DataContractUpdateAction(sta) => sta.validate_fee(drive),
            StateTransitionAction::DocumentsBatchAction(sta) => sta.validate_fee(drive),
            StateTransitionAction::IdentityCreateAction(sta) => sta.validate_fee(drive),
            StateTransitionAction::IdentityTopUpAction(sta) => sta.validate_fee(drive),
            StateTransitionAction::IdentityCreditWithdrawalAction(sta) => sta.validate_fee(drive),
            StateTransitionAction::IdentityUpdateAction(sta) => sta.validate_fee(drive),
        }
    }
}
