use dpp::{
    data_contract::state_transition::data_contract_update_transition::DataContractUpdateTransition,
    state_transition::StateTransitionAction, validation::SimpleValidationResult,
};
use drive::drive::Drive;

use crate::{error::Error, validation::bls::DriveBls};

use super::StateTransitionValidation;

impl StateTransitionValidation for DataContractUpdateTransition {
    fn validate_type(&self, drive: &Drive) -> Result<SimpleValidationResult, Error> {
        todo!()
    }

    fn validate_signature(
        &self,
        drive: &Drive,
        bls: &DriveBls,
    ) -> Result<SimpleValidationResult, Error> {
        todo!()
    }

    fn validate_key_signature(&self, bls: &DriveBls) -> Result<SimpleValidationResult, Error> {
        todo!()
    }

    fn validate_state(
        &self,
        drive: &Drive,
    ) -> Result<dpp::validation::ValidationResult<StateTransitionAction>, Error> {
        todo!()
    }
}
