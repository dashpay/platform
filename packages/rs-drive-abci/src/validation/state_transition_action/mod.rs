mod data_contract_create_action;
mod data_contract_update_action;
mod identity_update_action;
mod identity_top_up_action;
mod identity_credit_withdrawal_action;
mod identity_create_action;
mod documents_batch_action;

use dpp::ProtocolError;
use dpp::validation::SimpleValidationResult;
use drive::drive::Drive;

pub trait StateTransitionActionValidation {
    fn validate_fee(&self, drive: &Drive) -> Result<SimpleValidationResult, ProtocolError>;
}