use dpp::{
    identity::state_transition::identity_update_transition::identity_update_transition::IdentityUpdateTransition,
    state_transition::StateTransitionAction,
    validation::{SimpleValidationResult, ValidationResult},
};
use drive::{drive::Drive, grovedb::Transaction};

use crate::error::Error;

use super::StateTransitionValidation;

impl StateTransitionValidation for IdentityUpdateTransition {
    fn validate_type(
        &self,
        drive: &Drive,
        tx: &Transaction,
    ) -> Result<SimpleValidationResult, Error> {
        todo!()
    }

    fn validate_signature(&self, drive: &Drive) -> Result<SimpleValidationResult, Error> {
        todo!()
    }

    fn validate_key_signature(&self) -> Result<SimpleValidationResult, Error> {
        todo!()
    }

    fn validate_state(
        &self,
        drive: &Drive,
        tx: &Transaction,
    ) -> Result<ValidationResult<StateTransitionAction>, Error> {
        todo!()
    }
}
