use std::collections::BTreeMap;

use dpp::address_funds::{AddressFundsFeeStrategy, AddressFundsFeeStrategyStep, PlatformAddress};
use dpp::fee::Credits;
use dpp::identity::core_script::CoreScript;
use dpp::identity::signer::Signer;
use dpp::state_transition::address_credit_withdrawal_transition::AddressCreditWithdrawalTransition;
use dpp::version::PlatformVersion;
use dpp::withdrawal::Pooling;

use super::InputSelection;
use crate::error::promote_address_nonce_error_or_sdk;
use crate::wallet::PlatformAddressWallet;
use crate::{PlatformAddressChangeSet, PlatformWalletError};
use dash_sdk::platform::transition::address_credit_withdrawal::WithdrawAddressFunds;
use dash_sdk::platform::FetchMany;
use dash_sdk::query_types::AddressInfo;

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
/// Every per-input amount in [`inputs`](Self::inputs) is sized from the
/// address's **authoritative on-chain balance** — the value
/// [`plan_withdrawal`](PlatformAddressWallet::plan_withdrawal) fetches with
/// `AddressInfo::fetch_many`, which is the SAME balance the spend path
/// re-fetches and hard-checks in `fetch_inputs_with_nonce` before signing.
/// The plan is therefore never built from the wallet's cached
/// `address_credit_balance` (which a stale or racing sync could double or lag
/// behind chain), so a preflight can never approve — nor the AUTO spend
/// select — an input amount that exceeds what the address actually holds,
/// which would otherwise fail with `AddressNotEnoughFundsError`.
///
/// Constructing a plan issues one `fetch_many` proof query over the account's
/// funded addresses; it does **not** touch the Core receive pool (the fee
/// estimate depends only on the input/output *counts*, not on any destination
/// script), so a preflight can be run without burning a receive address.
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

        // `proof_height` is the broadcast proof's committed block — the
        // height pin for the reconciled absolutes below.
        let (address_infos, proof_height) = match input_selection {
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
                    .await
                    .map_err(promote_address_nonce_error_or_sdk)?
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
                    .await
                    .map_err(promote_address_nonce_error_or_sdk)?
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
                    .await
                    .map_err(promote_address_nonce_error_or_sdk)?
            }
        };

        // Apply + persist the proof-attested post-withdrawal balances via
        // the shared seam, pinned at `proof_height`. Input addresses
        // resolve through the provider's persisted index bijection (with
        // live-pool fallback), so a restored address that is no longer in
        // a live derived pool keeps its real derivation index instead of
        // corrupting index 0.
        //
        // Withdrawal inputs are recorded on-chain as `SetBalanceToAddress`
        // (absolute `SetCredits` ops), which were already replay-safe; the
        // pin additionally protects a future change output (an
        // `AddBalanceToAddress` delta) without any caller-side bookkeeping.
        Ok(self
            .reconcile_address_infos(&address_infos, proof_height, "address withdrawal")
            .await)
    }

    /// Plan an AUTO withdrawal for `account_index` against the SDK's
    /// **current** platform version, without signing, broadcasting, or
    /// touching the Core receive pool.
    ///
    /// Issues one `AddressInfo::fetch_many` proof query to read the account's
    /// authoritative on-chain balances (see
    /// [`plan_withdrawal`](Self::plan_withdrawal)); it does not sign,
    /// broadcast, or consume a receive address.
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
    /// # Balance source: on-chain, not the cache
    ///
    /// The account's derived address pool supplies the *set* of candidate
    /// addresses, but every candidate's **balance** is read from an
    /// `AddressInfo::fetch_many` proof query — the SAME authoritative value
    /// the spend path re-fetches in `fetch_inputs_with_nonce` and hard-checks
    /// with `ensure_address_balance` before signing. The wallet's cached
    /// `address_credit_balance` is deliberately NOT used to size inputs: a
    /// stale cache (lagging the chain) or a doubled cache (a sync/reconcile
    /// race writing a balance larger than reality) would let the planner
    /// select an input amount the spend path then rejects with
    /// `AddressNotEnoughFundsError` (the account's balance looked fundable in
    /// preflight, but the per-input on-chain balance was short). Sizing the
    /// plan from the same on-chain truth the spend fetches is what makes the
    /// "single source of truth" guarantee hold even when the cache is wrong.
    /// Addresses the proof reports as absent or missing (no on-chain balance)
    /// are treated as zero and filtered out as dust.
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
        // Candidate SET only — balances are read fresh from the chain below,
        // NOT from the account cache. Drops the wallet-manager read lock before
        // the network fetch so a concurrent sync/reconcile isn't blocked behind
        // the proof round-trip.
        let candidate_addresses = self.candidate_address_set(account_index).await?;

        if candidate_addresses.is_empty() {
            return Err(PlatformWalletError::AddressOperation(
                "No funded addresses available for withdrawal".to_string(),
            ));
        }

        // Read the authoritative on-chain balance for each candidate. This is
        // the exact query the spend path runs in `fetch_inputs_with_nonce`
        // (`AddressInfo::fetch_many` over a `BTreeSet<PlatformAddress>`), so
        // the amounts the plan reserves per input match what the spend will
        // re-fetch and hard-check — a stale or doubled cache can no longer
        // make the planner over-request. An address the proof reports as
        // absent / missing (`None`) has no on-chain balance and maps to 0,
        // which the dust filter then drops.
        let on_chain: dash_sdk::query_types::AddressInfos =
            AddressInfo::fetch_many(self.sdk.as_ref(), candidate_addresses).await?;

        // Collect every funded address's (PlatformAddress, on-chain balance)
        // pair, then let the helper apply the per-input-minimum filter and
        // classify the dust-only case. Keeping the filter in a free function
        // mirrors the transfer path and makes the dust policy unit-testable
        // without a live wallet.
        let funded = on_chain.into_iter().map(|(address, info)| {
            let balance = info.map(|i| i.balance).unwrap_or(0);
            (address, balance)
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

    // ---- ADDR-04 spendability regression ------------------------------------
    //
    // The withdrawal that fails at spend time does so because an input's
    // planned withdraw amount exceeds the address's on-chain balance, so the
    // spend path's `ensure_address_balance(on_chain, requested)` throws
    // `AddressNotEnoughFundsError`. The planner math itself never over-
    // requests: a non-fee-source input is planned at exactly its balance and
    // the fee-source input at `balance − fee`. These tests pin that invariant
    // — "every per-input requested amount ≤ that input's balance" — so a
    // future change to `reserve_withdrawal_fee_on_largest_input` can't
    // reintroduce an over-request. The doubled-balance ADDR-04 repro was a
    // WRONG INPUT to this function (a stale/doubled cached balance was passed
    // in); `plan_withdrawal` now sources balances from the same on-chain
    // `AddressInfo::fetch_many` proof the spend re-checks, so the values fed
    // here are the authoritative ones.

    /// Assert the plan is spendable: for every input, the planned withdraw
    /// amount is ≤ the balance that input was selected with. `balances` maps
    /// each address to the full balance passed into
    /// `reserve_withdrawal_fee_on_largest_input`.
    fn assert_plan_spendable(plan: &WithdrawalPlan, balances: &BTreeMap<PlatformAddress, Credits>) {
        for (addr, &requested) in plan.inputs.iter() {
            let balance = balances
                .get(addr)
                .copied()
                .expect("every planned input was among the selected balances");
            assert!(
                requested <= balance,
                "planned withdraw amount {requested} for {addr} exceeds its balance \
                 {balance} — the spend path would reject this with \
                 AddressNotEnoughFundsError",
            );
        }
    }

    /// Mandated multi-input plan test: one input sits at *exactly*
    /// `min_input_amount` (the smallest a withdrawable input can be) alongside
    /// a comfortably larger fee-source input. The plan must:
    ///   * withdraw the exactly-minimum input at its FULL balance (never
    ///     doubled, never more than it holds),
    ///   * reserve the fee on the larger input (`balance − fee`),
    ///   * be spendable: every per-input requested amount ≤ that input's
    ///     balance.
    #[test]
    fn plan_min_input_amount_input_is_spendable_at_full_balance() {
        let pv = PlatformVersion::latest();
        let fee = estimated_fee(2, pv);
        let min_input = pv.dpp.state_transitions.address_funds.min_input_amount;

        // The exactly-minimum input, and a larger fee-source input that can
        // absorb the fee while clearing every minimum.
        let min_amount = min_input;
        let large = fee + dpp::dash_to_credits!(1.0);

        let mut balances = BTreeMap::new();
        balances.insert(addr(1), min_amount); // lex-smallest, exactly the minimum
        balances.insert(addr(9), large); // larger → fee source

        let plan = reserve_withdrawal_fee_on_largest_input(balances.clone(), pv)
            .expect("an exactly-minimum input beside a fundable fee source must plan");

        // The exactly-minimum input is withdrawn at its FULL balance — not
        // reduced (it isn't the fee source) and never doubled.
        assert_eq!(
            plan.inputs.get(&addr(1)).copied(),
            Some(min_amount),
            "the exactly-min_input_amount input is planned at its full balance"
        );
        // The fee is reserved on the larger input.
        assert_eq!(
            plan.inputs.get(&addr(9)).copied(),
            Some(large - fee),
            "the fee is reserved on the larger fee-source input"
        );
        assert_eq!(
            plan.fee_strategy,
            vec![AddressFundsFeeStrategyStep::DeductFromInput(1)],
            "the emitted DeductFromInput index points at the larger input (BTreeMap index 1)"
        );
        assert_eq!(plan.estimated_fee, fee);
        assert_eq!(plan.net_withdrawable, min_amount + large - fee);

        // Core spendability invariant: nothing is requested beyond balance.
        assert_plan_spendable(&plan, &balances);
    }

    /// ADDR-04 shape regression: the recipient of a prior small transfer holds
    /// exactly 100_000_000 credits and is a NON-fee-source input beside the
    /// large origin address that pays the fee. The recipient input must be
    /// planned at exactly 100_000_000 — the bug reserved a *doubled*
    /// 200_000_000 (its balance had been cached at 2× reality), which the
    /// spend path rejected because on-chain it only held 100_000_000. With the
    /// correct balance fed in, the plan requests exactly the balance and is
    /// spendable.
    #[test]
    fn plan_does_not_over_request_small_recipient_input() {
        let pv = PlatformVersion::latest();
        let fee = estimated_fee(2, pv);

        // Addr#1: the ADDR-02 transfer recipient, 0.001 DASH.
        let recipient_balance: Credits = 100_000_000;
        // Addr#0: the topped-up origin, comfortably the largest → fee source.
        let origin_balance = fee + dpp::dash_to_credits!(0.0488);

        let mut balances = BTreeMap::new();
        balances.insert(addr(1), recipient_balance);
        balances.insert(addr(9), origin_balance);

        let plan = reserve_withdrawal_fee_on_largest_input(balances.clone(), pv)
            .expect("both inputs clear the minimums and the origin absorbs the fee");

        assert_eq!(
            plan.inputs.get(&addr(1)).copied(),
            Some(recipient_balance),
            "the recipient input is planned at its true balance, never doubled"
        );
        assert_ne!(
            plan.inputs.get(&addr(1)).copied(),
            Some(recipient_balance * 2),
            "the recipient input must NOT be planned at 2x its balance (ADDR-04)"
        );
        assert_eq!(
            plan.inputs.get(&addr(9)).copied(),
            Some(origin_balance - fee),
            "the origin (fee source) keeps fee headroom"
        );

        // Every input is spendable against its balance — the exact property
        // the spend path's `ensure_address_balance` enforces.
        assert_plan_spendable(&plan, &balances);
    }

    /// Generalised spendability: across a mix of input sizes, EVERY planned
    /// per-input amount stays ≤ that input's balance, and only the fee-source
    /// input is reduced (by exactly the fee). This is the invariant that keeps
    /// the preflight/plan from ever approving what the spend rejects, provided
    /// the balances fed in are the on-chain truth (which `plan_withdrawal` now
    /// guarantees by fetching them).
    #[test]
    fn plan_every_input_is_spendable_and_only_fee_source_is_reduced() {
        let pv = PlatformVersion::latest();
        let fee = estimated_fee(4, pv);

        let mut balances = BTreeMap::new();
        balances.insert(addr(1), dpp::dash_to_credits!(0.01));
        balances.insert(addr(4), dpp::dash_to_credits!(0.05));
        balances.insert(addr(7), dpp::dash_to_credits!(5.0)); // largest → fee source
        balances.insert(addr(9), dpp::dash_to_credits!(0.02));

        let plan = reserve_withdrawal_fee_on_largest_input(balances.clone(), pv)
            .expect("a spread of fundable inputs must plan");

        assert_plan_spendable(&plan, &balances);

        // The fee source (addr(7), the largest) is the only reduced input.
        let fee_source = addr(7);
        for (addr, &requested) in plan.inputs.iter() {
            let balance = balances[addr];
            if *addr == fee_source {
                assert_eq!(
                    requested,
                    balance - fee,
                    "the fee source is reduced by exactly the fee"
                );
            } else {
                assert_eq!(
                    requested, balance,
                    "a non-fee-source input is withdrawn at its full balance"
                );
            }
        }
    }
}

/// Integration test for the cache-vs-chain seam that this PR moves: the
/// `plan_withdrawal` orchestrator must size each input from the balance the
/// SDK's `AddressInfo::fetch_many` proof query returns (the on-chain truth the
/// spend re-checks), NOT from the wallet's cached `address_credit_balance`.
///
/// This is the behavior the ADDR-04 fix actually changes; the pure-function
/// tests above can't reach it because they call
/// `reserve_withdrawal_fee_on_largest_input` directly with balances already
/// chosen. Here we deliberately make the cache DISAGREE with the chain (a
/// doubled/stale cached balance for one address) and assert the plan follows
/// the chain. A future regression that reintroduced a cache read (e.g.
/// `.unwrap_or_else(|| cached_balance)`) would fail this test.
#[cfg(test)]
mod plan_withdrawal_seam_tests {
    use std::collections::BTreeSet;
    use std::sync::Arc;

    use dpp::address_funds::PlatformAddress;
    use dpp::version::PlatformVersion;
    use key_wallet::bip32::{ChildNumber, DerivationPath};
    use key_wallet::managed_account::address_pool::{
        AddressInfo as PoolAddressInfo, AddressPool, AddressPoolType,
    };
    use key_wallet::managed_account::managed_platform_account::ManagedPlatformAccount;
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;
    use key_wallet::{Network, PlatformP2PKHAddress};
    use key_wallet_manager::WalletManager;
    use tokio::sync::RwLock;

    use crate::wallet::platform_wallet::WalletId;
    use crate::wallet::PlatformAddressWallet;

    /// A deterministic testnet P2PKH address for a known 20-byte hash.
    fn address_for(byte: u8) -> dashcore::Address {
        PlatformP2PKHAddress::new([byte; 20]).to_address(Network::Testnet)
    }

    /// The `PlatformAddress::P2pkh` a given hash byte maps to (what
    /// `plan_withdrawal` derives from a pool entry, and the mock query key).
    fn platform_addr(byte: u8) -> PlatformAddress {
        PlatformAddress::P2pkh([byte; 20])
    }

    /// Build a `PoolAddressInfo` for a known testnet P2PKH address at `index`,
    /// via the public script-pubkey constructor (no private key needed).
    fn pool_entry(byte: u8, index: u32) -> PoolAddressInfo {
        let address = address_for(byte);
        let base_path = DerivationPath::from(vec![
            ChildNumber::from_hardened_idx(9).expect("purpose"),
            ChildNumber::from_hardened_idx(1).expect("coin type"),
            ChildNumber::from_hardened_idx(17).expect("feature"),
            ChildNumber::from_hardened_idx(0).expect("subfeature"),
            ChildNumber::from_hardened_idx(0).expect("account"),
        ]);
        PoolAddressInfo::new_from_script_pubkey_p2pkh(
            address.script_pubkey(),
            index,
            base_path,
            Network::Testnet,
        )
        .expect("known testnet P2PKH script")
    }

    /// The DOUBLED cache case: the recipient address's cached
    /// `address_credit_balance` is 200M (2× reality — the ADDR-04 bug), but the
    /// chain (via mocked `fetch_many`) reports its true 100M. `plan_withdrawal`
    /// must size that input at the on-chain 100M, so the plan is spendable and
    /// the spend path's `ensure_address_balance` would accept it — the cache's
    /// doubled value must NOT leak into the plan.
    #[tokio::test]
    async fn plan_withdrawal_sizes_inputs_from_chain_not_doubled_cache() {
        use dash_sdk::query_types::{AddressInfo, AddressInfos};

        const ACCOUNT: u32 = 0;

        // Addr#0 = the ADDR-02 recipient; Addr#1 = the large fee-source origin.
        let recipient_byte = 0x11u8;
        let origin_byte = 0x22u8;
        let recipient_on_chain: u64 = 100_000_000;
        let recipient_cached_doubled: u64 = 200_000_000; // the bug's 2x cache
        let origin_balance: u64 = dpp::dash_to_credits!(0.05);

        // --- Mock SDK: fetch_many returns the TRUE on-chain balances. The
        // query key is the exact BTreeSet plan_withdrawal builds from the pool.
        let mut sdk = dash_sdk::Sdk::new_mock();
        let query: BTreeSet<PlatformAddress> =
            [platform_addr(recipient_byte), platform_addr(origin_byte)]
                .into_iter()
                .collect();
        let response: AddressInfos = [
            (
                platform_addr(recipient_byte),
                Some(AddressInfo {
                    address: platform_addr(recipient_byte),
                    nonce: 1,
                    balance: recipient_on_chain,
                }),
            ),
            (
                platform_addr(origin_byte),
                Some(AddressInfo {
                    address: platform_addr(origin_byte),
                    nonce: 1,
                    balance: origin_balance,
                }),
            ),
        ]
        .into_iter()
        .collect();
        sdk.mock()
            .expect_fetch_many::<PlatformAddress, AddressInfo, _, AddressInfos>(
                query,
                Some(response),
            )
            .await
            .expect("set fetch_many expectation");
        let sdk = Arc::new(sdk);

        // --- Wallet manager with a platform account whose pool holds the two
        // known addresses, and whose CACHE is seeded with the DOUBLED recipient
        // balance (the stale/wrong value the planner must ignore).
        let mut wm = WalletManager::<crate::wallet::platform_wallet::PlatformWalletInfo>::new(
            Network::Testnet,
        );
        let wallet_id = wm
            .create_wallet_with_random_mnemonic(WalletAccountCreationOptions::None)
            .expect("create wallet");
        {
            let info = wm.get_wallet_info_mut(&wallet_id).expect("wallet info");
            let base_path = DerivationPath::from(vec![
                ChildNumber::from_hardened_idx(9).expect("purpose"),
                ChildNumber::from_hardened_idx(1).expect("coin type"),
                ChildNumber::from_hardened_idx(17).expect("feature"),
                ChildNumber::from_hardened_idx(0).expect("subfeature"),
                ChildNumber::from_hardened_idx(0).expect("account"),
            ]);
            let mut pool = AddressPool::new_without_generation(
                base_path,
                AddressPoolType::Absent,
                20,
                Network::Testnet,
            );
            pool.addresses.insert(0, pool_entry(recipient_byte, 0));
            pool.addresses.insert(1, pool_entry(origin_byte, 1));

            let mut platform_account = ManagedPlatformAccount::new(ACCOUNT, 0, pool, false);
            // Seed the cache to DISAGREE with the chain: recipient doubled.
            platform_account.set_address_credit_balance(
                PlatformP2PKHAddress::new([recipient_byte; 20]),
                recipient_cached_doubled,
                None,
            );
            platform_account.set_address_credit_balance(
                PlatformP2PKHAddress::new([origin_byte; 20]),
                origin_balance,
                None,
            );
            info.core_wallet
                .accounts
                .insert_platform_account(platform_account);
        }

        // Sanity: the cache really holds the doubled value the planner must
        // ignore.
        {
            let account = wm
                .get_wallet_info(&wallet_id)
                .expect("wallet info")
                .core_wallet
                .platform_payment_managed_account_at_index(ACCOUNT)
                .expect("platform account");
            assert_eq!(
                account.address_credit_balance(&PlatformP2PKHAddress::new([recipient_byte; 20])),
                recipient_cached_doubled,
                "test setup: the cache must hold the doubled recipient balance"
            );
        }

        let wallet_manager = Arc::new(RwLock::new(wm));
        let wallet = build_seam_test_wallet(sdk, wallet_manager, wallet_id);

        // --- Plan against a fixed version, then assert the recipient input is
        // sized from the CHAIN (100M), not the doubled cache (200M).
        let pv = PlatformVersion::latest();
        let plan = wallet
            .plan_withdrawal(ACCOUNT, pv)
            .await
            .expect("plan must succeed against the on-chain balances");

        assert_eq!(
            plan.inputs.get(&platform_addr(recipient_byte)).copied(),
            Some(recipient_on_chain),
            "the recipient input must be sized from the on-chain balance (100M), \
             NOT the doubled cache (200M)"
        );
        assert_ne!(
            plan.inputs.get(&platform_addr(recipient_byte)).copied(),
            Some(recipient_cached_doubled),
            "the doubled cached balance must never leak into the plan (ADDR-04)"
        );

        // The origin is the fee source (largest), so it is planned at
        // balance − fee; it must still be ≤ its on-chain balance.
        let origin_planned = plan
            .inputs
            .get(&platform_addr(origin_byte))
            .copied()
            .expect("origin input present");
        assert!(
            origin_planned <= origin_balance,
            "the fee-source input must not exceed its on-chain balance"
        );
        assert_eq!(
            origin_planned,
            origin_balance - plan.estimated_fee,
            "the fee source keeps exactly the estimated-fee headroom"
        );
    }

    /// Post-relaunch hydration bug: right after a fresh app relaunch the
    /// derived pool (`addresses.addresses`) is EMPTY until a platform sync
    /// repopulates it, but `initialize_from_persisted` has already hydrated
    /// the `address_balances` map from the persisted `platform_addresses`
    /// rows via `set_address_credit_balance(.., None)` — which writes the
    /// balance map but never the pool. A candidate enumeration that read only
    /// the pool saw zero candidates and failed with "No funded addresses
    /// available" even though the balance (and the on-chain funds) were
    /// present. This pins the fix: the candidate SET is the UNION of the pool
    /// and the balance-map keys, so a withdraw works immediately after launch.
    /// (`auto_select_inputs` on the transfer path shares the identical union
    /// enumeration.)
    #[tokio::test]
    async fn plan_withdrawal_finds_candidates_from_balance_map_when_pool_empty() {
        use dash_sdk::query_types::{AddressInfo, AddressInfos};

        const ACCOUNT: u32 = 0;
        let funded_byte = 0x33u8;
        let funded_balance: u64 = dpp::dash_to_credits!(0.05);

        // --- Mock SDK: the candidate SET must be built from the hydrated
        // balance map alone (the pool is empty), so the fetch_many query key
        // is exactly the single funded address.
        let mut sdk = dash_sdk::Sdk::new_mock();
        let query: BTreeSet<PlatformAddress> = [platform_addr(funded_byte)].into_iter().collect();
        let response: AddressInfos = [(
            platform_addr(funded_byte),
            Some(AddressInfo {
                address: platform_addr(funded_byte),
                nonce: 1,
                balance: funded_balance,
            }),
        )]
        .into_iter()
        .collect();
        sdk.mock()
            .expect_fetch_many::<PlatformAddress, AddressInfo, _, AddressInfos>(
                query,
                Some(response),
            )
            .await
            .expect("set fetch_many expectation");
        let sdk = Arc::new(sdk);

        // --- Wallet manager with a platform account whose derived pool is
        // EMPTY (the post-relaunch state) but whose `address_balances` map is
        // hydrated exactly the way `initialize_from_persisted` leaves it.
        let mut wm = WalletManager::<crate::wallet::platform_wallet::PlatformWalletInfo>::new(
            Network::Testnet,
        );
        let wallet_id = wm
            .create_wallet_with_random_mnemonic(WalletAccountCreationOptions::None)
            .expect("create wallet");
        {
            let info = wm.get_wallet_info_mut(&wallet_id).expect("wallet info");
            let base_path = DerivationPath::from(vec![
                ChildNumber::from_hardened_idx(9).expect("purpose"),
                ChildNumber::from_hardened_idx(1).expect("coin type"),
                ChildNumber::from_hardened_idx(17).expect("feature"),
                ChildNumber::from_hardened_idx(0).expect("subfeature"),
                ChildNumber::from_hardened_idx(0).expect("account"),
            ]);
            // Empty pool: no `pool.addresses.insert(..)` — the derived pool
            // has not been repopulated by a sync yet.
            let pool = AddressPool::new_without_generation(
                base_path,
                AddressPoolType::Absent,
                20,
                Network::Testnet,
            );
            let mut platform_account = ManagedPlatformAccount::new(ACCOUNT, 0, pool, false);
            // Hydrate ONLY the balance map (key_source None ⇒ pool untouched),
            // mirroring `initialize_from_persisted`.
            platform_account.set_address_credit_balance(
                PlatformP2PKHAddress::new([funded_byte; 20]),
                funded_balance,
                None,
            );
            info.core_wallet
                .accounts
                .insert_platform_account(platform_account);
        }

        // Sanity: the derived pool really is empty — the bug's precondition.
        {
            let account = wm
                .get_wallet_info(&wallet_id)
                .expect("wallet info")
                .core_wallet
                .platform_payment_managed_account_at_index(ACCOUNT)
                .expect("platform account");
            assert!(
                account.addresses.addresses.is_empty(),
                "test setup: the derived pool must be empty (post-relaunch state)"
            );
        }

        let wallet_manager = Arc::new(RwLock::new(wm));
        let wallet = build_seam_test_wallet(sdk, wallet_manager, wallet_id);

        let pv = PlatformVersion::latest();
        let plan = wallet
            .plan_withdrawal(ACCOUNT, pv)
            .await
            .expect("plan must succeed from the hydrated balance map even with an empty pool");

        // The funded address — discoverable ONLY via the balance-map union —
        // is the sole input, and as the only (largest) input it is the fee
        // source, planned at balance − fee.
        let planned = plan
            .inputs
            .get(&platform_addr(funded_byte))
            .copied()
            .expect("the balance-map-only address must be selected as an input");
        assert_eq!(
            planned,
            funded_balance - plan.estimated_fee,
            "the sole input is the fee source, planned at balance − estimated fee"
        );
    }

    /// Assemble a `PlatformAddressWallet` around a mock SDK + a seeded wallet
    /// manager. `plan_withdrawal` reads only the SDK and the wallet manager
    /// (never the provider), so the provider is left uninitialised. Mirrors the
    /// wiring in `provider::tests::reconcile_address_infos_persists_decremented_balance`.
    fn build_seam_test_wallet(
        sdk: Arc<dash_sdk::Sdk>,
        wallet_manager: Arc<
            RwLock<WalletManager<crate::wallet::platform_wallet::PlatformWalletInfo>>,
        >,
        wallet_id: WalletId,
    ) -> PlatformAddressWallet {
        use crate::broadcaster::SpvBroadcaster;
        use crate::events::PlatformEventManager;
        use crate::spv::SpvRuntime;
        use crate::wallet::asset_lock::manager::AssetLockManager;
        use crate::wallet::persister::{NoPlatformPersistence, WalletPersister};
        use tokio::sync::Notify;

        let persister = WalletPersister::new(wallet_id, Arc::new(NoPlatformPersistence));
        let event_manager = Arc::new(PlatformEventManager::new(Vec::new()));
        let spv = Arc::new(SpvRuntime::new(Arc::clone(&wallet_manager), event_manager));
        let broadcaster = Arc::new(SpvBroadcaster::new(spv));
        let asset_locks = Arc::new(AssetLockManager::new(
            Arc::clone(&sdk),
            Arc::clone(&wallet_manager),
            wallet_id,
            Arc::new(Notify::new()),
            broadcaster,
            persister.clone(),
        ));
        PlatformAddressWallet::new(sdk, wallet_manager, wallet_id, asset_locks, persister)
    }
}
