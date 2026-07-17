mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use dpp::address_funds::{AddressFundsFeeStrategy, PlatformAddress};
use dpp::asset_lock::reduced_asset_lock_value::AssetLockValueGettersV0;
use dpp::block::epoch::Epoch;
use dpp::fee::Credits;
use std::collections::BTreeMap;

use dpp::identity::PartialIdentity;
use dpp::prelude::{AddressNonce, UserFeeIncrease};

use dpp::version::PlatformVersion;
use drive::state_transition_action::StateTransitionAction;

use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use drive::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use drive::state_transition_action::system::bump_address_input_nonces_action::BumpAddressInputNonceActionAccessorsV0;
use drive::state_transition_action::system::partially_use_asset_lock_action::PartiallyUseAssetLockActionAccessorsV0;
use drive::util::batch::DriveOperation;

/// An execution event
#[derive(Clone)]
pub(in crate::execution) enum ExecutionEvent<'a> {
    /// A drive event that is paid by an identity
    Paid {
        /// The identity requesting the event
        identity: PartialIdentity,
        /// The removed balance in the case of a transfer or withdrawal
        removed_balance: Option<Credits>,
        /// Optional address outputs that should be tracked (for IdentityCreditTransferToAddresses)
        added_to_balance_outputs: Option<BTreeMap<PlatformAddress, Credits>>,
        /// the operations that the identity is requesting to perform
        operations: Vec<DriveOperation<'a>>,
        /// the execution operations that we must also pay for
        execution_operations: Vec<ValidationOperation>,
        /// Additional fee cost, these are processing fees where the user fee increase does not apply
        additional_fixed_fee_cost: Option<Credits>,
        /// the fee multiplier that the user agreed to, 0 means 100% of the base fee, 1 means 101%
        user_fee_increase: UserFeeIncrease,
    },
    /// A drive event that is paid by address inputs, this one can also be used by asset lock to address
    PaidFromAddressInputs {
        /// The removed balance in the case of a transfer or withdrawal
        input_current_balances: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        /// These are credits we added as outputs, we can only deduct fees of the amount we added.
        added_to_balance_outputs: BTreeMap<PlatformAddress, Credits>,
        /// Fee strategy
        fee_strategy: AddressFundsFeeStrategy,
        /// the operations that we are requesting to perform
        operations: Vec<DriveOperation<'a>>,
        /// the execution operations that we must also pay for
        execution_operations: Vec<ValidationOperation>,
        /// Additional fee cost, these are processing fees where the user fee increase does not apply.
        ///
        /// `Shield` (transparent shield) sets this to the shielded COMPUTE fee
        /// (`compute_shielded_verification_fee`: proof verification + per-action processing), which is
        /// added to the metered processing fee on top of the metered GroveDB storage of the
        /// note/nullifier writes — exactly like `IdentityCreateFromAddresses` adds its registration
        /// cost. No storage term is added here; storage comes entirely from metering.
        additional_fixed_fee_cost: Option<Credits>,
        /// the fee multiplier that the user agreed to, 0 means 100% of the base fee, 1 means 101%
        user_fee_increase: UserFeeIncrease,
    },
    /// A drive event that has a fixed cost that will be taken out in the operations
    PaidFixedCost {
        /// the operations that should be performed
        operations: Vec<DriveOperation<'a>>,
        /// fees to add
        fees_to_add_to_pool: Credits,
    },
    /// A drive event paid from the shielded pool's value_balance.
    /// The fee is embedded in the ZK-proven value_balance and validated
    /// at the processor level (validate_minimum_shielded_fee).
    /// Nullifiers are stored to recent block storage as part of the drive operations.
    ///
    /// This variant deliberately carries no `user_fee_increase`. Shielded transitions
    /// have no fee-bidding or priority market: the fee is pinned to the flat,
    /// client-predictable `compute_minimum_shielded_fee` (transfers must set
    /// `value_balance` to exactly that minimum, while unshields and withdrawals derive it
    /// from the action count), so every shielded transition of a given size pays an
    /// identical fee and no fee fingerprint can distinguish senders. It likewise carries
    /// no `execution_operations`: shielded transitions are not charged the per-operation
    /// GroveDB cost (the flat fee subsumes it), so the execution context is intentionally
    /// not threaded through here.
    PaidFromShieldedPool {
        /// the operations that should be performed
        operations: Vec<DriveOperation<'a>>,
        /// fees derived from value_balance to add to the fee pool
        fees_to_add_to_pool: Credits,
        /// Transparent platform-address credits produced by this shielded spend, to be folded into
        /// the recent-address-balance-changes tree (`store_address_balances_for_block`) so
        /// incremental client sync sees them. `Some` only for an `UnshieldAction` whose NET amount
        /// (`amount - fee`) is positive — mirroring the `AddBalanceToAddress` op the converter
        /// emits — which includes the `IdentityCreateFromShieldedPool` duplicate-key fallback,
        /// since that fallback surfaces as an `UnshieldAction` crediting the fallback address.
        /// `None` for ShieldedTransfer and ShieldedWithdrawal: they credit no transparent
        /// address, so there is nothing for incremental sync to observe.
        added_to_balance_outputs: Option<BTreeMap<PlatformAddress, Credits>>,
        /// `true` ONLY for the `IdentityCreateFromShieldedPool` chargeable-failure fallback. It
        /// authorizes the executor to apply `operations` even when consensus errors are attached
        /// (the spend is finalized to the fallback address minus the penalty). For every ordinary
        /// shielded spend (Unshield / ShieldedTransfer / ShieldedWithdrawal) this is `false`, so an
        /// error-bearing event of those types is NEVER applied — the apply-despite-errors contract
        /// is type-enforced here, not just by convention.
        chargeable_failure: bool,
    },
    /// A drive event that is paid from an asset lock
    PaidFromAssetLock {
        /// The identity requesting the event
        identity: PartialIdentity,
        /// The added balance
        added_balance: Credits,
        /// the operations that should be performed
        operations: Vec<DriveOperation<'a>>,
        /// the execution operations that we must also pay for
        execution_operations: Vec<ValidationOperation>,
        /// the fee multiplier that the user agreed to, 0 means 100% of the base fee, 1 means 101%
        user_fee_increase: UserFeeIncrease,
    },
    /// A drive event that is paid from an asset lock
    PaidFromAssetLockWithoutIdentity {
        /// The processing fees that should be distributed to validators
        processing_fees: Credits,
        /// the operations that should be performed
        operations: Vec<DriveOperation<'a>>,
    },
    /// A drive event paid from an asset lock with funds going to the shielded pool (with fee validation)
    PaidFromAssetLockToPool {
        /// Fee (asset_lock_value - shield_amount) to add to the fee pool
        fees_to_add_to_pool: Credits,
        /// Transparent platform-address credit produced by this shield, to be folded into the
        /// recent-address-balance-changes tree (`store_address_balances_for_block`) so incremental
        /// client sync sees it. `Some` only when `ShieldFromAssetLock` routes an asset-lock surplus
        /// to a `surplus_output` address (mirroring the converter's `AddBalanceToAddress` op); `None`
        /// when there is no surplus output (the surplus folds into the fee pools instead).
        added_to_balance_outputs: Option<BTreeMap<PlatformAddress, Credits>>,
        /// the operations that should be performed
        operations: Vec<DriveOperation<'a>>,
        /// the execution operations that we must also pay for
        execution_operations: Vec<ValidationOperation>,
    },
    /// A drive event for `IdentityCreateFromShieldedPool`: the new identity is created holding the
    /// full `denomination` (debited from the shielded pool by the converter), then the metered
    /// GroveDB write cost PLUS the flat shielded compute fee (`additional_fixed_fee_cost`) is MOVED
    /// from the new identity's balance into the fee pools — so the identity ends with
    /// `denomination - total_fee` and the credit supply is conserved. Mirrors `PaidFromAssetLock`
    /// (create-then-deduct-from-the-new-identity) but funded from the shielded pool instead of an
    /// asset lock. The metered write grows with the key count, which is why this transition meters
    /// rather than carving a flat pool fee like the other pool-paid shielded transitions.
    PaidFromShieldedPoolToNewIdentity {
        /// The new identity (id derived from the spend nullifiers, balance = `denomination`).
        identity: PartialIdentity,
        /// the operations that should be performed
        operations: Vec<DriveOperation<'a>>,
        /// the execution operations that we must also pay for (per-key signature verifications)
        execution_operations: Vec<ValidationOperation>,
        /// The exit denomination = the new identity's initial balance = the affordability ceiling
        /// the fee must not exceed.
        denomination: Credits,
        /// The flat shielded COMPUTE fee (Halo 2 proof verification + per-action processing) added
        /// to the metered processing fee — GroveDB cannot meter the ZK work.
        additional_fixed_fee_cost: Option<Credits>,
    },
    /// A drive event that is free
    #[allow(dead_code)] // TODO investigate why `variant `Free` is never constructed`
    Free {
        /// the operations that should be performed
        operations: Vec<DriveOperation<'a>>,
    },
}

