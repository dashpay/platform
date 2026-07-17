use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::execution_event::ExecutionEvent;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::address_funds::fee_strategy::deduct_fee_from_inputs_and_outputs::deduct_fee_from_outputs_or_remaining_balance_of_inputs;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::state::address_funds::AddressesNotEnoughFundsError;
use dpp::consensus::state::identity::IdentityInsufficientBalanceError;
use dpp::consensus::state::shielded::invalid_shielded_proof_error::InvalidShieldedProofError;
use dpp::consensus::state::state_error::StateError;
use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
use dpp::fee::fee_result::FeeResult;

use dpp::prelude::ConsensusValidationResult;
use dpp::version::PlatformVersion;

use drive::grovedb::TransactionArg;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Validates the fees of a given `ExecutionEvent`.
    ///
    /// # Arguments
    ///
    /// * `event` - The `ExecutionEvent` instance to validate.
    /// * `block_info` - Information about the current block.
    /// * `transaction` - The transaction arguments for the given event.
    ///
    /// # Returns
    ///
    /// * `Result<ConsensusValidationResult<FeeResult>, Error>` - On success, returns a
    ///   `ConsensusValidationResult` containing an `FeeResult`. On error, returns an `Error`.
    ///
    /// # Errors
    ///
    /// * This function may return an `Error::Execution` if the identity balance is not found.
    /// * This function may return an `Error::Drive` if there's an issue with applying drive operations.
    pub(super) fn validate_fees_of_event_v0(
        &self,
        event: &ExecutionEvent,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
        previous_fee_versions: &CachedEpochIndexFeeVersions,
    ) -> Result<ConsensusValidationResult<FeeResult>, Error> {
        match event {
            ExecutionEvent::PaidFromAssetLock {
                identity,
                added_balance,
                operations,
                execution_operations,
                user_fee_increase,
            } => {
                let previous_balance = identity.balance.ok_or(Error::Execution(
                    ExecutionError::CorruptedCodeExecution("partial identity info with no balance in paid from asset lock execution event"),
                ))?;
                let previous_balance_with_top_up = previous_balance.saturating_add(*added_balance);
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

                estimated_fee_result.apply_user_fee_increase(*user_fee_increase);

                // TODO: Should take into account refunds as well
                let total_fee = estimated_fee_result.total_base_fee();
                if previous_balance_with_top_up >= total_fee {
                    Ok(ConsensusValidationResult::new_with_data(
                        estimated_fee_result,
                    ))
                } else {
                    Ok(ConsensusValidationResult::new_with_data_and_errors(
                        estimated_fee_result,
                        vec![StateError::IdentityInsufficientBalanceError(
                            IdentityInsufficientBalanceError::new(
                                identity.id,
                                previous_balance_with_top_up,
                                total_fee,
                            ),
                        )
                        .into()],
                    ))
                }
            }
            ExecutionEvent::Paid {
                identity,
                removed_balance,
                operations,
                execution_operations,
                additional_fixed_fee_cost: additional_fee_cost,
                user_fee_increase,
                ..
            } => {
                let balance = identity.balance.ok_or(Error::Execution(
                    ExecutionError::CorruptedCodeExecution(
                        "partial identity info with no balance in paid execution event",
                    ),
                ))?;
                let balance_after_principal_operation =
                    balance.saturating_sub(removed_balance.unwrap_or_default());
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

                estimated_fee_result.apply_user_fee_increase(*user_fee_increase);

                // TODO: Should take into account refunds as well
                let mut required_balance = estimated_fee_result.total_base_fee();

                if let Some(additional_fee_cost) = additional_fee_cost {
                    required_balance = required_balance.saturating_add(*additional_fee_cost);
                }

                if balance_after_principal_operation >= required_balance {
                    Ok(ConsensusValidationResult::new_with_data(
                        estimated_fee_result,
                    ))
                } else {
                    let total_required =
                        required_balance.saturating_add(removed_balance.unwrap_or_default());
                    Ok(ConsensusValidationResult::new_with_data_and_errors(
                        estimated_fee_result,
                        vec![StateError::IdentityInsufficientBalanceError(
                            IdentityInsufficientBalanceError::new(
                                identity.id,
                                balance,
                                total_required,
                            ),
                        )
                        .into()],
                    ))
                }
            }
            ExecutionEvent::PaidFromAddressInputs {
                input_current_balances,
                added_to_balance_outputs,
                fee_strategy,
                operations,
                execution_operations,
                additional_fixed_fee_cost,
                user_fee_increase,
            } => {
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

                estimated_fee_result.apply_user_fee_increase(*user_fee_increase);

                // Advertise the fee that block execution will actually charge: the metered base fee
                // PLUS any `additional_fixed_fee_cost`. For the transparent `Shield` that fixed cost is
                // the ZK compute fee `compute_shielded_verification_fee` (added on top of the metered
                // storage/processing of the note/nullifier writes); for `IdentityCreateFromAddresses`
                // it is the registration cost. Execution folds this fixed cost into `processing_fee`
                // (see `paid_from_address_inputs_and_outputs`), so we fold it into the advertised
                // `FeeResult` here too — otherwise CheckTx `gas_wanted` would under-report the fee
                // relative to what is booked. The required funding balance is exactly this fee
                // (storage + processing), unchanged by the fold.
                let mut result_fee_result = estimated_fee_result;
                if let Some(additional_fixed_fee_cost) = additional_fixed_fee_cost {
                    result_fee_result.processing_fee = result_fee_result
                        .processing_fee
                        .saturating_add(*additional_fixed_fee_cost);
                }
                let required_balance = result_fee_result.total_base_fee();

                let fee_deduction_result = deduct_fee_from_outputs_or_remaining_balance_of_inputs(
                    input_current_balances.clone(),
                    added_to_balance_outputs.clone(),
                    fee_strategy,
                    required_balance,
                    platform_version,
                )?;

                if fee_deduction_result.fee_fully_covered {
                    Ok(ConsensusValidationResult::new_with_data(result_fee_result))
                } else {
                    Ok(ConsensusValidationResult::new_with_data_and_errors(
                        result_fee_result,
                        vec![StateError::AddressesNotEnoughFundsError(
                            AddressesNotEnoughFundsError::new(
                                input_current_balances.clone(),
                                required_balance,
                            ),
                        )
                        .into()],
                    ))
                }
            }
            ExecutionEvent::PaidFromAssetLockToPool {
                fees_to_add_to_pool,
                operations,
                execution_operations,
                ..
            } => {
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

                let required_fee = estimated_fee_result.total_base_fee();
                // Advertise the authoritative pool fee for `gas_wanted`, not the metered estimate
                // (the executor books `fees_to_add_to_pool` regardless).
                let storage_fee = estimated_fee_result.storage_fee.min(*fees_to_add_to_pool);
                let authoritative_fee_result = FeeResult {
                    storage_fee,
                    processing_fee: *fees_to_add_to_pool - storage_fee,
                    fee_refunds: Default::default(),
                    removed_bytes_from_system: 0,
                };
                if *fees_to_add_to_pool >= required_fee {
                    Ok(ConsensusValidationResult::new_with_data(
                        authoritative_fee_result,
                    ))
                } else {
                    Ok(ConsensusValidationResult::new_with_data_and_errors(
                        authoritative_fee_result,
                        vec![StateError::InvalidShieldedProofError(
                            InvalidShieldedProofError::new(format!(
                                "shield_from_asset_lock fee insufficient: provided {} but minimum required {}",
                                fees_to_add_to_pool, required_fee
                            )),
                        )
                        .into()],
                    ))
                }
            }
            ExecutionEvent::PaidFromShieldedPoolToNewIdentity {
                identity,
                operations,
                execution_operations,
                denomination,
                additional_fixed_fee_cost,
            } => {
                // Affordability gate mirroring `PaidFromAssetLock`: the new identity is created
                // holding `denomination`, and the metered fee + the flat compute fee must not exceed
                // it (otherwise the identity would be credited <= 0). Estimate the metered cost
                // (apply = false), fold the compute fee into processing for gas-wanted parity, and
                // reject with `IdentityInsufficientBalanceError` if `denomination < total_fee`.
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

                if let Some(additional_fixed_fee_cost) = additional_fixed_fee_cost {
                    estimated_fee_result.processing_fee = estimated_fee_result
                        .processing_fee
                        .saturating_add(*additional_fixed_fee_cost);
                }

                let total_fee = estimated_fee_result.total_base_fee();
                if *denomination >= total_fee {
                    Ok(ConsensusValidationResult::new_with_data(
                        estimated_fee_result,
                    ))
                } else {
                    Ok(ConsensusValidationResult::new_with_data_and_errors(
                        estimated_fee_result,
                        vec![StateError::IdentityInsufficientBalanceError(
                            IdentityInsufficientBalanceError::new(
                                identity.id,
                                *denomination,
                                total_fee,
                            ),
                        )
                        .into()],
                    ))
                }
            }
            ExecutionEvent::PaidFixedCost { .. }
            | ExecutionEvent::PaidFromShieldedPool { .. }
            | ExecutionEvent::Free { .. }
            | ExecutionEvent::PaidFromAssetLockWithoutIdentity { .. } => Ok(
                ConsensusValidationResult::new_with_data(FeeResult::default()),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::types::execution_event::ExecutionEvent;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::fee::fee_result::FeeResult;
    use dpp::identity::PartialIdentity;
    use dpp::prelude::Identifier;
    use std::collections::{BTreeMap, BTreeSet};

    fn build_partial_identity_with_balance(balance: Option<u64>) -> PartialIdentity {
        PartialIdentity {
            id: Identifier::new([1u8; 32]),
            loaded_public_keys: BTreeMap::new(),
            balance,
            revision: None,
            not_found_public_keys: BTreeSet::new(),
        }
    }

    /// Exercises the `CorruptedCodeExecution` error branch for
    /// `ExecutionEvent::Paid` when `identity.balance` is None. This code path
    /// is protective and should never actually trigger, which is exactly why
    /// it's covered by a unit test rather than an integration test.
    #[test]
    fn validate_fees_of_event_v0_paid_no_balance_returns_corrupted_code_error() {
        let platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let platform_version = PlatformVersion::latest();
        let block_info = dpp::block::block_info::BlockInfo::default();

        let event = ExecutionEvent::Paid {
            identity: build_partial_identity_with_balance(None),
            removed_balance: None,
            added_to_balance_outputs: None,
            operations: vec![],
            execution_operations: vec![],
            additional_fixed_fee_cost: None,
            user_fee_increase: 0,
        };

        let previous_fee_versions = Default::default();
        let err = platform
            .platform
            .validate_fees_of_event_v0(
                &event,
                &block_info,
                None,
                platform_version,
                &previous_fee_versions,
            )
            .expect_err("missing balance should yield a CorruptedCodeExecution error");

        let msg = err.to_string();
        assert!(
            msg.contains("partial identity info with no balance")
                || msg.contains("paid execution event"),
            "expected CorruptedCodeExecution about missing balance, got: {}",
            msg
        );
    }

    /// Exercises the `CorruptedCodeExecution` error branch for
    /// `ExecutionEvent::PaidFromAssetLock` when `identity.balance` is None.
    #[test]
    fn validate_fees_of_event_v0_asset_lock_no_balance_returns_corrupted_code_error() {
        let platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let platform_version = PlatformVersion::latest();
        let block_info = dpp::block::block_info::BlockInfo::default();

        let event = ExecutionEvent::PaidFromAssetLock {
            identity: build_partial_identity_with_balance(None),
            added_balance: 0,
            operations: vec![],
            execution_operations: vec![],
            user_fee_increase: 0,
        };

        let previous_fee_versions = Default::default();
        let err = platform
            .platform
            .validate_fees_of_event_v0(
                &event,
                &block_info,
                None,
                platform_version,
                &previous_fee_versions,
            )
            .expect_err("missing balance should yield a CorruptedCodeExecution error");

        let msg = err.to_string();
        assert!(
            msg.contains("partial identity info")
                || msg.contains("paid from asset lock execution event"),
            "expected CorruptedCodeExecution about missing balance, got: {}",
            msg
        );
    }

    /// Covers the short-circuit arms for events with no per-event fee validation:
    /// `PaidFixedCost`, `PaidFromShieldedPool`, `Free`, and
    /// `PaidFromAssetLockWithoutIdentity`. All four must return a
    /// `ConsensusValidationResult` with a default `FeeResult` and no errors.
    #[test]
    fn validate_fees_of_event_v0_no_fee_events_return_default_fee_result() {
        let platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_initial_state_structure();
        let platform_version = PlatformVersion::latest();
        let block_info = dpp::block::block_info::BlockInfo::default();
        let previous_fee_versions = Default::default();

        let events = vec![
            ExecutionEvent::PaidFixedCost {
                operations: vec![],
                fees_to_add_to_pool: 0,
            },
            ExecutionEvent::PaidFromShieldedPool {
                operations: vec![],
                fees_to_add_to_pool: 0,
                added_to_balance_outputs: None,
                chargeable_failure: false,
            },
            ExecutionEvent::Free { operations: vec![] },
            ExecutionEvent::PaidFromAssetLockWithoutIdentity {
                processing_fees: 0,
                operations: vec![],
            },
        ];

        for event in events {
            let result = platform
                .platform
                .validate_fees_of_event_v0(
                    &event,
                    &block_info,
                    None,
                    platform_version,
                    &previous_fee_versions,
                )
                .expect("no-fee events should always succeed");

            assert!(
                result.errors.is_empty(),
                "no-fee event should produce no consensus errors"
            );
            assert!(result.data.is_some(), "default FeeResult must be present");
            let fee = result.into_data().expect("data present");
            assert_eq!(fee, FeeResult::default());
        }
    }

    /// `ExecutionEvent::PaidFromAssetLockToPool` with `fees_to_add_to_pool`
    /// strictly greater than the required fee must return a valid
    /// `ConsensusValidationResult` with NO errors. With empty `operations`
    /// and `execution_operations`, the required fee is zero so any
    /// non-negative `fees_to_add_to_pool` satisfies `>= required_fee`.
    ///
    /// It must ALSO advertise the authoritative pool fee for `gas_wanted`: the
    /// arm books `fees_to_add_to_pool` regardless of the metered estimate, so the
    /// advertised `FeeResult` (split storage/processing) must total exactly
    /// `fees_to_add_to_pool`. Mirrors the `additional_fixed_fee_cost` advertisement
    /// assertion in the `PaidFromAddressInputs` test above.
    #[test]
    fn validate_fees_of_event_v0_paid_from_asset_lock_to_pool_sufficient_fee_is_valid() {
        let platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let platform_version = PlatformVersion::latest();
        let block_info = dpp::block::block_info::BlockInfo::default();
        let previous_fee_versions = Default::default();

        let fees_to_add_to_pool = 1_000_000u64;
        let event = ExecutionEvent::PaidFromAssetLockToPool {
            fees_to_add_to_pool,
            added_to_balance_outputs: None,
            operations: vec![],
            execution_operations: vec![],
        };

        let result = platform
            .platform
            .validate_fees_of_event_v0(
                &event,
                &block_info,
                None,
                platform_version,
                &previous_fee_versions,
            )
            .expect("sufficient pool fee must be Ok");

        assert!(
            result.errors.is_empty(),
            "sufficient fees_to_add_to_pool must not produce consensus errors"
        );
        // The advertised fee must equal the authoritative pool fee (`fees_to_add_to_pool`),
        // not the metered estimate (which is 0 here with empty operations). The arm splits
        // the total into storage/processing, so assert on the total.
        let fee = result.into_data().expect("fee result present");
        assert_eq!(
            fee.total_base_fee(),
            fees_to_add_to_pool,
            "advertised pool fee must equal fees_to_add_to_pool (gas_wanted parity)"
        );
    }

    /// `ExecutionEvent::PaidFromAddressInputs` (the transparent `Shield` /
    /// `IdentityCreateFromAddresses` path) must ADVERTISE the same fee block execution charges:
    /// the metered base fee PLUS any `additional_fixed_fee_cost`. CheckTx turns the advertised
    /// `FeeResult` into `gas_wanted`, and execution folds the fixed cost into `processing_fee`, so
    /// the advertised fee must include it too — otherwise `gas_wanted` under-reports the fee. With
    /// empty operations the metered fee is 0, so the advertised fee must equal exactly the fixed
    /// cost (for a `Shield` this is the shielded compute fee).
    #[test]
    fn validate_fees_of_event_v0_paid_from_address_inputs_advertises_additional_fixed_fee_cost() {
        use dpp::address_funds::fee_strategy::AddressFundsFeeStrategyStep;
        use dpp::address_funds::PlatformAddress;
        use std::collections::BTreeMap;

        let platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let platform_version = PlatformVersion::latest();
        let block_info = dpp::block::block_info::BlockInfo::default();
        let previous_fee_versions = Default::default();

        let fixed_cost = 100_000_000u64;
        let address = PlatformAddress::P2pkh([0x11; 20]);
        let mut input_current_balances = BTreeMap::new();
        // Balance comfortably covers the fixed cost so the deduction fully succeeds.
        input_current_balances.insert(address, (1u32, fixed_cost + 1_000_000));

        let event = ExecutionEvent::PaidFromAddressInputs {
            input_current_balances,
            added_to_balance_outputs: BTreeMap::new(),
            fee_strategy: vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
            operations: vec![],
            execution_operations: vec![],
            additional_fixed_fee_cost: Some(fixed_cost),
            user_fee_increase: 0,
        };

        let result = platform
            .platform
            .validate_fees_of_event_v0(
                &event,
                &block_info,
                None,
                platform_version,
                &previous_fee_versions,
            )
            .expect("validation must be Ok");

        assert!(
            result.errors.is_empty(),
            "sufficient balance must not produce consensus errors"
        );
        let fee = result.into_data().expect("fee result present");
        // Empty operations => metered storage/processing are 0, so the advertised fee is exactly the
        // fixed cost. Before the gas_wanted fix, processing_fee was 0 here and CheckTx under-reported.
        assert_eq!(
            fee.storage_fee, 0,
            "no metered storage with empty operations"
        );
        assert_eq!(
            fee.processing_fee, fixed_cost,
            "advertised processing fee must include additional_fixed_fee_cost (gas_wanted parity)"
        );
        assert_eq!(fee.total_base_fee(), fixed_cost);
    }

    /// `ExecutionEvent::Paid` where the identity balance is smaller than
    /// the required_balance must return an `IdentityInsufficientBalanceError`
    /// inside the validation result (not an `Error`). With no `operations`
    /// but a non-zero `additional_fixed_fee_cost`, required_balance is
    /// non-zero and the zero-balance identity cannot cover it.
    #[test]
    fn validate_fees_of_event_v0_paid_insufficient_balance_returns_consensus_error() {
        let platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let platform_version = PlatformVersion::latest();
        let block_info = dpp::block::block_info::BlockInfo::default();
        let previous_fee_versions = Default::default();

        let event = ExecutionEvent::Paid {
            identity: build_partial_identity_with_balance(Some(0)),
            removed_balance: None,
            added_to_balance_outputs: None,
            operations: vec![],
            execution_operations: vec![],
            additional_fixed_fee_cost: Some(1_000),
            user_fee_increase: 0,
        };

        let result = platform
            .platform
            .validate_fees_of_event_v0(
                &event,
                &block_info,
                None,
                platform_version,
                &previous_fee_versions,
            )
            .expect("insufficient balance must be a consensus error, not a plain Error");

        assert!(
            !result.errors.is_empty(),
            "a zero-balance identity owing a fixed fee cost must yield a consensus error"
        );
        let first = &result.errors[0];
        let as_str = format!("{:?}", first);
        assert!(
            as_str.contains("IdentityInsufficientBalance"),
            "expected IdentityInsufficientBalanceError, got: {}",
            as_str
        );
    }

    /// `ExecutionEvent::PaidFromAssetLock` where the pre-top-up balance
    /// plus `added_balance` is at least the required fee (zero here) must
    /// pass without errors. This covers the "enough balance after top-up"
    /// happy-path branch that isn't exercised by the no-balance error test.
    #[test]
    fn validate_fees_of_event_v0_paid_from_asset_lock_with_balance_is_valid() {
        let platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let platform_version = PlatformVersion::latest();
        let block_info = dpp::block::block_info::BlockInfo::default();
        let previous_fee_versions = Default::default();

        let event = ExecutionEvent::PaidFromAssetLock {
            identity: build_partial_identity_with_balance(Some(10_000)),
            added_balance: 5_000,
            operations: vec![],
            execution_operations: vec![],
            user_fee_increase: 0,
        };

        let result = platform
            .platform
            .validate_fees_of_event_v0(
                &event,
                &block_info,
                None,
                platform_version,
                &previous_fee_versions,
            )
            .expect("Ok with sufficient top-up balance");

        assert!(
            result.errors.is_empty(),
            "sufficient balance + top-up must not emit consensus errors"
        );
    }

    /// `saturating_add` in the balance-with-top-up calculation must saturate,
    /// not overflow, when `added_balance` is near `u64::MAX`. This tests the
    /// implicit overflow-protection guarantee of the branch and ensures we
    /// don't regress to a plain `+` in the future.
    #[test]
    fn validate_fees_of_event_v0_paid_from_asset_lock_saturates_on_overflow() {
        let platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let platform_version = PlatformVersion::latest();
        let block_info = dpp::block::block_info::BlockInfo::default();
        let previous_fee_versions = Default::default();

        // previous_balance + added_balance would overflow without saturation.
        let event = ExecutionEvent::PaidFromAssetLock {
            identity: build_partial_identity_with_balance(Some(u64::MAX)),
            added_balance: u64::MAX,
            operations: vec![],
            execution_operations: vec![],
            user_fee_increase: 0,
        };

        let result = platform
            .platform
            .validate_fees_of_event_v0(
                &event,
                &block_info,
                None,
                platform_version,
                &previous_fee_versions,
            )
            .expect("saturating add must prevent panics on overflow");

        // The saturated balance (u64::MAX) trivially covers the zero fee,
        // so there should be no consensus errors.
        assert!(
            result.errors.is_empty(),
            "saturating balance must cover the zero required fee"
        );
    }
}
