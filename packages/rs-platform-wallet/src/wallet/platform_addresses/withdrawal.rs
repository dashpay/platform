use std::collections::BTreeMap;

use dpp::address_funds::{AddressFundsFeeStrategy, AddressFundsFeeStrategyStep, PlatformAddress};
use dpp::fee::Credits;
use dpp::identity::core_script::CoreScript;
use dpp::identity::signer::Signer;
use dpp::state_transition::address_credit_withdrawal_transition::AddressCreditWithdrawalTransition;
use dpp::version::PlatformVersion;
use dpp::withdrawal::Pooling;
use key_wallet::PlatformP2PKHAddress;

use super::InputSelection;
use crate::changeset::Merge;
use crate::wallet::PlatformAddressWallet;
use crate::{PlatformAddressChangeSet, PlatformWalletError};
use dash_sdk::platform::transition::address_credit_withdrawal::WithdrawAddressFunds;

/// The fully-planned shape of an AUTO withdrawal, computed by
/// [`PlatformAddressWallet::plan_withdrawal`] without any signing, broadcast,
/// or Core-address consumption.
///
/// A `WithdrawalPlan` is the single source of truth for *can this account
/// withdraw, and for how much*: it carries the dust-filtered, fee-reserved
/// input map and matching fee strategy that the real `withdraw(...)` path
/// signs and submits, alongside the two figures a UI preflight needs
/// (`net_withdrawable`, `estimated_fee`). Building the plan and executing it
/// from the **same** function guarantees the preflight gate and the spend
/// path can never drift — there is no second, parallel fee/min computation to
/// fall out of sync with the protocol version.
///
/// Constructing a plan is a pure, in-memory computation over the account's
/// cached balances and the active platform version; it does **not** touch the
/// Core receive pool (the fee estimate depends only on the input/output
/// *counts*, not on any destination script), so a preflight can be run on
/// every input change without burning a receive address.
#[derive(Debug, Clone)]
pub struct WithdrawalPlan {
    /// The adjusted **withdraw-amount** map: each chosen input address mapped
    /// to the amount to withdraw from it (the fee-source input's amount is
    /// already reduced by `estimated_fee` so the chain has fee headroom). This
    /// is what `withdraw(...)` hands to the SDK as the explicit input set.
    pub inputs: BTreeMap<PlatformAddress, Credits>,
    /// The fee strategy targeting the fee-source (largest-balance) input by
    /// its BTreeMap index. The AUTO path owns this because only the planner
    /// knows the final input ordering.
    pub fee_strategy: AddressFundsFeeStrategy,
    /// The net credits that will actually be withdrawn:
    /// `Σ inputs − estimated_fee`. This is the figure a UI should show as
    /// "amount to withdraw" and the figure that must clear
    /// `system_limits.min_withdrawal_amount`.
    pub net_withdrawable: Credits,
    /// The estimated address-credit-withdrawal transition fee reserved on the
    /// fee-source input, sized from the selected input count (no change
    /// output) and the active platform version's fee schedule.
    pub estimated_fee: Credits,
}

