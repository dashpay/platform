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
use dpp::state_repository::StateRepositoryLike;
use dpp::state_transition::state_transition_execution_context::StateTransitionExecutionContext;
use dpp::state_transition::validation::validate_state_transition_by_type::ValidatorByStateTransitionType;
use dpp::state_transition::{
    StateTransition, StateTransitionAction, StateTransitionConvert, StateTransitionLike,
};
use dpp::validation::{ConsensusValidationResult, SimpleConsensusValidationResult};
use dpp::{BlsModule, DashPlatformProtocol};
use drive::drive::Drive;
use drive::query::TransactionArg;

use crate::error::Error;
use crate::execution::execution_event::ExecutionEvent;
use crate::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;

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
pub fn process_state_transition<'a, C, SR, BLS>(
    state_transition: StateTransition,
    dpp: &DashPlatformProtocol<SR, BLS>,
    platform: &'a PlatformRef<C>,
    transaction: TransactionArg,
) -> Result<ConsensusValidationResult<ExecutionEvent<'a>>, Error>
where
    C: CoreRPCLike,
    SR: StateRepositoryLike + Clone,
    BLS: BlsModule + Clone,
{
    // Run DPP validation methods and print consensus errors so we can see
    // the difference between DPP and Drive-based validation

    let execution_context = StateTransitionExecutionContext::default();

    // Basic DPP validation
    let result = dpp
        .state_transitions
        .basic_validator
        .validate_state_transition_by_type
        .validate(
            &state_transition.to_object(false)?,
            state_transition.get_type(),
            &execution_context,
        )?;

    if result.is_valid() {
        // Signature validation
        let result = dpp
            .state_transitions
            .validate_signature(state_transition.clone(), &execution_context)?;

        if result.is_valid() {
            // State validation
            let result = dpp
                .state_transitions
                .validate_state(&state_transition, &execution_context)?;

            if !result.is_valid() {
                dbg!("State validation errors", &result.errors);
            }
        } else {
            dbg!("Signature validation errors", &result.errors);
        }
    } else {
        dbg!("Structure validation errors", &result.errors);
    }

    // Run Drive-based validation

    // Validating structure
    let result = state_transition.validate_structure(platform.drive, transaction)?;
    if !result.is_valid() {
        return Ok(ConsensusValidationResult::<ExecutionEvent>::new_with_errors(result.errors));
    }

    // Validating signatures
    let result = state_transition.validate_identity_and_signatures(platform.drive, transaction)?;
    if !result.is_valid() {
        return Ok(ConsensusValidationResult::<ExecutionEvent>::new_with_errors(result.errors));
    }
    let maybe_identity = result.into_data()?;

    // Validating state
    let result = state_transition.validate_state(platform, transaction)?;

    result.map_result(|action| (maybe_identity, action, &platform.state.epoch()).try_into())
}

/// A trait for validating state transitions within a blockchain.
pub trait StateTransitionValidation {
    /// Validates the structure of a transaction by checking its basic elements.
    ///
    /// # Arguments
    ///
    /// * `drive` - A reference to the drive containing the transaction data.
    /// * `tx` - The transaction argument to be checked.
    ///
    /// # Returns
    ///
    /// * `Result<SimpleConsensusValidationResult, Error>` - A result with either a SimpleConsensusValidationResult or an Error.
    fn validate_structure(
        &self,
        drive: &Drive,
        tx: TransactionArg,
    ) -> Result<SimpleConsensusValidationResult, Error>;

    /// Validates the identity and signatures of a transaction to ensure its authenticity.
    ///
    /// # Arguments
    ///
    /// * `drive` - A reference to the drive containing the transaction data.
    /// * `tx` - The transaction argument to be authenticated.
    ///
    /// # Returns
    ///
    /// * `Result<ConsensusValidationResult<Option<PartialIdentity>>, Error>` - A result with either a ConsensusValidationResult containing an optional PartialIdentity or an Error.
    fn validate_identity_and_signatures(
        &self,
        drive: &Drive,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<Option<PartialIdentity>>, Error>;

    /// Validates the state transition by analyzing the changes in the platform state after applying the transaction.
    ///
    /// # Arguments
    ///
    /// * `platform` - A reference to the platform containing the state data.
    /// * `tx` - The transaction argument to be applied.
    ///
    /// # Type Parameters
    ///
    /// * `C: CoreRPCLike` - A type constraint indicating that C should implement `CoreRPCLike`.
    ///
    /// # Returns
    ///
    /// * `Result<ConsensusValidationResult<StateTransitionAction>, Error>` - A result with either a ConsensusValidationResult containing a StateTransitionAction or an Error.
    fn validate_state<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;

    /// Transforms a `TransactionArg` into a `StateTransitionAction`, primarily for testing purposes.
    ///
    /// This function should not be called directly in production since the functionality is already contained within `validate_state`.
    ///
    /// # Type Parameters
    /// * `C`: A type implementing the `CoreRPCLike` trait.
    ///
    /// # Arguments
    /// * `platform`: A reference to a platform implementing CoreRPCLike.
    /// * `tx`: The `TransactionArg` to be transformed into a `StateTransitionAction`.
    ///
    /// # Returns
    /// A `Result` containing either a `ConsensusValidationResult<StateTransitionAction>` or an `Error`.
    fn transform_into_action<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
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

    fn validate_identity_and_signatures(
        &self,
        drive: &Drive,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<Option<PartialIdentity>>, Error> {
        match self {
            StateTransition::DataContractCreate(st) => {
                st.validate_identity_and_signatures(drive, tx)
            }
            StateTransition::DataContractUpdate(st) => {
                st.validate_identity_and_signatures(drive, tx)
            }
            StateTransition::IdentityCreate(st) => st.validate_identity_and_signatures(drive, tx),
            StateTransition::IdentityUpdate(st) => st.validate_identity_and_signatures(drive, tx),
            StateTransition::IdentityTopUp(st) => st.validate_identity_and_signatures(drive, tx),
            StateTransition::IdentityCreditWithdrawal(st) => {
                st.validate_identity_and_signatures(drive, tx)
            }
            StateTransition::DocumentsBatch(st) => st.validate_identity_and_signatures(drive, tx),
        }
    }

    fn validate_state<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
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

    fn transform_into_action<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        match self {
            StateTransition::DataContractCreate(st) => st.transform_into_action(platform, tx),
            StateTransition::DataContractUpdate(st) => st.transform_into_action(platform, tx),
            StateTransition::IdentityCreate(st) => st.transform_into_action(platform, tx),
            StateTransition::IdentityUpdate(st) => st.transform_into_action(platform, tx),
            StateTransition::IdentityTopUp(st) => st.transform_into_action(platform, tx),
            StateTransition::IdentityCreditWithdrawal(st) => st.transform_into_action(platform, tx),
            StateTransition::DocumentsBatch(st) => st.transform_into_action(platform, tx),
        }
    }
}
