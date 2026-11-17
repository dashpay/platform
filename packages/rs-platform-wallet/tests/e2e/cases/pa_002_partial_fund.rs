//! PA-002 — Partial-fund + change handling (output < input balance).
//! Spec: `tests/e2e/TEST_SPEC.md` §3 "Platform Addresses (PA)" → PA-002.
//! Priority: P0.
//!
//! Bank funds `addr_1` with [`FUNDING_CREDITS`]; the wallet self-transfers
//! [`TRANSFER_CREDITS`] to a fresh `addr_2`. The auto-selector picks
//! exactly enough input to cover the gross output sum (Σ inputs == Σ
//! outputs) so addr_1 retains the difference as change. With the default
//! `[ReduceOutput(0)]` fee strategy the bank's funding output and the
//! self-transfer's destination output each absorb their respective
//! chain-time fee — assertions below derive both fees from observed
//! balances rather than pinning exact numbers.
//!
//! Gated behind the `e2e` cargo feature so a stock `cargo test -p platform-wallet`
//! (or workspace-wide invocation) stays green for contributors and CI
//! jobs that lack a funded testnet bank wallet, live DAPI access, and
//! the operator `.env`. Operator setup lives in `tests/.env`
//! (template: `tests/.env.example`); a missing
//! `PLATFORM_WALLET_E2E_BANK_MNEMONIC` would otherwise surface as a
//! [`FrameworkError::Bank`](crate::framework::FrameworkError::Bank)
//! during context init, escalated to a panic by `setup().expect(..)`.
//!
//! ```bash
//! cp packages/rs-platform-wallet/tests/.env.example \
//!    packages/rs-platform-wallet/tests/.env
//! # edit tests/.env to set PLATFORM_WALLET_E2E_BANK_MNEMONIC
//! cargo test --test e2e -- --ignored --nocapture
//! ```

use std::collections::BTreeMap;
use std::time::Duration;

use crate::framework::prelude::*;
use crate::framework::wait::{
    wait_for_address_balance_chain_confirmed_n, CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
};

// Sized to dodge platform #3040 — `AddressFundsTransferTransition::
// calculate_min_required_fee` returns the static
// `state_transition_min_fees` floor (~6.5M for 1in/1out) but Drive's
// chain-time fee includes storage + processing costs that scale with
// the operation set (~15M empirically for the same shape). With
// `[ReduceOutput(0)]`, `output[0]` absorbs the fee at chain time;
// if it's smaller than the realistic fee the broadcast fails with
// `AddressesNotEnoughFundsError`. Picking output amounts well above
// the empirical chain-time ceiling sidesteps the bug until #3040
// lands at the dpp layer.

/// Credits the bank delivers to `addr_1`. The bank uses
/// `[DeductFromInput(0)]`, so addr_1 receives this exact amount;
/// the bank's fee is absorbed by the bank's own input. Sized well
/// above the chain-time fee (~15M empirically) so addr_1 has
/// enough headroom for the self-transfer (see #3040 comment above).
const FUNDING_CREDITS: u64 = 100_000_000;

/// Safety floor for the addr_1 wait. Under `[DeductFromInput(0)]`
/// addr_1 receives FUNDING_CREDITS exactly; the floor is kept as a
/// guard against an empty/stale observation slipping through.
const FUNDING_FLOOR: u64 = 70_000_000;

/// Gross credits the test wallet submits in its self-transfer to
/// `addr_2`. Same `[ReduceOutput(0)]` semantics — addr_2 receives
/// `TRANSFER_CREDITS − transfer_fee`. Sized well above the empirical
/// chain-time fee (~15M) to avoid #3040.
const TRANSFER_CREDITS: u64 = 50_000_000;

/// Lower bound on what addr_2 must receive before the assertions
/// run. A non-zero floor prevents an empty observation from passing
/// the wait.
const TRANSFER_FLOOR: u64 = 1_000_000;

/// Upper bound on the chain-time fee for a 1in/1out transition. Empirical
/// fee at write-time is ~15M credits (per platform #3040's static-vs-
/// chain-time gap analysis); pinning the regression-guard ceiling at 25M
/// leaves room for protocol-version drift while still surfacing a fee-
/// explosion regression. A failure means either (a) the protocol's fee
/// schedule shifted significantly (update this constant deliberately) or
/// (b) a wallet-side or dpp-side regression is over-charging.
const TRANSFER_FEE_CEILING: u64 = 25_000_000;

/// Per-step deadline for balance observations.
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

