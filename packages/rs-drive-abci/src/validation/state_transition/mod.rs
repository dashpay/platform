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
use drive::{drive::Drive, grovedb::Transaction};

use crate::error::Error;
use crate::execution::execution_event::ExecutionEvent;
use crate::platform::Platform;

pub fn validate_state_transition<'a, C>(
    platform: &Platform<C>,
    state_transition: StateTransition,
) -> Result<ValidationResult<ExecutionEvent<'a>>, Error> {
    // I still insist on better specifying function arguments, that won't allow us to have
    // None for execution context here what Platform in general permits

    // let tx = platform
    //     .block_execution_context
    //     .read()
    //     .expect("lock is poisoned")
    //     .expect("TODO: there must be a block currently being processed")
    //     .current_transaction;

    let tx: Transaction = todo!();

    let result = state_transition.validate_type(&platform.drive, &tx)?;
    if !result.is_valid() {
        return Ok(ValidationResult::<ExecutionEvent>::new_with_errors(
            result.errors,
        ));
    }
    let result = state_transition.validate_signature(&platform.drive)?;
    if !result.is_valid() {
        return Ok(ValidationResult::<ExecutionEvent>::new_with_errors(
            result.errors,
        ));
    }
    let result = state_transition.validate_key_signature()?;
    if !result.is_valid() {
        return Ok(ValidationResult::<ExecutionEvent>::new_with_errors(
            result.errors,
        ));
    }

    let result = state_transition.validate_state(&platform.drive, &tx)?;

    todo!()
    // if !result.is_valid() {
    //     return Ok(result);
    // } else {
    //     let action = result.into_data()?;
    //     action.validate_fee()
    // }
}

pub trait StateTransitionValidation {
    fn validate_type(
        &self,
        drive: &Drive,
        tx: &Transaction,
    ) -> Result<SimpleValidationResult, Error>;

    fn validate_signature(
        &self,
        drive: &Drive,
    ) -> Result<SimpleValidationResult, Error>;

    fn validate_key_signature(&self) -> Result<SimpleValidationResult, Error>;

    fn validate_state(
        &self,
        drive: &Drive,
        tx: &Transaction,
    ) -> Result<ValidationResult<StateTransitionAction>, Error>;
}

impl StateTransitionValidation for StateTransition {
    fn validate_type(
        &self,
        drive: &Drive,
        tx: &Transaction,
    ) -> Result<SimpleValidationResult, Error> {
        match self {
            StateTransition::DataContractCreate(st) => st.validate_type(drive, tx),
            StateTransition::DataContractUpdate(st) => st.validate_type(drive, tx),
            StateTransition::IdentityCreate(st) => st.validate_type(drive, tx),
            StateTransition::IdentityUpdate(st) => st.validate_type(drive, tx),
            StateTransition::IdentityTopUp(st) => st.validate_type(drive, tx),
            StateTransition::IdentityCreditWithdrawal(st) => st.validate_type(drive, tx),
            StateTransition::DocumentsBatch(st) => st.validate_type(drive, tx),
        }
    }

    fn validate_signature(
        &self,
        drive: &Drive,
    ) -> Result<SimpleValidationResult, Error> {
        match self {
            StateTransition::DataContractCreate(st) => st.validate_signature(drive),
            StateTransition::DataContractUpdate(st) => st.validate_signature(drive),
            StateTransition::IdentityCreate(st) => st.validate_signature(drive),
            StateTransition::IdentityUpdate(st) => st.validate_signature(drive),
            StateTransition::IdentityTopUp(st) => st.validate_signature(drive),
            StateTransition::IdentityCreditWithdrawal(st) => st.validate_signature(drive),
            StateTransition::DocumentsBatch(st) => st.validate_signature(drive),
        }
    }

    fn validate_key_signature(&self) -> Result<SimpleValidationResult, Error> {
        match self {
            StateTransition::DataContractCreate(st) => st.validate_key_signature(),
            StateTransition::DataContractUpdate(st) => st.validate_key_signature(),
            StateTransition::IdentityCreate(st) => st.validate_key_signature(),
            StateTransition::IdentityUpdate(st) => st.validate_key_signature(),
            StateTransition::IdentityTopUp(st) => st.validate_key_signature(),
            StateTransition::IdentityCreditWithdrawal(st) => st.validate_key_signature(),
            StateTransition::DocumentsBatch(st) => st.validate_key_signature(),
        }
    }

    fn validate_state(
        &self,
        drive: &Drive,
        tx: &Transaction,
    ) -> Result<ValidationResult<StateTransitionAction>, Error> {
        match self {
            StateTransition::DataContractCreate(st) => st.validate_state(drive, tx),
            StateTransition::DataContractUpdate(st) => st.validate_state(drive, tx),
            StateTransition::IdentityCreate(st) => st.validate_state(drive, tx),
            StateTransition::IdentityUpdate(st) => st.validate_state(drive, tx),
            StateTransition::IdentityTopUp(st) => st.validate_state(drive, tx),
            StateTransition::IdentityCreditWithdrawal(st) => st.validate_state(drive, tx),
            StateTransition::DocumentsBatch(st) => st.validate_state(drive, tx),
        }
    }
}