impl PlatformAddressWallet {
    /// Withdraw platform credits to a Core L1 address.
    ///
    /// Input addresses can be specified explicitly or selected automatically
    /// from the account via [`InputSelection::Auto`].
    ///
    /// If `platform_version` is `None`, the wallet's SDK version
    /// (`self.sdk.version()`) is used for fee estimation and every
    /// version-keyed limit during auto-selection — the same source the UI
    /// preflight reads, so the gate and the spend path never diverge on a
    /// non-latest-pinned SDK. An explicit `Some(v)` is honored as given.
    ///
    /// `address_signer` produces ECDSA signatures for the input
    /// [`PlatformAddress`]es; the wallet struct carries no key material
    /// itself (see the type-level docs on
    /// [`PlatformAddressWallet`]).
    #[allow(clippy::too_many_arguments)]
    pub async fn withdraw<S: Signer<PlatformAddress> + Send + Sync>(
        &self,
        account_index: u32,
        input_selection: InputSelection,
        output_script: CoreScript,
        core_fee_per_byte: u32,
        fee_strategy: AddressFundsFeeStrategy,
        platform_version: Option<&PlatformVersion>,
        address_signer: &S,
    ) -> Result<PlatformAddressChangeSet, PlatformWalletError> {
        // Validate that the output script is a supported type (P2PKH or P2SH).
        if !output_script.is_p2pkh() && !output_script.is_p2sh() {
            return Err(PlatformWalletError::AddressOperation(
                "Output script must be P2PKH or P2SH".to_string(),
            ));
        }

        // Single source of truth for the planning version: when the caller
        // pins an explicit `Some(v)` we honor it, but the default is the
        // wallet's SDK version (`self.sdk.version()`) — NOT
        // `LATEST_PLATFORM_VERSION`. This is the same network-floored,
        // protocol-version-tracking accessor that `preflight_withdrawal`,
        // `min_input_amount`, and `min_output_amount` read, so the preflight
        // gate and this spend path size every version-keyed value
        // (min_input_amount, min_withdrawal_amount, max_address_inputs,
        // max_withdrawal_amount, and `estimate_min_fee`) against the SAME
        // version. Defaulting to LATEST here would let the gate and the spend
        // path diverge on a non-latest-pinned SDK.
        let version = platform_version.unwrap_or_else(|| self.sdk.version());

        let address_infos = match input_selection {
            InputSelection::Explicit(inputs) => {
                if inputs.is_empty() {
                    return Err(PlatformWalletError::AddressOperation(
                        "Withdrawal requires at least one input address".to_string(),
                    ));
                }
                self.sdk
                    .withdraw_address_funds(
                        inputs,
                        None,
                        fee_strategy,
                        core_fee_per_byte,
                        Pooling::Never,
                        output_script,
                        address_signer,
                        None,
                    )
                    .await?
            }
            InputSelection::ExplicitWithNonces(inputs) => {
                if inputs.is_empty() {
                    return Err(PlatformWalletError::AddressOperation(
                        "Withdrawal requires at least one input address".to_string(),
                    ));
                }
                self.sdk
                    .withdraw_address_funds_with_nonce(
                        inputs,
                        None,
                        fee_strategy,
                        core_fee_per_byte,
                        Pooling::Never,
                        output_script,
                        address_signer,
                        None,
                    )
                    .await?
            }
            InputSelection::Auto => {
                // The AUTO path owns its own fee strategy: it picks the
                // fee-source input by balance (largest selected input) and
                // emits the matching `DeductFromInput(index)`, ignoring the
                // caller's `fee_strategy`. The caller cannot know the final
                // BTreeMap ordering of auto-selected inputs, so trusting a
                // hardcoded index (e.g. the wrapper's `DeductFromInput(0)`,
                // which resolves to the lex-smallest address regardless of
                // balance) would reserve the fee on an arbitrarily small
                // input and reject otherwise-fundable withdrawals.
                //
                // Selection, fee estimation, fee reservation, and the
                // minimum-withdrawal check all live in `plan_withdrawal`, the
                // SAME function the UI preflight calls. Executing the plan it
                // returns (rather than re-deriving inputs/fee here) guarantees
                // the preflight gate and this spend path can never disagree
                // about whether — or for how much — the account can withdraw.
                let plan = self.plan_withdrawal(account_index, version).await?;
                self.sdk
                    .withdraw_address_funds(
                        plan.inputs,
                        None,
                        plan.fee_strategy,
                        core_fee_per_byte,
                        Pooling::Never,
                        output_script,
                        address_signer,
                        None,
                    )
                    .await?
            }
        };

        // Get the cached key source from the unified provider for gap
        // limit maintenance.
        let key_source = {
            let guard = self.provider.read().await;
            guard
                .as_ref()
                .and_then(|p| p.key_source(&self.wallet_id, account_index))
        };

        // Update balances in the ManagedPlatformAccount.
        let mut wm = self.wallet_manager.write().await;
        let mut cs = PlatformAddressChangeSet::default();
        if let Some(info) = wm.get_wallet_info_mut(&self.wallet_id) {
            if let Some(account) = info
                .core_wallet
                .platform_payment_managed_account_at_index_mut(account_index)
            {
                for (addr, maybe_info) in address_infos.iter() {
                    let PlatformAddress::P2pkh(hash) = addr else {
                        continue;
                    };
                    let p2pkh = PlatformP2PKHAddress::new(*hash);
                    let funds = match maybe_info {
                        Some(ai) => dash_sdk::platform::address_sync::AddressFunds {
                            balance: ai.balance,
                            nonce: ai.nonce,
                        },
                        None => dash_sdk::platform::address_sync::AddressFunds {
                            balance: 0,
                            nonce: 0,
                        },
                    };
                    account.set_address_credit_balance(p2pkh, funds.balance, key_source.as_ref());
                    let address_index = account
                        .addresses
                        .addresses
                        .iter()
                        .find_map(|(&idx, info)| {
                            PlatformP2PKHAddress::from_address(&info.address)
                                .ok()
                                .filter(|found| *found == p2pkh)
                                .map(|_| idx)
                        })
                        .unwrap_or(0);
                    cs.addresses.push(crate::PlatformAddressBalanceEntry {
                        wallet_id: self.wallet_id,
                        account_index,
                        address_index,
                        address: p2pkh,
                        funds,
                    });
                }
            }
        }
        drop(wm);

        // Mirror `transfer.rs` / `sync.rs`: persist post-broadcast balances so a
        // restart doesn't reseed `plan_withdrawal` from stale rows (which would
        // let a non-Swift caller, or any host where the SwiftData write
        // side-channel is absent, build invalid follow-up spends against
        // pre-withdrawal balances). Log-on-error because the on-chain
        // transition already succeeded.
        if !cs.is_empty() {
            if let Err(e) = self.persister.store(cs.clone().into()) {
                tracing::error!("Failed to persist withdrawal changeset: {}", e);
            }
        }

        Ok(cs)
    }

    /// Plan an AUTO withdrawal for `account_index` against the SDK's
    /// **current** platform version, without signing, broadcasting, or
    /// touching the Core receive pool.
    ///
    /// This is the public preflight entry point: it resolves the version from
    /// the wallet's SDK (the same network-floored, protocol-version-tracking
    /// source the real spend runs under) and delegates to
    /// [`plan_withdrawal`](Self::plan_withdrawal). On success the returned
    /// [`WithdrawalPlan`] reports `net_withdrawable`/`estimated_fee` for a UI
    /// summary; the *typed* error variants distinguish a genuine "can't fund"
    /// (`OnlyDustInputs`, or the `AddressOperation` fee/minimum-withdrawal
    /// failures) from a hard failure (missing wallet/account), letting the FFI
    /// surface "can't fund" as a normal disabled-button result rather than an
    /// error.
    ///
    /// Because the plan it returns is the exact same object `withdraw(...)`'s
    /// AUTO path executes, gating the UI on this can never enable a withdrawal
    /// the spend path would then reject (or vice versa).
    pub async fn preflight_withdrawal(
        &self,
        account_index: u32,
    ) -> Result<WithdrawalPlan, PlatformWalletError> {
        let version = self.sdk.version();
        self.plan_withdrawal(account_index, version).await
    }

