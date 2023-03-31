use crate::platform::Platform;
use crate::validation::state_transition::StateTransitionValidation;
use dpp::data_contract::state_transition::data_contract_create_transition::DataContractCreateTransition;
use dpp::prelude::ValidationResult;
use dpp::state_transition::StateTransitionAction;
use dpp::validation::SimpleValidationResult;
use dpp::ProtocolError;
use drive::drive::Drive;

impl<C> StateTransitionValidation<C> for DataContractCreateTransition {
    fn validate_type(&self) -> Result<SimpleValidationResult, ProtocolError> {
        todo!()
    }

    fn validate_signature(&self) -> Result<SimpleValidationResult, ProtocolError> {
        todo!()
    }

    fn validate_key_signature(&self) -> Result<SimpleValidationResult, ProtocolError> {
        todo!()
    }

    fn validate_fee(&self) -> Result<SimpleValidationResult, ProtocolError> {
        todo!()
    }

    fn validate_state(
        &self,
        drive: &Drive,
    ) -> Result<ValidationResult<StateTransitionAction>, ProtocolError> {
        todo!()
    }
}
