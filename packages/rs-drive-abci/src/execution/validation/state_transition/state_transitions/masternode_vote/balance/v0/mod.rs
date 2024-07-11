use std::collections::BTreeMap;
use dpp::consensus::state::prefunded_specialized_balances::prefunded_specialized_balance_insufficient_error::PrefundedSpecializedBalanceInsufficientError;
use dpp::consensus::state::prefunded_specialized_balances::prefunded_specialized_balance_not_found_error::PrefundedSpecializedBalanceNotFoundError;
use crate::error::Error;
use dpp::fee::Credits;
use dpp::prefunded_specialized_balance::PrefundedSpecializedBalanceIdentifier;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::masternode_vote_transition::accessors::MasternodeVoteTransitionAccessorsV0;
use dpp::state_transition::masternode_vote_transition::MasternodeVoteTransition;

use crate::error::execution::ExecutionError;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;

pub(super) trait MasternodeVoteTransitionBalanceValidationV0 {
    fn validate_advanced_minimum_balance_pre_check_v0(
        &self,
        drive: &Drive,
        tx: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<
        ConsensusValidationResult<BTreeMap<PrefundedSpecializedBalanceIdentifier, Credits>>,
        Error,
    >;
}

impl MasternodeVoteTransitionBalanceValidationV0 for MasternodeVoteTransition {
    fn validate_advanced_minimum_balance_pre_check_v0(
        &self,
        drive: &Drive,
        tx: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<
        ConsensusValidationResult<BTreeMap<PrefundedSpecializedBalanceIdentifier, Credits>>,
        Error,
    > {
        execution_context.add_operation(ValidationOperation::RetrievePrefundedSpecializedBalance);

        let vote = self.vote();

        let balance_id = vote.specialized_balance_id()?.ok_or(Error::Execution(
            ExecutionError::CorruptedCodeExecution(
                "In this version there should always be a specialized balance id",
            ),
        ))?;
        let maybe_balance = drive.fetch_prefunded_specialized_balance(
            balance_id.to_buffer(),
            tx,
            platform_version,
        )?;

        let Some(balance) = maybe_balance else {
            // If there is no balance we are voting on something that either was never created or has finished
            return Ok(ConsensusValidationResult::new_with_error(
                PrefundedSpecializedBalanceNotFoundError::new(balance_id).into(),
            ));
        };
        if balance
            < platform_version
                .fee_version
                .state_transition_min_fees
                .masternode_vote
        {
            return Ok(ConsensusValidationResult::new_with_error(
                PrefundedSpecializedBalanceInsufficientError::new(
                    balance_id,
                    balance,
                    platform_version
                        .fee_version
                        .state_transition_min_fees
                        .masternode_vote,
                )
                .into(),
            ));
        }

        Ok(ConsensusValidationResult::new_with_data(BTreeMap::from([
            (balance_id, balance),
        ])))
    }
}