impl ExecutionEvent<'_> {
    pub(crate) fn create_from_state_transition_action(
        action: StateTransitionAction,
        identity: Option<PartialIdentity>,
        epoch: &Epoch,
        execution_context: StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        match &action {
            StateTransitionAction::IdentityCreateAction(identity_create_action) => {
                let user_fee_increase = identity_create_action.user_fee_increase();
                let identity = identity_create_action.into();
                let operations =
                    action.into_high_level_drive_operations(epoch, platform_version)?;
                Ok(ExecutionEvent::PaidFromAssetLock {
                    identity,
                    added_balance: 0,
                    operations,
                    execution_operations: execution_context.operations_consume(),
                    user_fee_increase,
                })
            }
            StateTransitionAction::IdentityTopUpAction(identity_top_up_action) => {
                let user_fee_increase = identity_top_up_action.user_fee_increase();
                let added_balance = identity_top_up_action
                    .top_up_asset_lock_value()
                    .remaining_credit_value();
                let operations =
                    action.into_high_level_drive_operations(epoch, platform_version)?;
                if let Some(identity) = identity {
                    Ok(ExecutionEvent::PaidFromAssetLock {
                        identity,
                        added_balance,
                        operations,
                        execution_operations: execution_context.operations_consume(),
                        user_fee_increase,
                    })
                } else {
                    Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "partial identity should be present for identity top up action",
                    )))
                }
            }
            StateTransitionAction::PartiallyUseAssetLockAction(partially_used_asset_lock) => {
                let used_credits = partially_used_asset_lock.used_credits();
                let operations =
                    action.into_high_level_drive_operations(epoch, platform_version)?;
                // We mark it as a free operation because the event itself is paying for itself
                Ok(ExecutionEvent::PaidFromAssetLockWithoutIdentity {
                    processing_fees: used_credits,
                    operations,
                })
            }
            StateTransitionAction::IdentityCreditWithdrawalAction(identity_credit_withdrawal) => {
                let user_fee_increase = identity_credit_withdrawal.user_fee_increase();
                let removed_balance = identity_credit_withdrawal.amount();
                let operations =
                    action.into_high_level_drive_operations(epoch, platform_version)?;
                if let Some(identity) = identity {
                    Ok(ExecutionEvent::Paid {
                        identity,
                        removed_balance: Some(removed_balance),
                        added_to_balance_outputs: None,
                        operations,
                        execution_operations: execution_context.operations_consume(),
                        additional_fixed_fee_cost: None,
                        user_fee_increase,
                    })
                } else {
                    Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "partial identity should be present for identity credit withdrawal action",
                    )))
                }
            }
            StateTransitionAction::IdentityCreditTransferAction(identity_credit_transfer) => {
                let user_fee_increase = identity_credit_transfer.user_fee_increase();
                let removed_balance = identity_credit_transfer.transfer_amount();
                let operations =
                    action.into_high_level_drive_operations(epoch, platform_version)?;
                if let Some(identity) = identity {
                    Ok(ExecutionEvent::Paid {
                        identity,
                        removed_balance: Some(removed_balance),
                        added_to_balance_outputs: None,
                        operations,
                        execution_operations: execution_context.operations_consume(),
                        additional_fixed_fee_cost: None,
                        user_fee_increase,
                    })
                } else {
                    Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "partial identity should be present for identity credit transfer action",
                    )))
                }
            }
            StateTransitionAction::BatchAction(batch_action) => {
                let user_fee_increase = action.user_fee_increase();
                let removed_balance = batch_action.all_used_balances()?;
                let operations =
                    action.into_high_level_drive_operations(epoch, platform_version)?;
                if let Some(identity) = identity {
                    Ok(ExecutionEvent::Paid {
                        identity,
                        removed_balance,
                        added_to_balance_outputs: None,
                        operations,
                        execution_operations: execution_context.operations_consume(),
                        additional_fixed_fee_cost: None,
                        user_fee_increase,
                    })
                } else {
                    Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "partial identity should be present for other state transitions",
                    )))
                }
            }
            StateTransitionAction::MasternodeVoteAction(_) => {
                let operations =
                    action.into_high_level_drive_operations(epoch, platform_version)?;

                Ok(ExecutionEvent::PaidFixedCost {
                    operations,
                    fees_to_add_to_pool: platform_version
                        .fee_version
                        .vote_resolution_fund_fees
                        .contested_document_single_vote_cost,
                })
            }
            StateTransitionAction::DataContractCreateAction(data_contract_create_action) => {
                let user_fee_increase = action.user_fee_increase();
                let registration_cost = data_contract_create_action
                    .data_contract_ref()
                    .registration_cost(platform_version)?;
                let operations =
                    action.into_high_level_drive_operations(epoch, platform_version)?;
                if let Some(identity) = identity {
                    Ok(ExecutionEvent::Paid {
                        identity,
                        removed_balance: None,
                        added_to_balance_outputs: None,
                        operations,
                        execution_operations: execution_context.operations_consume(),
                        additional_fixed_fee_cost: Some(registration_cost),
                        user_fee_increase,
                    })
                } else {
                    Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "partial identity should be present for other state transitions",
                    )))
                }
            }
            StateTransitionAction::DataContractUpdateAction(data_contract_update_action) => {
                let user_fee_increase = action.user_fee_increase();
                // The update cost pays for the whole contract again, that's fine for now.
                // It would be a lot of work to fix this, so we'll just go with this for now.
                let registration_cost = data_contract_update_action
                    .data_contract_ref()
                    .registration_cost(platform_version)?;
                let operations =
                    action.into_high_level_drive_operations(epoch, platform_version)?;
                if let Some(identity) = identity {
                    Ok(ExecutionEvent::Paid {
                        identity,
                        removed_balance: None,
                        added_to_balance_outputs: None,
                        operations,
                        execution_operations: execution_context.operations_consume(),
                        additional_fixed_fee_cost: Some(registration_cost),
                        user_fee_increase,
                    })
                } else {
                    Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "partial identity should be present for other state transitions",
                    )))
                }
            }
            StateTransitionAction::AddressFundsTransfer(address_funds_transfer_action) => {
                let user_fee_increase = address_funds_transfer_action.user_fee_increase();
                let input_current_balances = address_funds_transfer_action
                    .inputs_with_remaining_balance()
                    .clone();
                let added_to_balance_outputs = address_funds_transfer_action.outputs().clone();
                let fee_strategy = address_funds_transfer_action.fee_strategy().clone();
                let operations =
                    action.into_high_level_drive_operations(epoch, platform_version)?;
                Ok(ExecutionEvent::PaidFromAddressInputs {
                    input_current_balances,
                    added_to_balance_outputs,
                    fee_strategy,
                    operations,
                    execution_operations: execution_context.operations_consume(),
                    additional_fixed_fee_cost: None,
                    user_fee_increase,
                })
            }
            StateTransitionAction::AddressFundingFromAssetLock(
                address_funding_from_asset_lock_action,
            ) => {
                let user_fee_increase = address_funding_from_asset_lock_action.user_fee_increase();
                let input_current_balances = address_funding_from_asset_lock_action
                    .inputs_with_remaining_balance()
                    .clone();
                // Use resolved_outputs to compute the remainder and get concrete amounts
                let added_to_balance_outputs =
                    address_funding_from_asset_lock_action.resolved_outputs();
                let fee_strategy = address_funding_from_asset_lock_action
                    .fee_strategy()
                    .clone();
                let operations =
                    action.into_high_level_drive_operations(epoch, platform_version)?;
                Ok(ExecutionEvent::PaidFromAddressInputs {
                    input_current_balances,
                    added_to_balance_outputs,
                    fee_strategy,
                    operations,
                    execution_operations: execution_context.operations_consume(),
                    additional_fixed_fee_cost: None,
                    user_fee_increase,
                })
            }
            StateTransitionAction::AddressCreditWithdrawal(address_credit_withdrawal_action) => {
                let user_fee_increase = address_credit_withdrawal_action.user_fee_increase();
                let input_current_balances = address_credit_withdrawal_action
                    .inputs_with_remaining_balance()
                    .clone();
                let added_to_balance_outputs =
                    if let Some(output) = address_credit_withdrawal_action.output() {
                        [output].into()
                    } else {
                        BTreeMap::new()
                    };
                let fee_strategy = address_credit_withdrawal_action.fee_strategy().clone();
                let operations =
                    action.into_high_level_drive_operations(epoch, platform_version)?;
                Ok(ExecutionEvent::PaidFromAddressInputs {
                    input_current_balances,
                    added_to_balance_outputs,
                    fee_strategy,
                    operations,
                    execution_operations: execution_context.operations_consume(),
                    additional_fixed_fee_cost: None,
                    user_fee_increase,
                })
            }
            StateTransitionAction::IdentityCreateFromAddressesAction(
                identity_create_from_addresses_action,
            ) => {
                let user_fee_increase = identity_create_from_addresses_action.user_fee_increase();
                let input_current_balances = identity_create_from_addresses_action
                    .inputs_with_remaining_balance()
                    .clone();
                let added_to_balance_outputs =
                    if let Some(output) = identity_create_from_addresses_action.output() {
                        [output].into()
                    } else {
                        BTreeMap::new()
                    };
                let fee_strategy = identity_create_from_addresses_action.fee_strategy().clone();
                let operations =
                    action.into_high_level_drive_operations(epoch, platform_version)?;
                Ok(ExecutionEvent::PaidFromAddressInputs {
                    input_current_balances,
                    added_to_balance_outputs,
                    fee_strategy,
                    operations,
                    execution_operations: execution_context.operations_consume(),
                    additional_fixed_fee_cost: None,
                    user_fee_increase,
                })
            }
            StateTransitionAction::IdentityTopUpFromAddressesAction(
                identity_top_up_from_addresses_action,
            ) => {
                let user_fee_increase = identity_top_up_from_addresses_action.user_fee_increase();
                let input_current_balances = identity_top_up_from_addresses_action
                    .inputs_with_remaining_balance()
                    .clone();
                let added_to_balance_outputs =
                    if let Some(output) = identity_top_up_from_addresses_action.output() {
                        [output].into()
                    } else {
                        BTreeMap::new()
                    };
                let fee_strategy = identity_top_up_from_addresses_action.fee_strategy().clone();
                let operations =
                    action.into_high_level_drive_operations(epoch, platform_version)?;
                Ok(ExecutionEvent::PaidFromAddressInputs {
                    input_current_balances,
                    added_to_balance_outputs,
                    fee_strategy,
                    operations,
                    execution_operations: execution_context.operations_consume(),
                    additional_fixed_fee_cost: None,
                    user_fee_increase,
                })
            }
            StateTransitionAction::BumpAddressInputNoncesAction(
                bump_address_input_nonces_action,
            ) => {
                let user_fee_increase = bump_address_input_nonces_action.user_fee_increase();
                let input_current_balances = bump_address_input_nonces_action
                    .inputs_with_remaining_balance()
                    .clone();
                // BumpAddressInputNoncesAction doesn't have outputs - it only bumps nonces and pays fees
                let added_to_balance_outputs = BTreeMap::new();
                let fee_strategy = bump_address_input_nonces_action.fee_strategy().clone();
                let operations =
                    action.into_high_level_drive_operations(epoch, platform_version)?;
                Ok(ExecutionEvent::PaidFromAddressInputs {
                    input_current_balances,
                    added_to_balance_outputs,
                    fee_strategy,
                    operations,
                    execution_operations: execution_context.operations_consume(),
                    additional_fixed_fee_cost: None,
                    user_fee_increase,
                })
            }
            StateTransitionAction::IdentityCreditTransferToAddressesAction(
                identity_credit_transfer_to_addresses,
            ) => {
                let user_fee_increase = identity_credit_transfer_to_addresses.user_fee_increase();
                let removed_balance: Credits = identity_credit_transfer_to_addresses
                    .recipient_addresses()
                    .values()
                    .sum();
                let added_to_balance_outputs = identity_credit_transfer_to_addresses
                    .recipient_addresses()
                    .clone();
                let operations =
                    action.into_high_level_drive_operations(epoch, platform_version)?;
                if let Some(identity) = identity {
                    Ok(ExecutionEvent::Paid {
                        identity,
                        removed_balance: Some(removed_balance),
                        added_to_balance_outputs: Some(added_to_balance_outputs),
                        operations,
                        execution_operations: execution_context.operations_consume(),
                        additional_fixed_fee_cost: None,
                        user_fee_increase,
                    })
                } else {
                    Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "partial identity should be present for identity credit transfer to addresses action",
                    )))
                }
            }
            StateTransitionAction::ShieldAction(shield_action) => {
                let user_fee_increase = shield_action.user_fee_increase();
                let input_current_balances = shield_action.inputs_with_remaining_balance().clone();
                let added_to_balance_outputs = BTreeMap::new();
                let fee_strategy = shield_action.fee_strategy().clone();
                // Transparent shield is metered + compute: GroveDB meters the real storage and
                // processing of the note/nullifier writes (via `into_high_level_drive_operations`),
                // and we add ONLY the shielded COMPUTE fee
                // `compute_shielded_verification_fee(num_actions)` (Halo 2 proof verification +
                // per-action processing) that GroveDB cannot see, as `additional_fixed_fee_cost`.
                // This is exactly the `IdentityCreateFromAddresses` model: a fixed cost added to
                // processing on top of the metered fee. No storage term is added here, so storage is
                // never double-counted. `notes` are built 1:1 from the on-wire Orchard `actions`
                // (see the shield action transformer), so `notes().len()` is the on-wire action
                // count that the structure-validation floor also prices the compute fee against.
                let shielded_verification_fee = dpp::shielded::compute_shielded_verification_fee(
                    shield_action.notes().len(),
                    platform_version,
                )?;
                let operations =
                    action.into_high_level_drive_operations(epoch, platform_version)?;
                Ok(ExecutionEvent::PaidFromAddressInputs {
                    input_current_balances,
                    added_to_balance_outputs,
                    fee_strategy,
                    operations,
                    execution_operations: execution_context.operations_consume(),
                    additional_fixed_fee_cost: Some(shielded_verification_fee),
                    user_fee_increase,
                })
            }
            StateTransitionAction::ShieldedTransferAction(ref shielded_transfer_action) => {
                let fee_amount = shielded_transfer_action.fee_amount();
                let operations =
                    action.into_high_level_drive_operations(epoch, platform_version)?;
                Ok(ExecutionEvent::PaidFromShieldedPool {
                    operations,
                    fees_to_add_to_pool: fee_amount,
                    // A shielded-to-shielded transfer credits no transparent address.
                    added_to_balance_outputs: None,
                    chargeable_failure: false,
                })
            }
            StateTransitionAction::UnshieldAction(ref unshield_action) => {
                let fee_amount = unshield_action.fee_amount();
                // An ordinary Unshield is always `false`; only the IdentityCreateFromShieldedPool
                // duplicate-key fallback (which also surfaces as an UnshieldAction) sets it `true`.
                let chargeable_failure = unshield_action.chargeable_failure();
                // The converter credits the output address the NET amount (`amount - fee`) via an
                // `AddBalanceToAddress` op, and only when that net is positive (see
                // `unshield_transition.rs`). Thread the identical value here so the credit is also
                // recorded in the recent-address-balance-changes tree for incremental sync.
                // `checked_sub` returning `None` on underflow yields no output here — the converter
                // call below then errors on the same underflow, so no event is constructed.
                let added_to_balance_outputs = unshield_action
                    .amount()
                    .checked_sub(fee_amount)
                    .filter(|net_recipient_amount| *net_recipient_amount > 0)
                    .map(|net_recipient_amount| {
                        BTreeMap::from([(*unshield_action.output_address(), net_recipient_amount)])
                    });
                let operations =
                    action.into_high_level_drive_operations(epoch, platform_version)?;
                Ok(ExecutionEvent::PaidFromShieldedPool {
                    operations,
                    fees_to_add_to_pool: fee_amount,
                    added_to_balance_outputs,
                    chargeable_failure,
                })
            }
            StateTransitionAction::ShieldFromAssetLockAction(ref shield_from_asset_lock_action) => {
                // The fully-consumed asset lock (added to system credits by the converter) is
                // distributed three ways: `shield_amount` -> shielded pool, `surplus_amount` ->
                // the `surplus_output` address (via the converter, when set), and the remainder ->
                // the fee pools (this value). Computing the fee by subtraction from the single
                // source of truth (the action) keeps conservation by construction:
                //   AddToSystemCredits(consumed) == shield_amount + surplus_amount + fee_amount.
                // When `surplus_output` is unset, `surplus_amount == 0` and the surplus folds into
                // the fee here (bounded by the implicit-fee cap enforced in the transform).
                let fee_amount = shield_from_asset_lock_action
                    .asset_lock_value_to_be_consumed()
                    .checked_sub(shield_from_asset_lock_action.shield_amount())
                    .and_then(|v| v.checked_sub(shield_from_asset_lock_action.surplus_amount()))
                    .ok_or(Error::Execution(
                        ExecutionError::CorruptedCodeExecution(
                            "shield amount + surplus exceeds asset lock value to be consumed in ShieldFromAssetLock fee computation",
                        ),
                    ))?;
                // The converter routes `surplus_amount` to the `surplus_output` platform address via
                // an `AddBalanceToAddress` op, but only when the output is set AND the surplus is
                // positive (see `shield_from_asset_lock_transition.rs`). Thread the identical credit
                // here so it is recorded in the recent-address-balance-changes tree for incremental
                // sync. When there is no surplus output the surplus folds into the fee pools instead,
                // crediting no address, so this stays `None`.
                let added_to_balance_outputs = match shield_from_asset_lock_action.surplus_output()
                {
                    Some(surplus_address) if shield_from_asset_lock_action.surplus_amount() > 0 => {
                        Some(BTreeMap::from([(
                            *surplus_address,
                            shield_from_asset_lock_action.surplus_amount(),
                        )]))
                    }
                    _ => None,
                };
                let operations =
                    action.into_high_level_drive_operations(epoch, platform_version)?;
                Ok(ExecutionEvent::PaidFromAssetLockToPool {
                    fees_to_add_to_pool: fee_amount,
                    added_to_balance_outputs,
                    operations,
                    execution_operations: execution_context.operations_consume(),
                })
            }
            StateTransitionAction::ShieldedWithdrawalAction(ref shielded_withdrawal_action) => {
                let fee_amount = shielded_withdrawal_action.fee_amount();
                let operations =
                    action.into_high_level_drive_operations(epoch, platform_version)?;
                Ok(ExecutionEvent::PaidFromShieldedPool {
                    operations,
                    fees_to_add_to_pool: fee_amount,
                    // A shielded withdrawal leaves Platform for a Core-chain output; it credits no
                    // transparent platform address, so there is nothing to record here.
                    added_to_balance_outputs: None,
                    chargeable_failure: false,
                })
            }
            StateTransitionAction::IdentityCreateFromShieldedPoolAction(ref action_ref) => {
                use std::collections::{BTreeMap, BTreeSet};
                // The new identity is created holding the full denomination; the fee is the metered
                // GroveDB write cost (from `operations`) PLUS the flat shielded COMPUTE fee
                // (proof verification + per-action processing) GroveDB cannot meter, added as
                // `additional_fixed_fee_cost` — exactly the transparent `Shield` model. That total is
                // then moved out of the new identity's balance into the fee pools at execution.
                let denomination = action_ref.denomination();
                let compute_fee = dpp::shielded::compute_shielded_verification_fee(
                    action_ref.notes().len(),
                    platform_version,
                )?;
                // Only `id` (for the fee balance-change) and `balance` (for the affordability gate)
                // are needed; the keys themselves are written by the `AddNewIdentity` operation.
                let partial_identity = PartialIdentity {
                    id: action_ref.identity_id(),
                    loaded_public_keys: BTreeMap::new(),
                    balance: Some(denomination),
                    revision: None,
                    not_found_public_keys: BTreeSet::new(),
                };
                let operations =
                    action.into_high_level_drive_operations(epoch, platform_version)?;
                Ok(ExecutionEvent::PaidFromShieldedPoolToNewIdentity {
                    identity: partial_identity,
                    operations,
                    execution_operations: execution_context.operations_consume(),
                    denomination,
                    additional_fixed_fee_cost: Some(compute_fee),
                })
            }
            _ => {
                let user_fee_increase = action.user_fee_increase();
                let operations =
                    action.into_high_level_drive_operations(epoch, platform_version)?;
                if let Some(identity) = identity {
                    Ok(ExecutionEvent::Paid {
                        identity,
                        removed_balance: None,
                        added_to_balance_outputs: None,
                        operations,
                        execution_operations: execution_context.operations_consume(),
                        additional_fixed_fee_cost: None,
                        user_fee_increase,
                    })
                } else {
                    Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "partial identity should be present for other state transitions",
                    )))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::version::DefaultForPlatformVersion;
    use drive::state_transition_action::shielded::shield_from_asset_lock::v0::ShieldFromAssetLockTransitionActionV0;
    use drive::state_transition_action::shielded::shield_from_asset_lock::ShieldFromAssetLockTransitionAction;
    use drive::state_transition_action::shielded::unshield::v0::UnshieldTransitionActionV0;
    use drive::state_transition_action::shielded::unshield::UnshieldTransitionAction;
    use drive::state_transition_action::shielded::ShieldedActionNote;

    fn note() -> ShieldedActionNote {
        ShieldedActionNote {
            nullifier: [0x11; 32],
            cmx: [0x22; 32],
            cv_net: [0x33; 32],
            encrypted_note: vec![1, 2, 3],
        }
    }

    fn ctx() -> StateTransitionExecutionContext {
        StateTransitionExecutionContext::default_for_platform_version(PlatformVersion::latest())
            .expect("execution context")
    }

    /// An `Unshield` crediting its output address a positive NET (`amount - fee`) must carry that
    /// credit in `PaidFromShieldedPool::added_to_balance_outputs`, so the executor can record it in
    /// the recent-address-balance-changes tree that incremental client sync reads.
    #[test]
    fn unshield_populates_net_output_credit() {
        let output_address = PlatformAddress::P2pkh([0xBB; 20]);
        let action = StateTransitionAction::UnshieldAction(UnshieldTransitionAction::V0(
            UnshieldTransitionActionV0 {
                output_address,
                amount: 3000,
                notes: vec![note()],
                anchor: [0xAA; 32],
                fee_amount: 500,
                current_total_balance: 10_000,
                chargeable_failure: false,
            },
        ));

        let event = ExecutionEvent::create_from_state_transition_action(
            action,
            None,
            &Epoch::new(0).unwrap(),
            ctx(),
            PlatformVersion::latest(),
        )
        .expect("create event");

        match event {
            ExecutionEvent::PaidFromShieldedPool {
                added_to_balance_outputs,
                ..
            } => {
                let outputs =
                    added_to_balance_outputs.expect("net > 0 must carry an output credit");
                assert_eq!(outputs.len(), 1);
                // 3000 amount - 500 fee = 2500 net to the output address.
                assert_eq!(outputs.get(&output_address).copied(), Some(2500));
            }
            _ => panic!("expected PaidFromShieldedPool"),
        }
    }

    /// A net-zero `Unshield` (the whole unshielding amount consumed by the fee) credits no address —
    /// the converter emits no `AddBalanceToAddress`, so the event must carry no output either.
    #[test]
    fn unshield_net_zero_records_no_output_credit() {
        let action = StateTransitionAction::UnshieldAction(UnshieldTransitionAction::V0(
            UnshieldTransitionActionV0 {
                output_address: PlatformAddress::P2pkh([0xBB; 20]),
                amount: 500,
                notes: vec![note()],
                anchor: [0xAA; 32],
                fee_amount: 500, // net = 0
                current_total_balance: 10_000,
                chargeable_failure: false,
            },
        ));

        let event = ExecutionEvent::create_from_state_transition_action(
            action,
            None,
            &Epoch::new(0).unwrap(),
            ctx(),
            PlatformVersion::latest(),
        )
        .expect("create event");

        match event {
            ExecutionEvent::PaidFromShieldedPool {
                added_to_balance_outputs,
                ..
            } => assert!(
                added_to_balance_outputs.is_none(),
                "net-zero unshield must credit no address"
            ),
            _ => panic!("expected PaidFromShieldedPool"),
        }
    }

    /// The IdentityCreateFromShieldedPool duplicate-key fallback surfaces as a `chargeable_failure`
    /// `UnshieldAction` that still credits the fallback address the net amount. The event must carry
    /// that credit AND set `chargeable_failure`, so the executor records it on the applied path even
    /// though the transition is paid-invalid. (The `chargeable_failure` flag is independent of the
    /// output-credit computation — this pins the exact shape the paid-invalid seam relies on.)
    #[test]
    fn unshield_chargeable_failure_still_populates_net_output_credit() {
        let output_address = PlatformAddress::P2pkh([0xCD; 20]);
        let action = StateTransitionAction::UnshieldAction(UnshieldTransitionAction::V0(
            UnshieldTransitionActionV0 {
                output_address,
                amount: 3000,
                notes: vec![note()],
                anchor: [0xAA; 32],
                fee_amount: 500,
                current_total_balance: 10_000,
                chargeable_failure: true,
            },
        ));

        let event = ExecutionEvent::create_from_state_transition_action(
            action,
            None,
            &Epoch::new(0).unwrap(),
            ctx(),
            PlatformVersion::latest(),
        )
        .expect("create event");

        match event {
            ExecutionEvent::PaidFromShieldedPool {
                added_to_balance_outputs,
                chargeable_failure,
                ..
            } => {
                assert!(chargeable_failure, "the fallback must be chargeable");
                let outputs = added_to_balance_outputs
                    .expect("fallback must carry the fallback-address credit");
                assert_eq!(outputs.get(&output_address).copied(), Some(2500));
            }
            _ => panic!("expected PaidFromShieldedPool"),
        }
    }

    /// A `ShieldFromAssetLock` routing a surplus to a `surplus_output` address must carry that credit
    /// in `PaidFromAssetLockToPool::added_to_balance_outputs`.
    #[test]
    fn shield_from_asset_lock_populates_surplus_credit() {
        let surplus_address = PlatformAddress::P2pkh([0x42; 20]);
        let action = StateTransitionAction::ShieldFromAssetLockAction(
            ShieldFromAssetLockTransitionAction::V0(ShieldFromAssetLockTransitionActionV0 {
                asset_lock_outpoint: [0xDD; 36],
                asset_lock_value_to_be_consumed: 10_000,
                signable_bytes_hasher: [0xEE; 32],
                shield_amount: 5_000,
                notes: vec![note()],
                current_total_balance: 10_000,
                surplus_output: Some(surplus_address),
                surplus_amount: 2_000,
            }),
        );

        let event = ExecutionEvent::create_from_state_transition_action(
            action,
            None,
            &Epoch::new(0).unwrap(),
            ctx(),
            PlatformVersion::latest(),
        )
        .expect("create event");

        match event {
            ExecutionEvent::PaidFromAssetLockToPool {
                added_to_balance_outputs,
                fees_to_add_to_pool,
                ..
            } => {
                let outputs =
                    added_to_balance_outputs.expect("surplus must carry an output credit");
                assert_eq!(outputs.get(&surplus_address).copied(), Some(2_000));
                // fee = consumed - shield - surplus = 10000 - 5000 - 2000.
                assert_eq!(fees_to_add_to_pool, 3_000);
            }
            _ => panic!("expected PaidFromAssetLockToPool"),
        }
    }

    /// A `ShieldFromAssetLock` with no `surplus_output` credits no address (the surplus folds into
    /// the fee pools), so the event must carry no output.
    #[test]
    fn shield_from_asset_lock_without_surplus_output_records_nothing() {
        let action = StateTransitionAction::ShieldFromAssetLockAction(
            ShieldFromAssetLockTransitionAction::V0(ShieldFromAssetLockTransitionActionV0 {
                asset_lock_outpoint: [0xDD; 36],
                asset_lock_value_to_be_consumed: 10_000,
                signable_bytes_hasher: [0xEE; 32],
                shield_amount: 5_000,
                notes: vec![note()],
                current_total_balance: 10_000,
                surplus_output: None,
                surplus_amount: 0,
            }),
        );

        let event = ExecutionEvent::create_from_state_transition_action(
            action,
            None,
            &Epoch::new(0).unwrap(),
            ctx(),
            PlatformVersion::latest(),
        )
        .expect("create event");

        match event {
            ExecutionEvent::PaidFromAssetLockToPool {
                added_to_balance_outputs,
                ..
            } => assert!(added_to_balance_outputs.is_none()),
            _ => panic!("expected PaidFromAssetLockToPool"),
        }
    }
}