    /// Build the full [`WithdrawalPlan`] for an AUTO withdrawal: select the
    /// withdrawable funded addresses, estimate the transition fee, reserve it
    /// on the largest-balance input, and verify the result clears the minimum
    /// withdrawal amount — the complete planning phase shared by the UI
    /// preflight and the real `withdraw(...)` spend path. NO signing,
    /// broadcast, or receive-address consumption happens here.
    ///
    /// Only addresses whose balance reaches `min_input_amount` are selected:
    /// DPP's `AddressCreditWithdrawalTransition` v0 validator rejects the
    /// *entire* transition if any input amount is below
    /// `platform_version.dpp.state_transitions.address_funds.min_input_amount`
    /// (see `InputBelowMinimumError` in
    /// `address_credit_withdrawal_transition/v0/state_transition_validation.rs`),
    /// so a single sub-minimum "dust" address would otherwise fail an
    /// otherwise-fundable withdrawal. The auto path therefore withdraws the
    /// full *withdrawable* (≥ `min_input_amount`) balance, NOT literally every
    /// credit — sub-minimum dust is left in place. This mirrors the transfer
    /// path's `build_auto_select_candidates`, which applies the same filter.
    /// When every funded address is dust we return a typed
    /// [`PlatformWalletError::OnlyDustInputs`], matching the transfer path's
    /// `detect_no_selectable_inputs`.
    ///
    /// The per-input `Credits` value in the plan's `inputs` map is the amount
    /// to *withdraw* from that address, not its on-chain balance. The chain
    /// deducts the transition fee from each input's **remaining** balance
    /// (`on_chain_balance − withdraw_amount`), so a withdraw amount equal to
    /// the full balance leaves zero remaining and is rejected with
    /// `fee_fully_covered = false` — see
    /// `test_exact_balance_withdrawal_fails_insufficient_remaining_for_fees`
    /// in the drive-abci address-credit-withdrawal tests, and the transfer
    /// path's `select_inputs_deduct_from_input` for the same invariant.
    ///
    /// We therefore select every withdrawable address at its full balance, then
    /// reduce the withdraw amount on the **largest-balance** selected input
    /// by the estimated fee so that input keeps `≥ estimated_fee` of
    /// remaining balance for the chain to deduct. The largest input is the
    /// most likely to absorb the fee while staying above `min_input_amount`,
    /// so picking it (rather than the lexicographically-smallest index-0
    /// entry) avoids rejecting an otherwise-fundable withdrawal when the
    /// lex-smallest input happens to be tiny.
    ///
    /// The plan's `fee_strategy` targets the fee-source input. The AUTO path
    /// owns this strategy because only it knows the final BTreeMap ordering of
    /// the auto-selected inputs (and therefore which `DeductFromInput(index)`
    /// resolves to the largest input).
    pub(crate) async fn plan_withdrawal(
        &self,
        account_index: u32,
        platform_version: &PlatformVersion,
    ) -> Result<WithdrawalPlan, PlatformWalletError> {
        let wm = self.wallet_manager.read().await;
        let info = wm.get_wallet_info(&self.wallet_id).ok_or_else(|| {
            PlatformWalletError::WalletNotFound(format!(
                "Wallet {:?} not found in wallet manager",
                hex::encode(self.wallet_id)
            ))
        })?;

        let account = info
            .core_wallet
            .platform_payment_managed_account_at_index(account_index)
            .ok_or_else(|| {
                PlatformWalletError::AddressSync(format!(
                    "No platform payment account at index {}",
                    account_index
                ))
            })?;

        // Collect every funded address's (PlatformAddress, on-chain balance)
        // pair, then let the helper apply the per-input-minimum filter and
        // classify the dust-only case. Keeping the filter in a free function
        // mirrors the transfer path and makes the dust policy unit-testable
        // without a live wallet.
        let funded = account
            .addresses
            .addresses
            .values()
            .filter_map(|addr_info| {
                PlatformP2PKHAddress::from_address(&addr_info.address)
                    .ok()
                    .map(|p2pkh| {
                        let balance = account.address_credit_balance(&p2pkh);
                        (PlatformAddress::P2pkh(p2pkh.to_bytes()), balance)
                    })
            });

        let selected = select_withdrawable_inputs(funded, platform_version)?;

        reserve_withdrawal_fee_on_largest_input(selected, platform_version)
    }
}

/// Filter the funded addresses to those withdrawable on their own — i.e. with a
/// balance of at least `min_input_amount`.
///
/// DPP's `AddressCreditWithdrawalTransition` v0 validator rejects the **entire**
/// transition if *any* input amount is below
/// `platform_version.dpp.state_transitions.address_funds.min_input_amount`, so a
/// single sub-minimum "dust" address would otherwise sink an otherwise-fundable
/// withdrawal. We therefore drop dust here, mirroring the transfer path's
/// `build_auto_select_candidates`.
///
/// Returns the selected full-balance input map. When no address clears the
/// minimum we return a typed error: [`PlatformWalletError::OnlyDustInputs`] when
/// every funded address is dust (an actionable consolidate-funds case, mirroring
/// the transfer path's `detect_no_selectable_inputs`), or
/// [`PlatformWalletError::AddressOperation`] when there are no funds at all.
fn select_withdrawable_inputs<I>(
    funded: I,
    platform_version: &PlatformVersion,
) -> Result<BTreeMap<PlatformAddress, Credits>, PlatformWalletError>
where
    I: IntoIterator<Item = (PlatformAddress, Credits)>,
{
    let min_input_amount = platform_version
        .dpp
        .state_transitions
        .address_funds
        .min_input_amount;

    let mut selected = BTreeMap::new();
    let mut sub_min_count: usize = 0;
    let mut sub_min_aggregate: Credits = 0;

    for (address, balance) in funded {
        if balance >= min_input_amount {
            selected.insert(address, balance);
        } else if balance > 0 {
            sub_min_count = sub_min_count.saturating_add(1);
            sub_min_aggregate = sub_min_aggregate.saturating_add(balance);
        }
    }

    if selected.is_empty() {
        if sub_min_count > 0 {
            return Err(PlatformWalletError::OnlyDustInputs {
                sub_min_count,
                sub_min_aggregate,
                min_input_amount,
            });
        }
        return Err(PlatformWalletError::AddressOperation(
            "No funded addresses available for withdrawal".to_string(),
        ));
    }

    Ok(selected)
}

