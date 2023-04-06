mod common;
mod data_contract_create;
mod data_contract_update;
mod documents_batch;
mod identity_create;
mod identity_credit_withdrawal;
mod identity_top_up;
mod identity_update;
mod key_validation;

use dpp::identity::PartialIdentity;
use dpp::state_transition::{StateTransition, StateTransitionAction};
use dpp::validation::{ConsensusValidationResult, SimpleConsensusValidationResult};
use drive::fee_pools::epochs::Epoch;
use drive::query::TransactionArg;
use drive::{drive::Drive, grovedb::Transaction};

use crate::error::Error;
use crate::execution::execution_event::ExecutionEvent;
use crate::platform::{Platform, PlatformRef};
use crate::state::PlatformState;

pub fn process_state_transition<'a, C>(
    platform: &'a PlatformRef<C>,
    state_transition: StateTransition,
    transaction: TransactionArg,
) -> Result<ConsensusValidationResult<ExecutionEvent<'a>>, Error> {
    // I still insist on better specifying function arguments, that won't allow us to have
    // None for execution context here what Platform in general permits

    let result = state_transition.validate_structure(&platform.drive, transaction)?;
    if !result.is_valid() {
        return Ok(ConsensusValidationResult::<ExecutionEvent>::new_with_errors(result.errors));
    }
    let result = state_transition.validate_signatures(&platform.drive)?;
    if !result.is_valid() {
        return Ok(ConsensusValidationResult::<ExecutionEvent>::new_with_errors(result.errors));
    }

    let maybe_identity = result.into_data()?;

    let result = state_transition.validate_state(&platform.drive, transaction)?;

    result.map_result(|action| (maybe_identity, action, &platform.state.current_epoch).try_into())
}

pub trait StateTransitionValidation {
    fn validate_structure(
        &self,
        drive: &Drive,
        tx: TransactionArg,
    ) -> Result<SimpleConsensusValidationResult, Error>;

    fn validate_signatures(
        &self,
        drive: &Drive,
    ) -> Result<ConsensusValidationResult<Option<PartialIdentity>>, Error>;

    fn validate_state(
        &self,
        drive: &Drive,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl StateTransitionValidation for StateTransition {
    fn validate_structure(
        &self,
        drive: &Drive,
        tx: TransactionArg,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match self {
            StateTransition::DataContractCreate(st) => st.validate_structure(drive, tx),
            StateTransition::DataContractUpdate(st) => st.validate_structure(drive, tx),
            StateTransition::IdentityCreate(st) => st.validate_structure(drive, tx),
            StateTransition::IdentityUpdate(st) => st.validate_structure(drive, tx),
            StateTransition::IdentityTopUp(st) => st.validate_structure(drive, tx),
            StateTransition::IdentityCreditWithdrawal(st) => st.validate_structure(drive, tx),
            StateTransition::DocumentsBatch(st) => st.validate_structure(drive, tx),
        }
    }

    fn validate_signatures(
        &self,
        drive: &Drive,
    ) -> Result<ConsensusValidationResult<Option<PartialIdentity>>, Error> {
        match self {
            StateTransition::DataContractCreate(st) => st.validate_signatures(drive),
            StateTransition::DataContractUpdate(st) => st.validate_signatures(drive),
            StateTransition::IdentityCreate(st) => st.validate_signatures(drive),
            StateTransition::IdentityUpdate(st) => st.validate_signatures(drive),
            StateTransition::IdentityTopUp(st) => st.validate_signatures(drive),
            StateTransition::IdentityCreditWithdrawal(st) => st.validate_signatures(drive),
            StateTransition::DocumentsBatch(st) => st.validate_signatures(drive),
        }
    }

    fn validate_state(
        &self,
        drive: &Drive,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
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
