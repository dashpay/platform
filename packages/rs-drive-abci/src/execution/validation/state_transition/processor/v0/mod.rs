use crate::error::Error;
use crate::execution::types::execution_event::ExecutionEvent;
use crate::execution::validation::state_transition::transformer::StateTransitionActionTransformerV0;
use crate::platform_types::platform::PlatformRef;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::rpc::core::CoreRPCLike;
use dpp::identity::PartialIdentity;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::{StateTransition, StateTransitionAction};
use dpp::state_transition_action::StateTransitionAction;
use dpp::validation::SimpleConsensusValidationResult;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;

pub(in crate::execution) fn process_state_transition_v0<'a, C: CoreRPCLike>(
    platform: &'a PlatformRef<C>,
    state_transition: StateTransition,
    transaction: TransactionArg,
) -> Result<ConsensusValidationResult<ExecutionEvent<'a>>, Error> {
    // Validating structure
    let result = state_transition.validate_structure(
        platform.drive,
        platform.state.current_protocol_version_in_consensus(),
        transaction,
    )?;
    if !result.is_valid() {
        return Ok(ConsensusValidationResult::<ExecutionEvent>::new_with_errors(result.errors));
    }

    // Validating signatures
    let result = state_transition.validate_identity_and_signatures(
        platform.drive,
        platform.state.current_protocol_version_in_consensus(),
        transaction,
    )?;
    if !result.is_valid() {
        return Ok(ConsensusValidationResult::<ExecutionEvent>::new_with_errors(result.errors));
    }
    let maybe_identity = result.into_data()?;

    // Validating state
    let result = state_transition.validate_state(platform, transaction)?;

    result.map_result(|action| (maybe_identity, action, &platform.state.epoch()).try_into())
}

/// A trait for validating state transitions within a blockchain.
pub(crate) trait StateTransitionValidationV0: StateTransitionActionTransformerV0 {
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
        protocol_version: u32,
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
        protocol_version: u32,
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
}

impl StateTransitionValidationV0 for StateTransition {
    fn validate_structure(
        &self,
        drive: &Drive,
        protocol_version: u32,
        tx: TransactionArg,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match self {
            StateTransition::DataContractCreate(st) => {
                st.validate_structure(drive, protocol_version, tx)
            }
            StateTransition::DataContractUpdate(st) => {
                st.validate_structure(drive, protocol_version, tx)
            }
            StateTransition::IdentityCreate(st) => {
                st.validate_structure(drive, protocol_version, tx)
            }
            StateTransition::IdentityUpdate(st) => {
                st.validate_structure(drive, protocol_version, tx)
            }
            StateTransition::IdentityTopUp(st) => {
                st.validate_structure(drive, protocol_version, tx)
            }
            StateTransition::IdentityCreditWithdrawal(st) => {
                st.validate_structure(drive, protocol_version, tx)
            }
            StateTransition::DocumentsBatch(st) => {
                st.validate_structure(drive, protocol_version, tx)
            }
            StateTransition::IdentityCreditTransfer(st) => {
                st.validate_structure(drive, protocol_version, tx)
            }
        }
    }

    fn validate_identity_and_signatures(
        &self,
        drive: &Drive,
        protocol_version: u32,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<Option<PartialIdentity>>, Error> {
        match self {
            StateTransition::DataContractCreate(st) => {
                st.validate_identity_and_signatures(drive, protocol_version, tx)
            }
            StateTransition::DataContractUpdate(st) => {
                st.validate_identity_and_signatures(drive, protocol_version, tx)
            }
            StateTransition::IdentityCreate(st) => {
                st.validate_identity_and_signatures(drive, protocol_version, tx)
            }
            StateTransition::IdentityUpdate(st) => {
                st.validate_identity_and_signatures(drive, protocol_version, tx)
            }
            StateTransition::IdentityTopUp(st) => {
                st.validate_identity_and_signatures(drive, protocol_version, tx)
            }
            StateTransition::IdentityCreditWithdrawal(st) => {
                st.validate_identity_and_signatures(drive, protocol_version, tx)
            }
            StateTransition::DocumentsBatch(st) => {
                st.validate_identity_and_signatures(drive, protocol_version, tx)
            }
            StateTransition::IdentityCreditTransfer(st) => {
                st.validate_identity_and_signatures(drive, protocol_version, tx)
            }
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
            StateTransition::IdentityCreditTransfer(st) => st.validate_state(platform, tx),
        }
    }
}
