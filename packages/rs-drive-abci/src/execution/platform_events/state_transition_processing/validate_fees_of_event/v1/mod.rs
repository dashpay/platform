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
use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
use dpp::fee::fee_result::FeeResult;
use dpp::fee::Credits;
use dpp::prelude::ConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;

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

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
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
    /// Every failure leaves the state transition unpaid, like an insufficient balance: nobody
    /// was allowed to be charged. This stage runs in check tx with the last committed block time
    /// and at execution with the block's own time.
    ///
    /// Every event with neither signing key limits nor a gas sponsor is validated by v0.
    pub(super) fn validate_fees_of_event_v1(
        &self,
        event: &ExecutionEvent,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
        previous_fee_versions: &CachedEpochIndexFeeVersions,
    ) -> Result<ConsensusValidationResult<FeeResult>, Error> {
        let ExecutionEvent::Paid {
            identity,
            removed_balance,
            operations,
            execution_operations,
            additional_fixed_fee_cost,
            user_fee_increase,
            signing_key_limits,
            gas_sponsor,
            ..
        } = event
        else {
            return self.validate_fees_of_event_v0(
                event,
                block_info,
                transaction,
                platform_version,
                previous_fee_versions,
            );
        };

        if signing_key_limits.is_none() && gas_sponsor.is_none() {
            return self.validate_fees_of_event_v0(
                event,
                block_info,
                transaction,
                platform_version,
                previous_fee_versions,
            );
        }

        if let Some(signing_key_limits) = signing_key_limits {
            if let Some(expires_at) = signing_key_limits.expires_at {
                if block_info.time_ms >= expires_at {
                    return Ok(ConsensusValidationResult::new_with_data_and_errors(
                        FeeResult::default(),
                        vec![PublicKeyExpiredError::new(
                            signing_key_limits.key_id,
                            expires_at,
                            block_info.time_ms,
                        )
                        .into()],
                    ));
                }
            }
        }

        // From here to the balance check this is the v0 `Paid` arm, keeping hold of the
        // processing fee before the user fee increase.
        let balance =
            identity
                .balance
                .ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "partial identity info with no balance in paid execution event",
                )))?;
        let principal = removed_balance.unwrap_or_default();
        let mut estimated_fee_result = self
            .drive
            .apply_drive_operations(
                operations.clone(),
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

        let mut required_balance = estimated_fee_result.total_base_fee();

        if let Some(additional_fixed_fee_cost) = additional_fixed_fee_cost {
            required_balance = required_balance.saturating_add(*additional_fixed_fee_cost);
        }

        // The sponsor pays the gas when their balance covers it; the identity always funds the
        // principal. Execution asks `covers` the same question on the same estimate.
        let sponsor_pays = match gas_sponsor {
            Some(gas_sponsor) => {
                if balance < principal {
                    return Ok(ConsensusValidationResult::new_with_data_and_errors(
                        estimated_fee_result,
                        vec![StateError::IdentityInsufficientBalanceError(
                            IdentityInsufficientBalanceError::new(identity.id, balance, principal),
                        )
                        .into()],
                    ));
                }
                if gas_sponsor.covers(required_balance) {
                    true
                } else if gas_sponsor.strict {
                    return Ok(ConsensusValidationResult::new_with_data_and_errors(
                        estimated_fee_result,
                        vec![StateError::GasSponsorInsufficientBalanceError(
                            GasSponsorInsufficientBalanceError::new(
                                gas_sponsor.identity_id,
                                gas_sponsor.balance,
                                required_balance,
                            ),
                        )
                        .into()],
                    ));
                } else {
                    false
                }
            }
            None => false,
        };

        if !sponsor_pays {
            let balance_after_principal_operation = balance.saturating_sub(principal);
            if balance_after_principal_operation < required_balance {
                let total_required = required_balance.saturating_add(principal);
                return Ok(ConsensusValidationResult::new_with_data_and_errors(
                    estimated_fee_result,
                    vec![StateError::IdentityInsufficientBalanceError(
                        IdentityInsufficientBalanceError::new(identity.id, balance, total_required),
                    )
                    .into()],
                ));
            }
        }

        if let Some(signing_key_limits) = signing_key_limits {
            if let Some(remaining_budget) = signing_key_limits.remaining_budget {
                let required_budget = if sponsor_pays {
                    principal
                } else {
                    required_from_key_budget(
                        *removed_balance,
                        estimated_fee_result.storage_fee,
                        *additional_fixed_fee_cost,
                        user_fee_increase_amount,
                    )
                };
                // Signature validation already refuses a spent budget; the zero check keeps a
                // transition that requires nothing from slipping through on an empty one.
                if remaining_budget == 0 || required_budget > remaining_budget {
                    return Ok(ConsensusValidationResult::new_with_data_and_errors(
                        estimated_fee_result,
                        vec![StateError::IdentityPublicKeyBudgetExceededError(
                            IdentityPublicKeyBudgetExceededError::new(
                                identity.id,
                                signing_key_limits.key_id,
                                remaining_budget,
                                required_budget,
                            ),
                        )
                        .into()],
                    ));
                }
            }
        }

        Ok(ConsensusValidationResult::new_with_data(
            estimated_fee_result,
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
