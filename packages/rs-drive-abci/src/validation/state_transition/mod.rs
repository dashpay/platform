mod data_contract_create;
mod data_contract_update;
mod documents_batch;
mod identity_create;
mod identity_credit_withdrawal;
mod identity_top_up;
mod identity_update;
mod key_validation;

use dpp::state_transition::{StateTransition, StateTransitionAction};
use dpp::validation::{SimpleValidationResult, ValidationResult};
use drive::drive::Drive;

use super::bls::DriveBls;
use crate::error::Error;
use crate::execution::execution_event::ExecutionEvent;
use crate::platform::Platform;

pub fn validate_state_transition<'a, C>(
    platform: &Platform<C>,
    bls: &DriveBls,
    state_transition: StateTransition,
) -> Result<ValidationResult<ExecutionEvent<'a>>, Error> {
    let result = state_transition.validate_type(&platform.drive)?;
    if !result.is_valid() {
        return Ok(ValidationResult::<ExecutionEvent>::new_with_errors(
            result.errors,
        ));
    }
    let result = state_transition.validate_signature(&platform.drive, &bls)?;
    if !result.is_valid() {
        return Ok(ValidationResult::<ExecutionEvent>::new_with_errors(
            result.errors,
        ));
    }
    let result = state_transition.validate_key_signature(&bls)?;
    if !result.is_valid() {
        return Ok(ValidationResult::<ExecutionEvent>::new_with_errors(
            result.errors,
        ));
    }
    let result = state_transition.validate_state(&platform.drive)?;

    todo!()
    // if !result.is_valid() {
    //     return Ok(result);
    // } else {
    //     let action = result.into_data()?;
    //     action.validate_fee()
    // }
}

pub trait StateTransitionValidation {
    fn validate_type(&self, drive: &Drive) -> Result<SimpleValidationResult, Error>;

    fn validate_signature(
        &self,
        drive: &Drive,
        bls: &DriveBls,
    ) -> Result<SimpleValidationResult, Error>;

    fn validate_key_signature(&self, bls: &DriveBls) -> Result<SimpleValidationResult, Error>;

    fn validate_state(
        &self,
        drive: &Drive,
    ) -> Result<ValidationResult<StateTransitionAction>, Error>;
}

impl StateTransitionValidation for StateTransition {
    fn validate_type(&self, drive: &Drive) -> Result<SimpleValidationResult, Error> {
        match self {
            StateTransition::DataContractCreate(st) => st.validate_type(drive),
            StateTransition::DataContractUpdate(st) => st.validate_type(drive),
            StateTransition::IdentityCreate(st) => st.validate_type(drive),
            StateTransition::IdentityUpdate(st) => st.validate_type(drive),
            StateTransition::IdentityTopUp(st) => st.validate_type(drive),
            StateTransition::IdentityCreditWithdrawal(st) => st.validate_type(drive),
            StateTransition::DocumentsBatch(st) => st.validate_type(drive),
        }
    }

    fn validate_signature(
        &self,
        drive: &Drive,
        bls: &DriveBls,
    ) -> Result<SimpleValidationResult, Error> {
        match self {
            StateTransition::DataContractCreate(st) => st.validate_signature(drive, bls),
            StateTransition::DataContractUpdate(st) => st.validate_signature(drive, bls),
            StateTransition::IdentityCreate(st) => st.validate_signature(drive, bls),
            StateTransition::IdentityUpdate(st) => st.validate_signature(drive, bls),
            StateTransition::IdentityTopUp(st) => st.validate_signature(drive, bls),
            StateTransition::IdentityCreditWithdrawal(st) => st.validate_signature(drive, bls),
            StateTransition::DocumentsBatch(st) => st.validate_signature(drive, bls),
        }
    }

    fn validate_key_signature(&self, bls: &DriveBls) -> Result<SimpleValidationResult, Error> {
        match self {
            StateTransition::DataContractCreate(st) => st.validate_key_signature(bls),
            StateTransition::DataContractUpdate(st) => st.validate_key_signature(bls),
            StateTransition::IdentityCreate(st) => st.validate_key_signature(bls),
            StateTransition::IdentityUpdate(st) => st.validate_key_signature(bls),
            StateTransition::IdentityTopUp(st) => st.validate_key_signature(bls),
            StateTransition::IdentityCreditWithdrawal(st) => st.validate_key_signature(bls),
            StateTransition::DocumentsBatch(st) => st.validate_key_signature(bls),
        }
    }

    fn validate_state(
        &self,
        drive: &Drive,
    ) -> Result<ValidationResult<StateTransitionAction>, Error> {
        match self {
            StateTransition::DataContractCreate(st) => st.validate_state(drive),
            StateTransition::DataContractUpdate(st) => st.validate_state(drive),
            StateTransition::IdentityCreate(st) => st.validate_state(drive),
            StateTransition::IdentityUpdate(st) => st.validate_state(drive),
            StateTransition::IdentityTopUp(st) => st.validate_state(drive),
            StateTransition::IdentityCreditWithdrawal(st) => st.validate_state(drive),
            StateTransition::DocumentsBatch(st) => st.validate_state(drive),
        }
    }
}
