use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use dpp::fee::Credits;
use dpp::prefunded_specialized_balance::PrefundedSpecializedBalanceIdentifier;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::StateTransition;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

pub(crate) trait StateTransitionPrefundedSpecializedBalanceValidationV0 {
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
    fn validate_minimum_prefunded_specialized_balance_pre_check(
        &self,
        drive: &Drive,
        tx: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<
        ConsensusValidationResult<BTreeMap<PrefundedSpecializedBalanceIdentifier, Credits>>,
        Error,
    >;

    /// Do we use a prefunded specialized balance for payment
    fn uses_prefunded_specialized_balance_for_payment(&self) -> bool {
        false
    }
}

impl StateTransitionPrefundedSpecializedBalanceValidationV0 for StateTransition {
    fn validate_minimum_prefunded_specialized_balance_pre_check(
        &self,
        drive: &Drive,
        tx: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<
        ConsensusValidationResult<BTreeMap<PrefundedSpecializedBalanceIdentifier, Credits>>,
        Error,
    > {
        match self {
            StateTransition::MasternodeVote(masternode_vote_transition) => {
                masternode_vote_transition.validate_minimum_prefunded_specialized_balance_pre_check(
                    drive,
                    tx,
                    execution_context,
                    platform_version,
                )
            }
            _ => Ok(ConsensusValidationResult::new()),
        }
    }

    fn uses_prefunded_specialized_balance_for_payment(&self) -> bool {
        matches!(self, StateTransition::MasternodeVote(_))
    }
}
