use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::platform_events::state_transition_processing::record_added_balance_outputs::AddedBalanceOutputsOrigin;
use crate::execution::platform_events::state_transition_processing::validate_fees_of_event::v1::SettledFees;
use crate::execution::types::execution_event::ExecutionEvent;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::signing_key_limits::SigningKeyLimits;
use crate::platform_types::event_execution_result::EventExecutionResult;
use crate::platform_types::event_execution_result::EventExecutionResult::{
    SuccessfulPaidExecution, UnpaidConsensusExecutionError, UnsuccessfulPaidExecution,
};
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::address_funds::PlatformAddress;
use dpp::balances::credits::CreditOperation;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::ConsensusError;
use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
use dpp::fee::fee_result::BalanceChange;
use dpp::fee::Credits;
use dpp::version::PlatformVersion;
use drive::drive::identity::update::apply_balance_change_outcome::ApplyBalanceChangeOutcomeV0Methods;
use drive::grovedb::Transaction;
use drive::state_transition_action::batch::{action_fee_operations, action_fees_total};
use drive::util::batch::DriveOperation;
use std::collections::BTreeMap;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// v1 executes a `Paid` event whose signing key carries a budget, whose batch names a gas
    /// sponsor, or whose batch owes document action fees, all recorded on the event from protocol
    /// version 14. Every other event, including one signed by a key that only expires, is
    /// executed by v0.
    ///
    /// Whoever pays the gas pays the document action fees: they move from the payer's balance to
    /// the contract's fee pots with the batch's own operations, and count against a budgeted key
    /// when its identity pays them. They are no part of the fee, which goes to the fee pools.
    ///
    /// The fee is charged to whoever fee validation settled on (`settle_fees_of_event_v1`, of the
    /// same generation): the gas sponsor when their balance covers the gas estimated with them
    /// paying, and the identity otherwise. Storage refunds still
    /// go to whoever paid the storage originally, so a sponsored document refunds its owner when
    /// it is deleted. A failed batch (`consensus_errors`) is never sponsored: its signer pays for
    /// the work that ran.
    ///
    /// A budgeted signing key is then charged with what the transition took from its identity:
    /// the balance it moved out (`removed_balance`) plus the fee the identity owes, net of the
    /// storage refunds the same transition returned to it, and nothing of a fee the sponsor paid.
    /// Fee validation let everything but the metered processing fee through only if it fit, so
    /// the deduction can take the budget past zero by that processing fee at most; it stops at
    /// zero, and a key at zero no longer signs. A failed state transition that is still paid for
    /// spends from the budget like a successful one. The deduction is applied outside of the fee,
    /// like the balance change, and never changes what is stored.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn execute_event_v1(
        &self,
        mut event: ExecutionEvent,
        mut consensus_errors: Vec<ConsensusError>,
        block_info: &BlockInfo,
        transaction: &Transaction,
        address_balances_in_update: Option<&mut BTreeMap<PlatformAddress, CreditOperation>>,
        block_credit_mints: &mut Credits,
        platform_version: &PlatformVersion,
        previous_fee_versions: &CachedEpochIndexFeeVersions,
    ) -> Result<EventExecutionResult, Error> {
        // The sponsor pays for a valid batch only; fee validation below must judge the same
        // event execution charges, so the sponsor leaves the event before either sees it.
        if let ExecutionEvent::Paid { gas_sponsor, .. } = &mut event {
            if !consensus_errors.is_empty() {
                *gas_sponsor = None;
            }
        }
        let (signed_by_budgeted_key, has_gas_sponsor, has_action_fees) = match &event {
            ExecutionEvent::Paid {
                signing_key_limits,
                gas_sponsor,
                action_fees,
                ..
            } => (
                matches!(
                    signing_key_limits,
                    Some(SigningKeyLimits {
                        remaining_budget: Some(_),
                        ..
                    })
                ),
                gas_sponsor.is_some(),
                !action_fees.is_empty(),
            ),
            _ => (false, false, false),
        };
        if !signed_by_budgeted_key && !has_gas_sponsor && !has_action_fees {
            return self.execute_event_v0(
                event,
                consensus_errors,
                block_info,
                transaction,
                address_balances_in_update,
                block_credit_mints,
                platform_version,
                previous_fee_versions,
            );
        }

        let mut fee_validation_result = self.settle_fees_of_event_v1(
            &event,
            block_info,
            Some(transaction),
            platform_version,
            previous_fee_versions,
        )?;

        let ExecutionEvent::Paid {
            identity,
            removed_balance,
            added_to_balance_outputs,
            operations,
            execution_operations,
            additional_fixed_fee_cost,
            user_fee_increase,
            signing_key_limits,
            gas_sponsor: _,
            action_fees,
        } = event
        else {
            return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                "the event was matched above as paid by an identity",
            )));
        };

        let result = if fee_validation_result.is_valid_with_data() {
            // Charge the payer fee validation settled on, so validation and execution never
            // name different payers.
            let SettledFees {
                estimated_fee_result,
                paying_sponsor,
            } = fee_validation_result.into_data()?;
            let payer_id = paying_sponsor
                .map(|gas_sponsor| gas_sponsor.identity_id)
                .unwrap_or(identity.id);

            // Whoever pays the gas pays the document action fees: they leave the payer's
            // balance for the contract's fee pots in the same batch as the documents, which
            // merges them with any other write of that balance (a purchase price, a voting
            // fund, a sale to a sponsoring contract owner). They are no part of the fee below,
            // which goes to the fee pools.
            //
            // They are a price the contract set, like the price of a purchase, and move as that
            // principal does: with the operations, before the gas is metered and debited. Fee
            // validation admitted the batch only with a balance covering them and the estimated
            // gas, so should the metered gas ever exceed its estimate, what falls short is the
            // gas, by the rule that already governs a principal, and never the pots against
            // credits that were not there.
            let mut operations = operations;
            operations.extend(action_fee_operations(payer_id, &action_fees)?);
            let action_fees_owed_by_identity = if paying_sponsor.is_some() {
                0
            } else {
                action_fees_total(&identity.id, &action_fees)?
            };

            let credit_mints = DriveOperation::credit_mints(&operations);
            let mut individual_fee_result = self
                .drive
                .apply_drive_operations(
                    operations,
                    true,
                    block_info,
                    Some(transaction),
                    platform_version,
                    Some(previous_fee_versions),
                )
                .map_err(Error::Drive)?;

            *block_credit_mints = block_credit_mints.saturating_add(credit_mints);

            ValidationOperation::add_many_to_fee_result(
                &execution_operations,
                &mut individual_fee_result,
                platform_version,
            )?;

            individual_fee_result.apply_user_fee_increase(user_fee_increase);

            if let Some(additional_fixed_fee_cost) = additional_fixed_fee_cost {
                individual_fee_result.processing_fee = individual_fee_result
                    .processing_fee
                    .saturating_add(additional_fixed_fee_cost);
            }

            let balance_change = individual_fee_result.into_balance_change(payer_id);

            let fee_owed_by_identity = if paying_sponsor.is_some() {
                0
            } else {
                match balance_change.change() {
                    BalanceChange::RemoveFromBalance {
                        desired_removed_balance,
                        ..
                    } => *desired_removed_balance,
                    BalanceChange::AddToBalance(_) | BalanceChange::NoBalanceChange => 0,
                }
            };
            let spent_from_key_budget = removed_balance
                .unwrap_or_default()
                .saturating_add(action_fees_owed_by_identity)
                .saturating_add(fee_owed_by_identity);

            let outcome = self.drive.apply_balance_change_from_fee_to_identity(
                balance_change,
                block_info,
                Some(transaction),
                platform_version,
            )?;

            if let Some(signing_key_limits) =
                signing_key_limits.filter(|limits| limits.remaining_budget.is_some())
            {
                self.drive.deduct_from_identity_key_budget(
                    identity.id.to_buffer(),
                    signing_key_limits.key_id,
                    spent_from_key_budget,
                    Some(transaction),
                    platform_version,
                )?;
            }

            // Only an executed event moved anything. No transition a limited key may sign has
            // address outputs today, so this records nothing; it must not start to for an event
            // that was refused.
            self.record_added_balance_outputs(
                address_balances_in_update,
                added_to_balance_outputs,
                AddedBalanceOutputsOrigin::Transparent,
                platform_version,
            )?;

            if consensus_errors.is_empty() {
                SuccessfulPaidExecution(Some(estimated_fee_result), outcome.actual_fee_paid_owned())
            } else {
                UnsuccessfulPaidExecution(
                    Some(estimated_fee_result),
                    outcome.actual_fee_paid_owned(),
                    consensus_errors,
                )
            }
        } else {
            consensus_errors.append(&mut fee_validation_result.errors);
            UnpaidConsensusExecutionError(consensus_errors)
        };

        Ok(result)
    }
}
