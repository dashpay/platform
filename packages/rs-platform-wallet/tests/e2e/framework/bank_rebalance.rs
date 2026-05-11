//! The operator maintains exactly one address balance: the bank's
//! Platform address (`tdash1kzz…` via
//! [`BankWallet::primary_receive_address`]). The harness auto-rebalances
//! Platform-to-Core internally as needed. Everything else is
//! implementation detail.
//!
//! Two helpers preserve this invariant at suite start:
//!
//! 1. [`drain_bank_identity_to_addresses`] — any credits accumulated on
//!    the bank identity (legacy + transient mid-run sinks) are moved
//!    back to the Platform address via the fast Platform-only
//!    `transfer_credits_to_addresses_with_external_signer` primitive.
//!
//! 2. [`refill_core_from_platform_if_below_threshold`] — if the bank's
//!    L1 Core balance is below the configured threshold, refill it from
//!    the Platform address via a (slow) Platform→Core withdrawal,
//!    chained `top_up_from_addresses` → `withdraw_credits_with_external_signer`.
//!    Gated by the threshold so the slow path runs only when needed.
//!
//! After this refactor lands, the operator can ignore the Core address
//! and the bank identity. Only the Platform address needs external
//! top-ups.

use std::collections::BTreeMap;
use std::time::Duration;

use dash_sdk::platform::Fetch;
use dash_sdk::query_types::IdentityBalance;
use dpp::fee::Credits;

use super::bank::BankWallet;
use super::bank_identity::BankIdentity;
use super::wait::wait_for_identity_balance;
use super::FrameworkResult;

/// Headroom kept on the bank identity after a Platform-side drain so a
/// follow-up `transfer_credits_to_addresses` (or core-refill chain) has
/// budget for its on-chain fee. Mirrors `IDENTITY_SWEEP_FEE_RESERVE` in
/// [`super::cleanup`] — empirically ~12-15M on testnet, 30M is generous.
const BANK_IDENTITY_DRAIN_FEE_RESERVE: Credits = 30_000_000;

/// 1 Core duff = 1000 Platform credits. Used by the core-refill chain to
/// translate duff thresholds / targets into credit-denominated transition
/// amounts.
const CREDITS_PER_DUFF: u64 = 1_000;

/// Default trip line for the core-refill fallback. Below this many duffs
/// of confirmed Core balance the harness rebalances Platform→Core at
/// suite start so CR-* / ID-007 cases have working capital. Overrideable
/// via [`super::config::vars::CORE_REFILL_THRESHOLD_DUFF`].
pub const DEFAULT_CORE_REFILL_THRESHOLD_DUFF: u64 = 100_000;

/// Default target balance (duffs) the core-refill chain aims to reach
/// when triggered. Overrideable via
/// [`super::config::vars::CORE_REFILL_TARGET_DUFF`].
pub const DEFAULT_CORE_REFILL_TARGET_DUFF: u64 = 1_000_000;

/// Identity-side fee reserve added on top of the desired core-refill
/// credit amount when topping up the bank identity. The withdrawal that
/// follows the top-up pays its own protocol fee out of the identity's
/// balance, so the top-up must overshoot the withdrawal target by at
/// least this much.
const CORE_REFILL_IDENTITY_FEE_RESERVE: Credits = 50_000_000;

/// Deadline for the post-top-up identity-balance visibility wait inside
/// [`refill_core_from_platform_if_below_threshold`]. Sized like the
/// bank-identity bootstrap path — generous, because the helper runs once
/// per suite.
const CORE_REFILL_TOPUP_TIMEOUT: Duration = Duration::from_secs(60);