/// Convert a full-balance input map into a withdraw-amount map that leaves the
/// chain enough fee headroom on the fee-source input, and compute the fee
/// strategy that targets that input.
///
/// `selected` maps each chosen input address to its **full on-chain balance**.
/// The chain deducts the transition fee from each input's *remaining* balance
/// (`on_chain_balance − withdraw_amount`); since the auto path has no change
/// output, withdrawing the full balance everywhere leaves zero remaining and
/// the chain rejects the transition with `fee_fully_covered = false`. We reduce
/// the withdraw amount on the **largest-balance** selected input by the
/// estimated fee, so that input retains exactly `estimated_fee` of remaining
/// balance for the chain to deduct. This mirrors the transfer path's
/// `select_inputs_deduct_from_input` invariant: the `DeductFromInput` target
/// must keep `balance − consumed ≥ estimated_fee`.
///
/// Picking the largest input as the fee source (rather than the
/// lexicographically-smallest index-0 entry) is what makes an otherwise-
/// fundable withdrawal succeed: the on-chain `DeductFromInput(index)` resolves
/// against BTreeMap iteration order, which is address-hash ordering — unrelated
/// to balance. A tiny lex-smallest input could fail to absorb the fee even
/// when a much larger peer trivially could. We therefore locate the largest
/// input, then emit `DeductFromInput(<its position in BTreeMap order>)`.
///
/// Also enforces the two DPP structure limits the auto path could otherwise
/// trip after signing, so the preflight gate, this spend path, and the DPP
/// validator stay in lockstep: the selected input count must not exceed
/// `platform_version.dpp.state_transitions.max_address_inputs`
/// (`TransitionOverMaxInputsError`), and the net withdrawal must not exceed
/// `platform_version.system_limits.max_withdrawal_amount`
/// (`WithdrawalBelowMinAmountError`, the range error). Both surface as typed
/// "can't fund" errors with consolidate/split guidance rather than auto-capping
/// (which would change the "withdraw the full withdrawable balance" semantics).
///
/// Returns a [`WithdrawalPlan`] carrying the adjusted withdraw-amount map, the
/// fee strategy targeting the fee-source input, and the `net_withdrawable` /
/// `estimated_fee` figures; or a typed [`PlatformWalletError::AddressOperation`]
/// when no input can absorb the fee while respecting the per-input minimum, the
/// net falls below the minimum withdrawal amount, there are too many inputs, or
/// the net exceeds the maximum withdrawal amount.
fn reserve_withdrawal_fee_on_largest_input(
    mut selected: BTreeMap<PlatformAddress, Credits>,
    platform_version: &PlatformVersion,
) -> Result<WithdrawalPlan, PlatformWalletError> {
    // DPP's `AddressCreditWithdrawalTransition` v0 validator rejects the whole
    // transition when `inputs.len() > max_address_inputs` (16 on v2/v3) with
    // `TransitionOverMaxInputsError` — see `validate_structure` in
    // `address_credit_withdrawal_transition/v0/state_transition_validation.rs`.
    // The auto path uses exactly one input per selected withdrawable address,
    // so an account with more than `max_address_inputs` funded (≥ min_input)
    // addresses would otherwise preflight as withdrawable, sign, then
    // deterministically fail structure validation. Gate it here so the
    // preflight reports `can_withdraw = false` with an actionable
    // "too many inputs — consolidate" reason BEFORE signing. We ERROR rather
    // than silently dropping inputs down to the cap: capping would change the
    // "withdraw the full withdrawable balance" semantics and is a product
    // decision out of scope.
    let max_address_inputs = platform_version.dpp.state_transitions.max_address_inputs as usize;
    if selected.len() > max_address_inputs {
        return Err(PlatformWalletError::AddressOperation(format!(
            "Too many funded addresses to withdraw at once: {} addresses clear the \
             per-input minimum but the protocol allows at most {} inputs per \
             withdrawal. Consolidate funds onto fewer addresses, then withdraw.",
            selected.len(),
            max_address_inputs
        )));
    }

    let accumulated: Credits = selected
        .values()
        .copied()
        .fold(0, |acc, b| acc.saturating_add(b));

    // Estimate the transition fee for the selected input count (no change
    // output on the auto path).
    let estimated_fee = AddressCreditWithdrawalTransition::estimate_min_fee(
        selected.len(),
        false, // no change output
        platform_version,
    );

    // Locate the fee-source input: the largest balance, ties broken by the
    // first in BTreeMap (address-hash) order so the choice is deterministic.
    // `max_by_key` returns the *last* maximal element on ties, so iterate and
    // keep the first occurrence of the maximum explicitly.
    let (fee_source_index, fee_source_addr, fee_source_balance) = selected
        .iter()
        .enumerate()
        .fold(None, |best, (idx, (&addr, &balance))| match best {
            Some((_, _, best_balance)) if best_balance >= balance => best,
            _ => Some((idx, addr, balance)),
        })
        .expect("selected is non-empty: callers reject empty input maps");

    // The reduced fee-source amount must still be ≥ `min_input_amount`, and the
    // overall withdrawal (accumulated − estimated_fee) must clear the minimum
    // withdrawal amount, otherwise the transition is rejected on-chain.
    let min_input_amount = platform_version
        .dpp
        .state_transitions
        .address_funds
        .min_input_amount;
    let min_withdrawal_amount = platform_version.system_limits.min_withdrawal_amount;
    // DPP rejects `withdrawal_amount > max_withdrawal_amount` (50_000_000_000_000
    // = 500 DASH on v1/v2 system_limits) with `WithdrawalBelowMinAmountError`
    // (the range error carries both bounds) — see `validate_structure` in
    // `address_credit_withdrawal_transition/v0/state_transition_validation.rs`.
    // The auto path withdraws the full withdrawable balance, so an account whose
    // aggregate-minus-fee exceeds the maximum would otherwise preflight as
    // withdrawable, sign, then fail structure validation. Fold the max into the
    // same range check as the min below.
    let max_withdrawal_amount = platform_version.system_limits.max_withdrawal_amount;

    let withdraw_total = accumulated.saturating_sub(estimated_fee);
    if accumulated <= estimated_fee || withdraw_total < min_withdrawal_amount {
        return Err(PlatformWalletError::AddressOperation(format!(
            "Insufficient balance for withdrawal fee: available {} credits, \
             estimated fee {}, leaving {} below the minimum withdrawal amount {}",
            accumulated, estimated_fee, withdraw_total, min_withdrawal_amount
        )));
    }
    if withdraw_total > max_withdrawal_amount {
        // ERROR rather than auto-cap: capping would change the "withdraw the
        // full withdrawable balance" semantics and is a product decision out
        // of scope. A clear "exceeds the maximum — split it up" message
        // matches the validator and tells the user what to do.
        return Err(PlatformWalletError::AddressOperation(format!(
            "Withdrawal amount {} exceeds the maximum single withdrawal of {} \
             credits. Withdraw to fewer addresses at a time, or split the \
             withdrawal into multiple transactions.",
            withdraw_total, max_withdrawal_amount
        )));
    }

    let fee_source_amount = fee_source_balance.saturating_sub(estimated_fee);
    if fee_source_amount < min_input_amount {
        // The largest input cannot absorb the fee while staying above the
        // per-input minimum, so no input can: a genuine insufficiency.
        return Err(PlatformWalletError::AddressOperation(format!(
            "Cannot reserve withdrawal fee on the fee-source input: largest input \
             balance {} minus estimated fee {} leaves {}, below the minimum input \
             amount {}. Consolidate funds onto fewer addresses or fund the largest \
             address more before withdrawing.",
            fee_source_balance, estimated_fee, fee_source_amount, min_input_amount
        )));
    }

    // Same key → BTreeMap ordering (and thus the index resolution below) is
    // preserved; only the withdraw amount on the fee-source input shrinks.
    selected.insert(fee_source_addr, fee_source_amount);

    // The fee-strategy index is a u16; guard the narrowing so a pathological
    // input count (> u16::MAX) errors instead of silently wrapping to the
    // wrong fee-source input.
    let fee_source_index_u16: u16 = fee_source_index.try_into().map_err(|_| {
        PlatformWalletError::AddressOperation(format!(
            "Too many withdrawal inputs: fee-source index {} exceeds u16::MAX",
            fee_source_index
        ))
    })?;

    let fee_strategy = vec![AddressFundsFeeStrategyStep::DeductFromInput(
        fee_source_index_u16,
    )];

    Ok(WithdrawalPlan {
        inputs: selected,
        fee_strategy,
        // `withdraw_total = accumulated − estimated_fee` is the net amount the
        // chain pays out (the fee is booked from the fee-source input's
        // remaining balance). We computed and validated it above against
        // `min_withdrawal_amount`, so it is the figure a UI should display.
        net_withdrawable: withdraw_total,
        estimated_fee,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `PlatformAddress::P2pkh` is `Ord`-derived, so a smaller leading byte sorts
    /// first in the BTreeMap and becomes the `DeductFromInput(0)` target.
    fn addr(first_byte: u8) -> PlatformAddress {
        let mut bytes = [0u8; 20];
        bytes[0] = first_byte;
        PlatformAddress::P2pkh(bytes)
    }

    fn estimated_fee(input_count: usize, pv: &PlatformVersion) -> Credits {
        AddressCreditWithdrawalTransition::estimate_min_fee(input_count, false, pv)
    }

    /// A single funded input must keep `estimated_fee` of headroom: the withdraw
    /// amount on the fee-source input is its balance minus the estimated fee, NOT
    /// the full balance (which would leave zero remaining → `fee_fully_covered =
    /// false` on-chain). With one input it is trivially the largest, so the
    /// emitted strategy targets index 0.
    #[test]
    fn reserves_fee_headroom_on_single_input() {
        let pv = PlatformVersion::latest();
        let fee = estimated_fee(1, pv);
        let balance = fee + dpp::dash_to_credits!(1.0);

        let mut input = BTreeMap::new();
        input.insert(addr(1), balance);

        let plan = reserve_withdrawal_fee_on_largest_input(input, pv)
            .expect("single funded input above the fee should select");

        assert_eq!(plan.inputs.get(&addr(1)).copied(), Some(balance - fee));
        assert_eq!(
            plan.fee_strategy,
            vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
            "the single input is the fee source at index 0"
        );
        // The plan's reported figures: the net is the full balance minus the
        // reserved fee, and `estimated_fee` matches the schedule for 1 input.
        assert_eq!(plan.estimated_fee, fee);
        assert_eq!(plan.net_withdrawable, balance - fee);
    }

    /// The reviewer's scenario, corrected: input[0] (lex-smallest, BTreeMap
    /// index 0) is much smaller than the fee while a larger peer exists. The fee
    /// must now be reserved on the LARGER peer (the fee source picked by
    /// balance), so the small lex-smallest input is withdrawn in full and the
    /// larger input's withdraw amount drops by the fee. The emitted strategy
    /// must target the larger input's BTreeMap index, NOT index 0.
    #[test]
    fn reserves_fee_on_largest_input_even_when_lex_smallest_is_tiny() {
        let pv = PlatformVersion::latest();
        let fee = estimated_fee(2, pv);
        // Small lex-smallest input: too small to absorb the fee on its own
        // (would have failed the old index-0 path), but withdrawn in full here.
        let small = dpp::dash_to_credits!(0.001);
        let large = dpp::dash_to_credits!(10.0);

        let mut inputs = BTreeMap::new();
        inputs.insert(addr(1), small); // lex-smallest → BTreeMap index 0
        inputs.insert(addr(9), large); // larger → BTreeMap index 1

        let plan = reserve_withdrawal_fee_on_largest_input(inputs, pv)
            .expect("the larger peer can absorb the fee");

        assert_eq!(
            plan.inputs.get(&addr(1)).copied(),
            Some(small),
            "the small lex-smallest input is withdrawn in full"
        );
        assert_eq!(
            plan.inputs.get(&addr(9)).copied(),
            Some(large - fee),
            "the fee is reserved on the largest input"
        );
        assert_eq!(
            plan.fee_strategy,
            vec![AddressFundsFeeStrategyStep::DeductFromInput(1)],
            "the emitted DeductFromInput index points at the largest input (BTreeMap index 1)"
        );
        // The net withdrawable is the aggregate minus the reserved fee, the
        // figure a UI preflight would display.
        assert_eq!(plan.estimated_fee, fee);
        assert_eq!(plan.net_withdrawable, small + large - fee);
    }

    /// The emitted `DeductFromInput` index points at the largest input even when
    /// that input is NOT the last in BTreeMap (address-hash) order — i.e. the
    /// balance ranking and the address-hash ranking disagree.
    #[test]
    fn emitted_index_points_at_largest_input_not_last() {
        let pv = PlatformVersion::latest();
        let fee = estimated_fee(3, pv);
        let large = dpp::dash_to_credits!(10.0);
        let small_a = dpp::dash_to_credits!(0.01);
        let small_b = dpp::dash_to_credits!(0.02);

        let mut inputs = BTreeMap::new();
        inputs.insert(addr(1), large); // lex-smallest → BTreeMap index 0, largest balance
        inputs.insert(addr(5), small_a); // index 1
        inputs.insert(addr(9), small_b); // index 2

        let plan = reserve_withdrawal_fee_on_largest_input(inputs, pv)
            .expect("the largest input can absorb the fee");

        assert_eq!(
            plan.fee_strategy,
            vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
            "the largest input is at BTreeMap index 0, so the fee deducts from index 0"
        );
        assert_eq!(plan.inputs.get(&addr(1)).copied(), Some(large - fee));
        assert_eq!(plan.inputs.get(&addr(5)).copied(), Some(small_a));
        assert_eq!(plan.inputs.get(&addr(9)).copied(), Some(small_b));
        assert_eq!(plan.estimated_fee, fee);
        assert_eq!(plan.net_withdrawable, large + small_a + small_b - fee);
    }

    /// Genuine insufficiency: even the LARGEST input cannot retain
    /// `estimated_fee` while keeping its withdraw amount ≥ `min_input_amount`,
    /// so no input can. We error rather than ship a guaranteed-rejected
    /// transition (mirrors the transfer path's headroom error). The aggregate
    /// here clears `min_withdrawal_amount`, so the error is specifically the
    /// per-input headroom failure, not the aggregate-too-small gate.
    #[test]
    fn errors_when_largest_input_too_small_to_absorb_fee() {
        let pv = PlatformVersion::latest();
        let fee = estimated_fee(3, pv);
        let min_input = pv.dpp.state_transitions.address_funds.min_input_amount;
        let min_withdrawal = pv.system_limits.min_withdrawal_amount;

        // Largest input leaves < min_input after the fee is reserved.
        let large = fee + min_input - 1;
        // Two equal peers, each smaller than `large` so it stays the maximum,
        // sized so the aggregate clears `min_withdrawal_amount + fee`.
        let peer = large / 2;

        let mut inputs = BTreeMap::new();
        inputs.insert(addr(1), peer);
        inputs.insert(addr(5), peer);
        inputs.insert(addr(9), large);

        // Sanity: the aggregate clears the withdrawal minimum, so the only
        // remaining failure path is the largest-input headroom check.
        let accumulated = peer + peer + large;
        assert!(
            accumulated.saturating_sub(fee) >= min_withdrawal,
            "test setup: aggregate must clear the withdrawal minimum"
        );

        let err = reserve_withdrawal_fee_on_largest_input(inputs, pv)
            .expect_err("largest input below fee + min_input must error");
        assert!(matches!(err, PlatformWalletError::AddressOperation(_)));
    }

    /// Aggregate balance below the fee (or leaving less than the minimum
    /// withdrawal amount) is rejected up front.
    #[test]
    fn errors_when_total_below_fee() {
        let pv = PlatformVersion::latest();
        let fee = estimated_fee(1, pv);

        let mut inputs = BTreeMap::new();
        inputs.insert(addr(1), fee - 1);

        let err = reserve_withdrawal_fee_on_largest_input(inputs, pv)
            .expect_err("balance below the fee must error");
        assert!(matches!(err, PlatformWalletError::AddressOperation(_)));
    }

    // ---- Planner-shaped tests for the new `WithdrawalPlan` contract ----
    //
    // These exercise the planning phase the UI preflight and the real
    // `withdraw(...)` path share, focusing on the three figures the preflight
    // surfaces: a fee-covering success returns the expected net, and the two
    // genuine "can't-fund" cases the FFI must report as `can_withdraw = false`.

    /// Covers-fee success: a single input comfortably above `min_input_amount`
    /// + the fee yields a plan whose `net_withdrawable` is exactly
    /// `Σ inputs − estimated_fee` and whose `inputs` reserve that fee on the
    /// fee-source input. This is the figure a UI preflight displays as "amount
    /// to withdraw".
    #[test]
    fn plan_covers_fee_returns_expected_net() {
        let pv = PlatformVersion::latest();
        let fee = estimated_fee(1, pv);
        let balance = fee + dpp::dash_to_credits!(2.0);

        let mut inputs = BTreeMap::new();
        inputs.insert(addr(3), balance);

        let plan = reserve_withdrawal_fee_on_largest_input(inputs, pv)
            .expect("a balance above min_input + fee must plan successfully");

        assert_eq!(plan.estimated_fee, fee);
        assert_eq!(
            plan.net_withdrawable,
            balance - fee,
            "net withdrawable is the input sum minus the reserved fee"
        );
        assert_eq!(
            plan.inputs.get(&addr(3)).copied(),
            Some(balance - fee),
            "the fee-source input keeps fee headroom"
        );
    }

    /// Single-input-below-(min_input + fee): the only funded input cannot keep
    /// `≥ min_input_amount` after reserving the fee, so no input can absorb it.
    /// The planner must return a typed "can't-fund" error (NOT a panic, NOT a
    /// success) so the FFI can report `can_withdraw = false`.
    ///
    /// The two guards (`accumulated ≤ fee || net < min_withdrawal`) are checked
    /// before the per-input headroom check, so this test sizes the input to net
    /// **above** `min_withdrawal_amount` — isolating the per-input headroom
    /// failure. With more than one input (so the largest is not trivially the
    /// whole aggregate) the per-input check is the only one left to fire. This
    /// is the multi-input variant of `errors_when_largest_input_too_small_to_
    /// absorb_fee`, reframed around the planner's "can't-fund" contract.
    #[test]
    fn plan_largest_input_below_min_plus_fee_cant_fund() {
        let pv = PlatformVersion::latest();
        let fee = estimated_fee(2, pv);
        let min_input = pv.dpp.state_transitions.address_funds.min_input_amount;
        let min_withdrawal = pv.system_limits.min_withdrawal_amount;

        // Largest input leaves < min_input after the fee is reserved on it.
        let large = fee + min_input - 1;
        // A peer (smaller than `large`, so `large` stays the maximum) sized so
        // the aggregate-after-fee clears the withdrawal minimum, isolating the
        // per-input headroom failure from the aggregate gate.
        let peer = min_withdrawal + min_input; // ≥ min_input, < large
        assert!(
            peer < large,
            "test setup: peer must stay below the largest input"
        );

        let mut inputs = BTreeMap::new();
        inputs.insert(addr(2), peer);
        inputs.insert(addr(8), large);

        // Sanity: the aggregate clears the withdrawal minimum after the fee, so
        // the only remaining failure path is the largest-input headroom check.
        let accumulated = peer + large;
        assert!(
            accumulated.saturating_sub(fee) >= min_withdrawal,
            "test setup: aggregate-after-fee must clear the withdrawal minimum"
        );

        let err = reserve_withdrawal_fee_on_largest_input(inputs, pv)
            .expect_err("the largest input below min_input + fee cannot fund a withdrawal");
        assert!(
            matches!(err, PlatformWalletError::AddressOperation(_)),
            "can't-fund must be a typed error the FFI maps to can_withdraw = false"
        );
    }

    /// Aggregate-below-min_withdrawal-after-fee: a single input clears
    /// `min_input_amount` and is larger than the fee (so it is not rejected by
    /// the `accumulated ≤ fee` guard), yet its net (`balance − fee`) is still
    /// below `system_limits.min_withdrawal_amount`. The planner must reject
    /// this as a typed "can't-fund" error so the FFI reports
    /// `can_withdraw = false` rather than shipping a transition the chain
    /// rejects on the minimum.
    ///
    /// Constructed only when `min_withdrawal_amount > 0` (always true on the
    /// real versions). The input is `fee + min_withdrawal − 1`, so it exceeds
    /// the fee but nets exactly one credit short of the withdrawal floor.
    #[test]
    fn plan_aggregate_below_min_withdrawal_cant_fund() {
        let pv = PlatformVersion::latest();
        let fee = estimated_fee(1, pv);
        let min_input = pv.dpp.state_transitions.address_funds.min_input_amount;
        let min_withdrawal = pv.system_limits.min_withdrawal_amount;

        // Net = balance − fee = min_withdrawal − 1, i.e. one credit short of
        // the withdrawal floor while still clearing the fee.
        let balance = fee + min_withdrawal - 1;
        // The input itself clears the per-input minimum, so this is genuinely
        // the aggregate-below-min_withdrawal gate, not the dust filter.
        assert!(
            balance >= min_input,
            "test setup: the input must clear the per-input minimum"
        );
        assert!(
            balance > fee,
            "test setup: the input must exceed the fee so the accumulated ≤ fee guard doesn't fire"
        );

        let mut inputs = BTreeMap::new();
        inputs.insert(addr(4), balance);

        let err = reserve_withdrawal_fee_on_largest_input(inputs, pv)
            .expect_err("an input that nets below the withdrawal floor cannot fund a withdrawal");
        assert!(
            matches!(err, PlatformWalletError::AddressOperation(_)),
            "can't-fund must be a typed error the FFI maps to can_withdraw = false"
        );
    }

    /// More than `max_address_inputs` funded (≥ min_input) addresses must NOT
    /// preflight as withdrawable: DPP's v0 validator rejects the whole
    /// transition with `TransitionOverMaxInputsError` once `inputs.len()`
    /// exceeds the cap. The auto path uses one input per selected address, so
    /// the planner must surface this as a typed "can't fund — too many inputs"
    /// error (mapped to `can_withdraw = false` by the FFI) rather than shipping
    /// a guaranteed-rejected transition. We size each input well above
    /// `min_input_amount + fee` so neither the per-input headroom nor the
    /// aggregate gates fire — isolating the input-count cap as the only failure.
    #[test]
    fn plan_more_than_max_inputs_cant_fund() {
        let pv = PlatformVersion::latest();
        let max_inputs = pv.dpp.state_transitions.max_address_inputs as usize;
        // One input per address, one more than the cap. Each input is large
        // enough that the only thing wrong is the count.
        let per_input = dpp::dash_to_credits!(1.0);

        let mut inputs = BTreeMap::new();
        for i in 0..=max_inputs {
            // Distinct addresses via the leading two bytes; max_inputs is 16 on
            // the real versions, so a u8 first byte is plenty.
            let mut bytes = [0u8; 20];
            bytes[0] = i as u8;
            inputs.insert(PlatformAddress::P2pkh(bytes), per_input);
        }
        assert_eq!(
            inputs.len(),
            max_inputs + 1,
            "test setup: must hold one more input than the cap"
        );

        let err = reserve_withdrawal_fee_on_largest_input(inputs, pv)
            .expect_err("more than max_address_inputs funded inputs cannot withdraw at once");
        match err {
            PlatformWalletError::AddressOperation(msg) => assert!(
                msg.contains("at most") && msg.contains("inputs"),
                "expected a too-many-inputs message, got: {msg}"
            ),
            other => panic!("expected AddressOperation too-many-inputs, got {other:?}"),
        }
    }

    /// A withdrawal whose net (aggregate − fee) exceeds
    /// `system_limits.max_withdrawal_amount` (500 DASH) must NOT preflight as
    /// withdrawable: DPP's v0 validator rejects the transition with the
    /// `WithdrawalBelowMinAmountError` range error once
    /// `withdrawal_amount > max_withdrawal_amount`. A single input keeps the
    /// input-count gate trivially satisfied, isolating the max-amount check.
    #[test]
    fn plan_aggregate_above_max_withdrawal_cant_fund() {
        let pv = PlatformVersion::latest();
        let fee = estimated_fee(1, pv);
        let max_withdrawal = pv.system_limits.max_withdrawal_amount;

        // Net = balance − fee = max_withdrawal + 1, i.e. one credit over the
        // maximum while a single input keeps the count gate satisfied.
        let balance = fee + max_withdrawal + 1;

        let mut inputs = BTreeMap::new();
        inputs.insert(addr(7), balance);

        let err = reserve_withdrawal_fee_on_largest_input(inputs, pv)
            .expect_err("a net above the maximum withdrawal cannot be withdrawn in one go");
        match err {
            PlatformWalletError::AddressOperation(msg) => assert!(
                msg.contains("exceeds the maximum"),
                "expected an exceeds-maximum message, got: {msg}"
            ),
            other => panic!("expected AddressOperation exceeds-maximum, got {other:?}"),
        }
    }

    /// AUTO selection must drop sub-`min_input_amount` dust: the chain rejects
    /// the whole transition if any input is below the per-input minimum, so a
    /// single dust address must NOT sink an otherwise-fundable withdrawal. The
    /// fundable peers are selected at full balance; the dust address is
    /// excluded. (Withdrawal therefore takes the full *withdrawable* balance,
    /// not literally every credit.)
    #[test]
    fn select_withdrawable_inputs_excludes_dust_keeps_fundable() {
        let pv = PlatformVersion::latest();
        let min_input = pv.dpp.state_transitions.address_funds.min_input_amount;

        let dust = min_input - 1; // below the per-input minimum
        let fundable_a = min_input; // exactly at the minimum is withdrawable
        let fundable_b = dpp::dash_to_credits!(1.0);

        let funded = vec![
            (addr(1), dust),
            (addr(5), fundable_a),
            (addr(9), fundable_b),
        ];

        let selected =
            select_withdrawable_inputs(funded, pv).expect("fundable peers exist beside the dust");

        assert_eq!(
            selected.get(&addr(1)).copied(),
            None,
            "the sub-minimum dust address is excluded"
        );
        assert_eq!(selected.get(&addr(5)).copied(), Some(fundable_a));
        assert_eq!(selected.get(&addr(9)).copied(), Some(fundable_b));
        assert_eq!(selected.len(), 2, "only the two fundable inputs survive");
    }

    /// An account whose every funded address is dust returns the typed
    /// `OnlyDustInputs` error (mirroring the transfer path), carrying the
    /// dust count/aggregate and the active `min_input_amount` so the UI can
    /// tell the user to consolidate funds — never a guaranteed-rejected
    /// transition.
    #[test]
    fn select_withdrawable_inputs_only_dust_errors_typed() {
        let pv = PlatformVersion::latest();
        let min_input = pv.dpp.state_transitions.address_funds.min_input_amount;

        let dust_a = min_input - 1;
        let dust_b = min_input / 2;
        let funded = vec![(addr(1), dust_a), (addr(9), dust_b)];

        let err = select_withdrawable_inputs(funded, pv)
            .expect_err("an all-dust account cannot withdraw");
        match err {
            PlatformWalletError::OnlyDustInputs {
                sub_min_count,
                sub_min_aggregate,
                min_input_amount,
            } => {
                assert_eq!(sub_min_count, 2);
                assert_eq!(sub_min_aggregate, dust_a + dust_b);
                assert_eq!(min_input_amount, min_input);
            }
            other => panic!("expected OnlyDustInputs, got {other:?}"),
        }
    }

    /// No funds at all (every balance is zero) is distinct from the dust case:
    /// it falls through to the generic `AddressOperation` error rather than
    /// `OnlyDustInputs`.
    #[test]
    fn select_withdrawable_inputs_no_funds_errors_generic() {
        let pv = PlatformVersion::latest();
        let funded = vec![(addr(1), 0u64), (addr(9), 0u64)];

        let err = select_withdrawable_inputs(funded, pv)
            .expect_err("a zero-balance account cannot withdraw");
        assert!(
            matches!(err, PlatformWalletError::AddressOperation(_)),
            "no-funds case is the generic error, not OnlyDustInputs"
        );
    }
}
