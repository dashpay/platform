use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::execution_event::ExecutionEvent;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::signature::PublicKeyExpiredError;
use dpp::consensus::state::identity::gas_sponsor_insufficient_balance_error::GasSponsorInsufficientBalanceError;
use dpp::consensus::state::identity::identity_public_key_budget_exceeded_error::IdentityPublicKeyBudgetExceededError;
use dpp::consensus::state::identity::IdentityInsufficientBalanceError;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::ConsensusError;
use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
use dpp::fee::fee_result::FeeResult;
use dpp::fee::Credits;
use dpp::platform_value::Identifier;
use dpp::prelude::ConsensusValidationResult;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::batch::{
    action_fee_operations, action_fees_total, ResolvedDocumentActionFee, ResolvedGasSponsor,
};

/// What a state transition needs to find in the budget of the key that signed it.
///
/// A budget caps what a key can take from its identity, with one allowance: the metered
/// processing fee may take it over, because that fee is only known once the transition has run
/// and is bounded by what one transition can do. Everything else must fit in what is left:
///
/// * `removed_balance`: credits the transition moves out of the identity (a document purchase,
///   a prefunded voting balance);
/// * the storage fee;
/// * `additional_fixed_fee_cost`: fees priced up front, such as a contract registration;
/// * `user_fee_increase_amount`: the part of the processing fee the signer chose to add.
pub(in crate::execution::platform_events::state_transition_processing) fn required_from_key_budget(
    removed_balance: Option<Credits>,
    storage_fee: Credits,
    additional_fixed_fee_cost: Option<Credits>,
    user_fee_increase_amount: Credits,
) -> Credits {
    removed_balance
        .unwrap_or_default()
        .saturating_add(storage_fee)
        .saturating_add(additional_fixed_fee_cost.unwrap_or_default())
        .saturating_add(user_fee_increase_amount)
}

/// Whether the gas sponsor pays: their balance covers the gas estimated with them paying and the
/// document action fees they owe. Whoever pays the gas pays the action fees, so this is the one
/// question; execution charges whoever fee validation settled on by it ([`SettledFees`]).
fn gas_sponsor_pays(
    gas_sponsor: &ResolvedGasSponsor,
    required_gas: Credits,
    action_fees: &[ResolvedDocumentActionFee],
) -> Result<bool, ProtocolError> {
    Ok(gas_sponsor.covers(required_from_gas_sponsor(
        gas_sponsor,
        required_gas,
        action_fees,
    )?))
}

/// What the gas sponsor's balance has to cover: the estimated gas and the document action fees
/// they would owe.
fn required_from_gas_sponsor(
    gas_sponsor: &ResolvedGasSponsor,
    required_gas: Credits,
    action_fees: &[ResolvedDocumentActionFee],
) -> Result<Credits, ProtocolError> {
    Ok(required_gas.saturating_add(action_fees_total(&gas_sponsor.identity_id, action_fees)?))
}

/// What fee validation settled on for an event: the gas estimated for the payer, and the gas
/// sponsor who pays it and the document action fees, if one does. Execution charges this payer
/// rather than asking again, so fee validation and execution never name different payers.
#[derive(Debug, Clone)]
pub(in crate::execution::platform_events::state_transition_processing) struct SettledFees {
    /// The gas estimated for the payer
    pub estimated_fee_result: FeeResult,
    /// The gas sponsor who pays; `None` when the identity pays
    pub paying_sponsor: Option<ResolvedGasSponsor>,
}