/// Drain the bank identity's Platform credits back to
/// [`BankWallet::primary_receive_address`] via the fast Platform-only
/// `transfer_credits_to_addresses_with_external_signer` primitive.
///
/// Leaves [`BANK_IDENTITY_DRAIN_FEE_RESERVE`] on the identity to cover
/// the transfer fee plus a small headroom for any follow-up cost (e.g.
/// the core-refill chain firing immediately afterwards). No-op when the
/// bank identity's balance is at or below that reserve.
///
/// Returns the amount drained (0 if no-op). Best-effort: failures are
/// logged at WARN and surfaced to the caller for context — the harness
/// init path treats them as non-fatal.
pub async fn drain_bank_identity_to_addresses(
    bank: &BankWallet,
    bank_identity: &BankIdentity,
) -> FrameworkResult<Credits> {
    let bank_wallet = bank.platform_wallet();
    let sdk = bank_wallet.sdk();

    // Ensure the bank identity is loaded into the bank wallet's
    // IdentityManager — `transfer_credits_to_addresses_with_external_signer`
    // looks it up there. On the persisted-id load path the manager
    // would otherwise be empty for the bank slot.
    if let Err(err) = bank_wallet
        .identity()
        .load_identity_by_index(bank_identity.identity_index)
        .await
    {
        tracing::warn!(
            target: "platform_wallet::e2e::bank_rebalance",
            bank_identity_id = %bank_identity.id,
            identity_index = bank_identity.identity_index,
            error = %err,
            "drain skipped: failed to load bank identity into manager"
        );
        return Ok(0);
    }

    // Authoritative balance comes from chain, not from the local cache,
    // for the same reason as `cleanup::sweep_identities_with_seed` — a
    // stale cache flips this helper between "no-op" and "over-amount
    // transfer that the chain rejects".
    let pre: Credits = match IdentityBalance::fetch(sdk, bank_identity.id).await {
        Ok(Some(b)) => b,
        Ok(None) => {
            tracing::warn!(
                target: "platform_wallet::e2e::bank_rebalance",
                bank_identity_id = %bank_identity.id,
                "drain skipped: chain reports bank identity absent"
            );
            return Ok(0);
        }
        Err(err) => {
            tracing::warn!(
                target: "platform_wallet::e2e::bank_rebalance",
                bank_identity_id = %bank_identity.id,
                error = %err,
                "drain skipped: bank identity balance refresh failed"
            );
            return Ok(0);
        }
    };

    if pre <= BANK_IDENTITY_DRAIN_FEE_RESERVE {
        tracing::debug!(
            target: "platform_wallet::e2e::bank_rebalance",
            bank_identity_id = %bank_identity.id,
            pre,
            reserve = BANK_IDENTITY_DRAIN_FEE_RESERVE,
            "drain no-op: bank identity at or below fee reserve"
        );
        return Ok(0);
    }

    let amount = pre - BANK_IDENTITY_DRAIN_FEE_RESERVE;
    let outputs: BTreeMap<_, _> =
        std::iter::once((*bank.primary_receive_address(), amount)).collect();

    match bank_wallet
        .identity()
        .transfer_credits_to_addresses_with_external_signer(
            &bank_identity.id,
            outputs,
            bank_identity.signer.as_ref(),
            None,
        )
        .await
    {
        Ok(post) => {
            tracing::info!(
                target: "platform_wallet::e2e::bank_rebalance",
                bank_identity_id = %bank_identity.id,
                pre,
                post,
                drained = amount,
                "drained bank identity credits back to bank Platform address"
            );
            Ok(amount)
        }
        Err(err) => {
            tracing::warn!(
                target: "platform_wallet::e2e::bank_rebalance",
                bank_identity_id = %bank_identity.id,
                pre,
                attempted = amount,
                error = %err,
                "bank identity drain broadcast failed; continuing without drain"
            );
            Ok(0)
        }
    }
}