#[tokio_shared_rt::test(shared)]
async fn pa_002_partial_fund_change() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    let s = setup().await.expect("e2e setup failed");

    // The hand-out reserves `addr_1` immediately. Funding then promotes
    // that reservation to Used before the partial-fund transfer.
    let addr_1 = s
        .test_wallet
        .next_unused_address()
        .await
        .expect("derive addr_1");

    s.ctx
        .bank()
        .fund_address(&addr_1, FUNDING_CREDITS)
        .await
        .expect("bank.fund_address");

    // Bank uses `[DeductFromInput(0)]`: addr_1 receives FUNDING_CREDITS
    // exactly. Gate on the proof-verified chain view (Found-025-immune):
    // a stale local-map 0 would hang this before the self-transfer that
    // consumes addr_1. The exact-amount assertion follows the sync.
    wait_for_address_balance_chain_confirmed_n(
        s.ctx.sdk(),
        &addr_1,
        FUNDING_FLOOR,
        CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
        STEP_TIMEOUT,
    )
    .await
    .expect("addr_1 funding never observed");

    // Chain-confirmed gate is sdk-only and never warms the wallet's local
    // balance map; refresh it before the self-transfer consumes addr_1.
    s.test_wallet.sync_balances().await.expect("pre-tx sync");

    let addr_2 = s
        .test_wallet
        .next_unused_address()
        .await
        .expect("derive addr_2");
    assert_ne!(
        addr_1, addr_2,
        "wallet must hand out a fresh address after addr_1 is reserved and funded"
    );

    let outputs: BTreeMap<_, _> = std::iter::once((addr_2, TRANSFER_CREDITS)).collect();
    s.test_wallet
        .transfer(outputs)
        .await
        .expect("self-transfer");

    // addr_2 receives `TRANSFER_CREDITS − transfer_fee` (also
    // `[ReduceOutput(0)]`). Wait on the post-fee floor.
    wait_for_balance(&s.test_wallet, &addr_2, TRANSFER_FLOOR, STEP_TIMEOUT)
        .await
        .expect("addr_2 transfer never observed");

    // Re-sync test wallet so the cached view reflects post-transfer
    // state across BOTH addresses.
    s.test_wallet
        .sync_balances()
        .await
        .expect("post-transfer sync");
    let balances = s.test_wallet.balances().await;
    let received = balances.get(&addr_2).copied().unwrap_or(0);
    let remaining = balances.get(&addr_1).copied().unwrap_or(0);
    // The transfer fee is the share TRANSFER_CREDITS lost while
    // crossing addr_1 -> addr_2 via `[ReduceOutput(0)]`.
    let transfer_fee = TRANSFER_CREDITS.saturating_sub(received);

    // The bank's funding fee is NOT directly observable from the test
    // wallet — under `[DeductFromInput(0)]` the recipient receives
    // exactly `FUNDING_CREDITS` and the bank's input absorbs the fee
    // privately. A pre/post `bank.total_credits()` snapshot would in
    // principle reveal the delta, but the bank is process-shared:
    // sibling tests funding or receiving sweep transitions during this
    // test's window pollute the delta in a parallel run
    // (`--test-threads>1`). The bank_fee invariant is enforced
    // implicitly by the bank-load balance check at framework init; we
    // don't re-assert it here. PA-004's module docs document the same
    // constraint.

    tracing::info!(
        target: "platform_wallet::e2e::cases::pa_002",
        ?addr_1,
        ?addr_2,
        funded = FUNDING_CREDITS,
        received,
        remaining,
        transfer_fee,
        "post-transfer balance snapshot"
    );

    // PA-002 asserts: addr_1 retains the difference (Σ inputs ==
    // Σ outputs invariant — the property fixed in `aaf8be74ee` and
    // `9ea9e7033c`). Under [ReduceOutput(0)], the protocol deducts the
    // transfer fee from output[0] — addr_2's received amount — not
    // from addr_1's residual. So addr_1 retains
    // FUNDING_CREDITS - TRANSFER_CREDITS and addr_2 receives
    // TRANSFER_CREDITS - transfer_fee.
    assert!(
        received >= TRANSFER_FLOOR,
        "addr_2 must hold at least TRANSFER_FLOOR ({TRANSFER_FLOOR}); observed {received}"
    );
    assert_eq!(
        remaining,
        FUNDING_CREDITS - TRANSFER_CREDITS,
        "addr_1 must retain FUNDING_CREDITS - TRANSFER_CREDITS \
         (transfer_fee is deducted from addr_2's amount, not addr_1's residual). \
         observed remaining={remaining} expected={}",
        FUNDING_CREDITS - TRANSFER_CREDITS,
    );
    assert_eq!(
        received,
        TRANSFER_CREDITS - transfer_fee,
        "addr_2 must receive TRANSFER_CREDITS minus the transfer fee \
         (ReduceOutput(0) deducts fee from the transferred amount). \
         observed received={received} expected={}",
        TRANSFER_CREDITS - transfer_fee,
    );
    assert!(
        transfer_fee > 0,
        "self-transfer must charge a non-zero fee (received={received})"
    );
    assert!(
        transfer_fee < TRANSFER_FEE_CEILING,
        "self-transfer fee {transfer_fee} exceeds the regression-guard ceiling \
         {TRANSFER_FEE_CEILING} — protocol fee shift or fee-explosion regression"
    );
    // Σ inputs == Σ outputs (test-wallet view): addr_1 retained exactly
    // `FUNDING_CREDITS − TRANSFER_CREDITS`. Under `[DeductFromInput(0)]`
    // the bank delivers FUNDING_CREDITS in full to addr_1; the
    // self-transfer's `[ReduceOutput(0)]` then deducts TRANSFER_CREDITS
    // from addr_1 (no change to the bank-side fee, which is private).
    // This pin is the strongest parallel-safe form of the original Σ
    // invariant — it doesn't require observing the bank's balance.
    let expected_change = FUNDING_CREDITS - TRANSFER_CREDITS;
    assert_eq!(
        remaining, expected_change,
        "addr_1 change must equal `FUNDING_CREDITS − TRANSFER_CREDITS` \
         under DeductFromInput(0)+ReduceOutput(0) (test-wallet view); \
         expected {expected_change}, got {remaining}"
    );

    s.teardown().await.expect("teardown");
}
