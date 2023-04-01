use dpp::{
    document::DocumentsBatchTransition,
    state_transition::StateTransitionAction,
    validation::{SimpleValidationResult, ValidationResult},
};
use drive::drive::Drive;

use crate::{error::Error, validation::bls::DriveBls};

use super::StateTransitionValidation;

impl StateTransitionValidation for DocumentsBatchTransition {
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
    ) -> Result<ValidationResult<StateTransitionAction>, Error> {
        todo!()
    }
}
