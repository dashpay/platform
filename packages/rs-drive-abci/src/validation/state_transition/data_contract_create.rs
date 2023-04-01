use crate::error::Error;
use crate::platform::Platform;
use crate::validation::state_transition::key_validation::validate_state_transition_identity_signature;
use crate::validation::state_transition::StateTransitionValidation;
use dpp::data_contract::state_transition::data_contract_create_transition::DataContractCreateTransition;
use dpp::prelude::ValidationResult;
use dpp::state_transition::StateTransitionAction;
use dpp::validation::SimpleValidationResult;
use dpp::ProtocolError;
use drive::drive::Drive;

impl<C> StateTransitionValidation<C> for DataContractCreateTransition {
    fn validate_type(&self) -> Result<SimpleValidationResult, Error> {
        todo!()
    }

    fn validate_signature(&self) -> Result<SimpleValidationResult, Error> {
        todo!()
    }

    fn validate_key_signature(&self, drive: &Drive) -> Result<SimpleValidationResult, Error> {
        validate_state_transition_identity_signature()
    }

    fn validate_state(
        &self,
        drive: &Drive,
    ) -> Result<ValidationResult<StateTransitionAction>, Error> {
        todo!()
    }
}
