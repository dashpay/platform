use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use dpp::consensus::state::prefunded_specialized_balances::prefunded_specialized_balance_insufficient_error::PrefundedSpecializedBalanceInsufficientError;
use dpp::consensus::state::prefunded_specialized_balances::prefunded_specialized_balance_not_found_error::PrefundedSpecializedBalanceNotFoundError;
use dpp::fee::Credits;
use dpp::prefunded_specialized_balance::PrefundedSpecializedBalanceIdentifier;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::masternode_vote_transition::accessors::MasternodeVoteTransitionAccessorsV0;
use dpp::state_transition::masternode_vote_transition::MasternodeVoteTransition;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;
use std::collections::BTreeMap;

pub(super) trait MasternodeVoteTransitionBalanceValidationV1 {
    fn validate_advanced_minimum_balance_pre_check_v1(
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

impl MasternodeVoteTransitionBalanceValidationV1 for MasternodeVoteTransition {
    /// v1 (protocol version 14) requires the poll's fund to cover the single vote cost, which is
    /// what executing the vote deducts from it. v0 required only the vote's minimum fee, which
    /// is smaller, so a fund between the two passed the pre-check and the vote then failed
    /// inside execution.
    fn validate_advanced_minimum_balance_pre_check_v1(
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

        let single_vote_cost = platform_version
            .fee_version
            .vote_resolution_fund_fees
            .contested_document_single_vote_cost;

        if balance < single_vote_cost {
            return Ok(ConsensusValidationResult::new_with_error(
                PrefundedSpecializedBalanceInsufficientError::new(
                    balance_id,
                    balance,
                    single_vote_cost,
                )
                .into(),
            ));
        }

        Ok(ConsensusValidationResult::new_with_data(BTreeMap::from([
            (balance_id, balance),
        ])))
    }
}