impl SettledFees {
    fn paid_by_the_identity(estimated_fee_result: FeeResult) -> Self {
        SettledFees {
            estimated_fee_result,
            paying_sponsor: None,
        }
    }
}

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Fee validation v1, as [`Platform::settle_fees_of_event_v1`] settles it, with the payer
    /// left out.
    pub(super) fn validate_fees_of_event_v1(
        &self,
        event: &ExecutionEvent,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
        previous_fee_versions: &CachedEpochIndexFeeVersions,
    ) -> Result<ConsensusValidationResult<FeeResult>, Error> {
        Ok(self
            .settle_fees_of_event_v1(
                event,
                block_info,
                transaction,
                platform_version,
                previous_fee_versions,
            )?
            .map(|settled_fees| settled_fees.estimated_fee_result))
    }

    /// v1 enforces, on top of the v0 balance check, what identity signature validation and the
    /// batch transformer recorded on the event from protocol version 14:
    ///
    /// * the usage limits of the key that signed the state transition: a key whose expiry is at
    ///   or before the block time can no longer sign, and what the transition requires from the
    ///   key's budget (see [`required_from_key_budget`]) must fit in what is left of it;
    /// * the contract owner a document batch asks to pay its gas: the fee is judged against the
    ///   sponsor's balance, and the identity only has to fund `removed_balance`. A sponsor whose
    ///   balance falls short refuses a batch that insists on them (`GasFeesPaidBy::ContractOwner`)
    ///   unpaid, and hands a batch that merely prefers them back to the identity's balance. When
    ///   the sponsor pays, the key's budget only has to cover `removed_balance`.
    ///
    /// * the document action fees a batch owes (the `actionFees` keyword): whoever pays the gas
    ///   pays them. The gas is estimated for the payer, as execution applies the batch: first
    ///   with the sponsor paying, whose balance has to cover that gas and the fees the sponsor
    ///   owes; failing that with the identity paying, whose balance has to cover that gas, its
    ///   own principal and the fees the identity owes, which also count against a budgeted
    ///   key. The identity's principal and fees are refused before anything is estimated when
    ///   its balance cannot fund them.
    ///
    /// Every failure leaves the state transition unpaid, like an insufficient balance: nobody
    /// was allowed to be charged. This stage runs in check tx with the last committed block time
    /// and at execution with the block's own time.
    ///
    /// Every event with no signing key limits, no gas sponsor and no action fees is validated
    /// by v0, and paid by its identity.
    ///
    /// It returns the payer it settles on with the estimate: `execute_event` 1, of the same
    /// generation (both are selected by `DRIVE_ABCI_METHOD_VERSIONS_V10`), charges that payer.
    pub(in crate::execution::platform_events::state_transition_processing) fn settle_fees_of_event_v1(
        &self,
        event: &ExecutionEvent,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
        previous_fee_versions: &CachedEpochIndexFeeVersions,
    ) -> Result<ConsensusValidationResult<SettledFees>, Error> {
        let ExecutionEvent::Paid {
            identity,
            removed_balance,
            operations,
            execution_operations,
            additional_fixed_fee_cost,
            user_fee_increase,
            signing_key_limits,
            gas_sponsor,
            action_fees,
            ..
        } = event
        else {
            return Ok(self
                .validate_fees_of_event_v0(
                    event,
                    block_info,
                    transaction,
                    platform_version,
                    previous_fee_versions,
                )?
                .map(SettledFees::paid_by_the_identity));
        };

        // A refusal: nobody is charged, so it names no sponsor.
        let refused = |estimated_fee_result: FeeResult, error: ConsensusError| {
            ConsensusValidationResult::new_with_data_and_errors(
                SettledFees::paid_by_the_identity(estimated_fee_result),
                vec![error],
            )
        };

        if signing_key_limits.is_none() && gas_sponsor.is_none() && action_fees.is_empty() {
            return Ok(self
                .validate_fees_of_event_v0(
                    event,
                    block_info,
                    transaction,
                    platform_version,
                    previous_fee_versions,
                )?
                .map(SettledFees::paid_by_the_identity));
        }

        if let Some(signing_key_limits) = signing_key_limits {
            if let Some(expires_at) = signing_key_limits.expires_at {
                if block_info.time_ms >= expires_at {
                    return Ok(refused(
                        FeeResult::default(),
                        PublicKeyExpiredError::new(
                            signing_key_limits.key_id,
                            expires_at,
                            block_info.time_ms,
                        )
                        .into(),
                    ));
                }
            }
        }

        let balance =
            identity
                .balance
                .ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "partial identity info with no balance in paid execution event",
                )))?;
        let principal = removed_balance.unwrap_or_default();
        let identity_action_fees = action_fees_total(&identity.id, action_fees)?;

        // What the batch is estimated to cost when `payer_id` pays its gas, and so its action
        // fees, charged by the operations execution appends and merged with the batch's own as
        // execution applies them; and the part of the processing fee the user fee increase adds.
        let estimate_paid_by = |payer_id: Identifier| -> Result<(FeeResult, Credits), Error> {
            let mut operations = operations.clone();
            operations.extend(action_fee_operations(payer_id, action_fees)?);
            let mut estimated_fee_result = self
                .drive
                .apply_drive_operations(
                    operations,
                    false,
                    block_info,
                    transaction,
                    platform_version,
                    Some(previous_fee_versions),
                )
                .map_err(Error::Drive)?;
            ValidationOperation::add_many_to_fee_result(
                execution_operations,
                &mut estimated_fee_result,
                platform_version,
            )?;
            let base_processing_fee = estimated_fee_result.processing_fee;
            estimated_fee_result.apply_user_fee_increase(*user_fee_increase);
            let user_fee_increase_amount = estimated_fee_result
                .processing_fee
                .saturating_sub(base_processing_fee);
            Ok((estimated_fee_result, user_fee_increase_amount))
        };
        let required_gas = |estimated_fee_result: &FeeResult| {
            estimated_fee_result
                .total_base_fee()
                .saturating_add(additional_fixed_fee_cost.unwrap_or_default())
        };
        let insufficient_balance = |estimated_fee_result: FeeResult, required: Credits| {
            refused(
                estimated_fee_result,
                StateError::IdentityInsufficientBalanceError(
                    IdentityInsufficientBalanceError::new(identity.id, balance, required),
                )
                .into(),
            )
        };
        // The refusal of a budgeted signing key with less than `required_budget` left.
        // Signature validation already refuses a spent budget; the zero check keeps a
        // transition that requires nothing from slipping through on an empty one.
        let key_budget_exceeded = |required_budget: Credits| -> Option<ConsensusError> {
            let signing_key_limits = signing_key_limits.as_ref()?;
            let remaining_budget = signing_key_limits.remaining_budget?;
            (remaining_budget == 0 || required_budget > remaining_budget).then(|| {
                StateError::IdentityPublicKeyBudgetExceededError(
                    IdentityPublicKeyBudgetExceededError::new(
                        identity.id,
                        signing_key_limits.key_id,
                        remaining_budget,
                        required_budget,
                    ),
                )
                .into()
            })
        };

        // The identity funds the principal itself, whoever pays the gas.
        if balance < principal {
            return Ok(insufficient_balance(FeeResult::default(), principal));
        }

        // The sponsor is judged on the batch as it runs when they pay: estimated with the
        // action fees they owe, never an owner part, since a sponsor is the contract owner.
        if let Some(gas_sponsor) = gas_sponsor {
            let (estimated_fee_result, _) = estimate_paid_by(gas_sponsor.identity_id)?;
            let required_balance = required_gas(&estimated_fee_result);
            if gas_sponsor_pays(gas_sponsor, required_balance, action_fees)? {
                // The key's budget only has to cover the principal.
                return Ok(match key_budget_exceeded(principal) {
                    Some(error) => refused(estimated_fee_result, error),
                    None => ConsensusValidationResult::new_with_data(SettledFees {
                        estimated_fee_result,
                        paying_sponsor: Some(*gas_sponsor),
                    }),
                });
            }
            if gas_sponsor.strict {
                return Ok(refused(
                    estimated_fee_result,
                    StateError::GasSponsorInsufficientBalanceError(
                        GasSponsorInsufficientBalanceError::new(
                            gas_sponsor.identity_id,
                            gas_sponsor.balance,
                            required_from_gas_sponsor(gas_sponsor, required_balance, action_fees)?,
                        ),
                    )
                    .into(),
                ));
            }
        }

        // The identity pays the gas, and with it the action fees it owes. Those are known
        // without an estimate and refused first, so the removal the estimate merges for the
        // identity never takes more than its balance, the most an estimate lets it take.
        let balance_after_principal_operation = balance - principal;
        if balance_after_principal_operation < identity_action_fees {
            return Ok(insufficient_balance(
                FeeResult::default(),
                principal.saturating_add(identity_action_fees),
            ));
        }
        let (estimated_fee_result, user_fee_increase_amount) = estimate_paid_by(identity.id)?;
        let required_balance =
            required_gas(&estimated_fee_result).saturating_add(identity_action_fees);
        if balance_after_principal_operation < required_balance {
            return Ok(insufficient_balance(
                estimated_fee_result,
                required_balance.saturating_add(principal),
            ));
        }
        // The action fees leave the identity like a principal does.
        if let Some(error) = key_budget_exceeded(required_from_key_budget(
            Some(principal.saturating_add(identity_action_fees)),
            estimated_fee_result.storage_fee,
            *additional_fixed_fee_cost,
            user_fee_increase_amount,
        )) {
            return Ok(refused(estimated_fee_result, error));
        }

        Ok(ConsensusValidationResult::new_with_data(
            SettledFees::paid_by_the_identity(estimated_fee_result),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_require_everything_but_metered_processing_from_the_key_budget() {
        assert_eq!(required_from_key_budget(None, 0, None, 0), 0);
        assert_eq!(
            required_from_key_budget(Some(5), 70, Some(300), 1_000),
            1_375
        );
        assert_eq!(required_from_key_budget(None, 70, None, 0), 70);
    }

    #[test]
    fn should_saturate_instead_of_wrapping() {
        assert_eq!(
            required_from_key_budget(Some(u64::MAX), 1, Some(1), 1),
            u64::MAX
        );
    }
}