/// Refill the bank's Core (Layer-1) confirmed balance from the Platform
/// address pool when it dips below `threshold_duff`, targeting
/// `target_duff` afterwards. Best-effort; never fails harness init.
///
/// Chain: `top_up_from_addresses` (bank address → bank identity)
/// followed by `withdraw_credits_with_external_signer` (bank identity →
/// bank Core address). The withdrawal is the canonical slow path — it
/// rides the Core withdrawal pool — so it's gated behind the
/// `threshold_duff` check to avoid the cost on every run.
///
/// Returns the duff amount the withdrawal was issued for (0 when the
/// helper short-circuited because the balance was above the threshold or
/// because any sub-step failed).
pub async fn refill_core_from_platform_if_below_threshold(
    bank: &BankWallet,
    bank_identity: &BankIdentity,
    threshold_duff: u64,
    target_duff: u64,
) -> FrameworkResult<u64> {
    if target_duff <= threshold_duff {
        tracing::warn!(
            target: "platform_wallet::e2e::bank_rebalance",
            threshold_duff,
            target_duff,
            "core-refill skipped: misconfigured (target must exceed threshold)"
        );
        return Ok(0);
    }

    let core_balance = bank.core_balance_confirmed();
    if core_balance >= threshold_duff {
        tracing::debug!(
            target: "platform_wallet::e2e::bank_rebalance",
            core_balance,
            threshold_duff,
            "core-refill no-op: Core balance above threshold"
        );
        return Ok(0);
    }

    let bank_wallet = bank.platform_wallet();

    // Ensure the bank identity is loaded into the manager — both the
    // top-up and the withdrawal look it up there.
    if let Err(err) = bank_wallet
        .identity()
        .load_identity_by_index(bank_identity.identity_index)
        .await
    {
        tracing::warn!(
            target: "platform_wallet::e2e::bank_rebalance",
            bank_identity_id = %bank_identity.id,
            error = %err,
            "core-refill skipped: failed to load bank identity into manager"
        );
        return Ok(0);
    }

    let withdraw_credits: Credits = target_duff.saturating_mul(CREDITS_PER_DUFF);
    let topup_credits: Credits = withdraw_credits.saturating_add(CORE_REFILL_IDENTITY_FEE_RESERVE);

    let inputs: BTreeMap<_, _> =
        std::iter::once((*bank.primary_receive_address(), topup_credits)).collect();
    let identity_balance_after_topup = match bank_wallet
        .identity()
        .top_up_from_addresses(&bank_identity.id, inputs, bank.address_signer(), None)
        .await
    {
        Ok(new_balance) => new_balance,
        Err(err) => {
            tracing::warn!(
                target: "platform_wallet::e2e::bank_rebalance",
                bank_identity_id = %bank_identity.id,
                topup_credits,
                error = %err,
                "core-refill skipped: top_up_from_addresses failed"
            );
            return Ok(0);
        }
    };

    // Wait for the new identity balance to be visible on chain before
    // issuing the withdrawal — the SDK's `WithdrawFromIdentity` rebuilds
    // the transition from `Identity::fetch`, so a stale view rejects
    // with `InsufficientIdentityBalance`.
    if let Err(err) = wait_for_identity_balance(
        bank_wallet.sdk(),
        bank_identity.id,
        identity_balance_after_topup,
        CORE_REFILL_TOPUP_TIMEOUT,
    )
    .await
    {
        tracing::warn!(
            target: "platform_wallet::e2e::bank_rebalance",
            bank_identity_id = %bank_identity.id,
            expected = identity_balance_after_topup,
            error = %err,
            "core-refill skipped: post-top-up identity balance never \
             converged; abandoning withdrawal"
        );
        return Ok(0);
    }

    let core_addr = match bank.primary_core_receive_address().await {
        Ok(addr) => addr,
        Err(err) => {
            tracing::warn!(
                target: "platform_wallet::e2e::bank_rebalance",
                error = %err,
                "core-refill skipped: bank Core receive-address resolution failed"
            );
            return Ok(0);
        }
    };

    match bank_wallet
        .identity()
        .withdraw_credits_with_external_signer(
            &bank_identity.id,
            withdraw_credits,
            &core_addr,
            bank_identity.signer.as_ref(),
            None,
        )
        .await
    {
        Ok(()) => {
            tracing::info!(
                target: "platform_wallet::e2e::bank_rebalance",
                bank_identity_id = %bank_identity.id,
                core_balance_before = core_balance,
                target_duff,
                withdrew_credits = withdraw_credits,
                bank_core_addr = %core_addr,
                "Platform→Core refill issued (will settle through the Core \
                 withdrawal pool; the bank's confirmed balance updates after \
                 SPV observes the unlock)"
            );
            Ok(target_duff)
        }
        Err(err) => {
            tracing::warn!(
                target: "platform_wallet::e2e::bank_rebalance",
                bank_identity_id = %bank_identity.id,
                withdraw_credits,
                error = %err,
                "core-refill withdrawal failed; Core balance unchanged"
            );
            Ok(0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pin the CREDITS_PER_DUFF constant — getting this wrong silently
    /// converts 1 DASH into either 1000 DASH or 0.001 DASH downstream.
    #[test]
    fn credits_per_duff_is_one_thousand() {
        assert_eq!(CREDITS_PER_DUFF, 1_000);
    }

    /// 1 duff round-trips through the duff→credits cast used by the
    /// core-refill helper at the 1000x ratio. Mirrors what
    /// `refill_core_from_platform_if_below_threshold` does to compute
    /// `withdraw_credits` from `target_duff`.
    #[test]
    fn duff_to_credits_conversion_round_trips() {
        let duff: u64 = 1;
        let credits: Credits = duff.saturating_mul(CREDITS_PER_DUFF);
        assert_eq!(credits, 1_000);
        let duff_back: u64 = (credits / CREDITS_PER_DUFF) as u64;
        assert_eq!(duff_back, duff);
    }

    /// Misconfigured (target ≤ threshold) is caught before any chain
    /// contact — pinned as a guard so a future "swap the args" edit
    /// can't silently waste a slow withdrawal.
    #[test]
    fn refill_misconfig_target_must_exceed_threshold() {
        let threshold = DEFAULT_CORE_REFILL_THRESHOLD_DUFF;
        let target = DEFAULT_CORE_REFILL_TARGET_DUFF;
        assert!(
            target > threshold,
            "defaults must obey target > threshold (target={target} threshold={threshold})"
        );
    }
}
