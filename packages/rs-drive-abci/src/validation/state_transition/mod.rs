mod common;
mod data_contract_create;
mod data_contract_update;
mod document_state_validation;
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
use crate::rpc::core::CoreRPCLike;
use crate::state::PlatformState;
use crate::validation::state_transition::key_validation::validate_state_transition_identity_signature;

/// There are 3 stages in a state transition processing:
/// Structure, Signature and State validation,
///
/// The structure validation verifies that the form of the state transition is good, for example
/// that a contract is well formed, or that a document is valid against the contract.
///
/// Signature validation verifies signatures of a state transition, it will also verify
/// signatures of keys for identity create and identity update. At this stage we will get back
/// a partial identity.
///
/// Validate state verifies that there are no state based conflicts, for example that a document
/// with a unique index isn't already taken.
///
pub fn process_state_transition<'a, C: CoreRPCLike>(
    platform: &'a PlatformRef<C>,
    state_transition: StateTransition,
    transaction: TransactionArg,
) -> Result<ConsensusValidationResult<ExecutionEvent<'a>>, Error> {
    // Validating structure
    let result = state_transition.validate_structure(platform.drive, transaction)?;
    if !result.is_valid() {
        return Ok(ConsensusValidationResult::<ExecutionEvent>::new_with_errors(result.errors));
    }

    // Validating signatures
    let result = state_transition.validate_signatures(platform.drive, transaction)?;
    if !result.is_valid() {
        return Ok(ConsensusValidationResult::<ExecutionEvent>::new_with_errors(result.errors));
    }
    let maybe_identity = result.into_data()?;

    // Validating state
    let result = state_transition.validate_state(platform.drive, platform.core_rpc, transaction)?;

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
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<Option<PartialIdentity>>, Error>;

    fn validate_state<'a, C: CoreRPCLike>(
        &self,
        platform: &'a PlatformRef<C>,
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
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<Option<PartialIdentity>>, Error> {
        match self {
            StateTransition::DataContractCreate(st) => st.validate_signatures(drive, tx),
            StateTransition::DataContractUpdate(st) => st.validate_signatures(drive, tx),
            StateTransition::IdentityCreate(st) => st.validate_signatures(drive, tx),
            StateTransition::IdentityUpdate(st) => st.validate_signatures(drive, tx),
            StateTransition::IdentityTopUp(st) => st.validate_signatures(drive, tx),
            StateTransition::IdentityCreditWithdrawal(st) => st.validate_signatures(drive, tx),
            StateTransition::DocumentsBatch(st) => st.validate_signatures(drive, tx),
        }
    }

    fn validate_state<'a, C: CoreRPCLike>(
        &self,
        platform: &'a PlatformRef<C>,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        match self {
            StateTransition::DataContractCreate(st) => st.validate_state(platform, tx),
            StateTransition::DataContractUpdate(st) => st.validate_state(platform, tx),
            StateTransition::IdentityCreate(st) => st.validate_state(platform, tx),
            StateTransition::IdentityUpdate(st) => st.validate_state(platform, tx),
            StateTransition::IdentityTopUp(st) => st.validate_state(platform, tx),
            StateTransition::IdentityCreditWithdrawal(st) => st.validate_state(platform, tx),
            StateTransition::DocumentsBatch(st) => st.validate_state(platform, tx),
        }
    }
}
