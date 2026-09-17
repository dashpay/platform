use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::platform_events::state_transition_processing::record_added_balance_outputs::AddedBalanceOutputsOrigin;
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
use drive::util::batch::DriveOperation;
use std::collections::BTreeMap;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// v1 keeps the budget of the key that signed a state transition up to date.
    ///
    /// An event paid by an identity whose signing key is budgeted is executed like the v0 `Paid`
    /// event, and then what the transition took from the identity is deducted from what is left
    /// of the key's budget: the balance it moved out (`removed_balance`) plus the fee the identity
    /// owes, net of the storage refunds the same transition returned to it. Fee validation let
    /// everything but the metered processing fee through only if it fit, so the deduction can
    /// take the budget past zero by that processing fee at most; it stops at zero, and a key at
    /// zero no longer signs. A failed state transition that is still paid for spends from the
    /// budget like a successful one. The deduction is applied outside of the fee, like the
    /// balance change, and never changes what is stored.
    ///
    /// Every other event, including one signed by a key that only expires, is executed by v0.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn execute_event_v1(
        &self,
        event: ExecutionEvent,
        mut consensus_errors: Vec<ConsensusError>,
        block_info: &BlockInfo,
        transaction: &Transaction,
        address_balances_in_update: Option<&mut BTreeMap<PlatformAddress, CreditOperation>>,
        block_credit_mints: &mut Credits,
        platform_version: &PlatformVersion,
        previous_fee_versions: &CachedEpochIndexFeeVersions,
    ) -> Result<EventExecutionResult, Error> {
        let signed_by_budgeted_key = matches!(
            &event,
            ExecutionEvent::Paid {
                signing_key_limits: Some(SigningKeyLimits {
                    remaining_budget: Some(_),
                    ..
                }),
                ..
            }
        );
        if !signed_by_budgeted_key {
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

        let mut fee_validation_result = self.validate_fees_of_event(
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
            signing_key_limits: Some(signing_key_limits),
        } = event
        else {
            return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                "the event was matched above as paid and signed by a budgeted key",
            )));
        };

        let result = if fee_validation_result.is_valid_with_data() {
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

            let balance_change = individual_fee_result.into_balance_change(identity.id);

            let fee_owed_by_identity = match balance_change.change() {
                BalanceChange::RemoveFromBalance {
                    desired_removed_balance,
                    ..
                } => *desired_removed_balance,
                BalanceChange::AddToBalance(_) | BalanceChange::NoBalanceChange => 0,
            };
            let spent_from_key_budget = removed_balance
                .unwrap_or_default()
                .saturating_add(fee_owed_by_identity);

            let outcome = self.drive.apply_balance_change_from_fee_to_identity(
                balance_change,
                Some(transaction),
                platform_version,
            )?;

            self.drive.deduct_from_identity_key_budget(
                identity.id.to_buffer(),
                signing_key_limits.key_id,
                spent_from_key_budget,
                Some(transaction),
                platform_version,
            )?;

            if consensus_errors.is_empty() {
                SuccessfulPaidExecution(
                    Some(fee_validation_result.into_data()?),
                    outcome.actual_fee_paid_owned(),
                )
            } else {
                UnsuccessfulPaidExecution(
                    Some(fee_validation_result.into_data()?),
                    outcome.actual_fee_paid_owned(),
                    consensus_errors,
                )
            }
        } else {
            consensus_errors.append(&mut fee_validation_result.errors);
            UnpaidConsensusExecutionError(consensus_errors)
        };

        self.record_added_balance_outputs(
            address_balances_in_update,
            added_to_balance_outputs,
            AddedBalanceOutputsOrigin::Transparent,
            platform_version,
        )?;

        Ok(result)
    }
}
