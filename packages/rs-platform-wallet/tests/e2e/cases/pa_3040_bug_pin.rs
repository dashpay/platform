//! PA-#3040 bug-pin / workaround regression guard — a `[DeductFromInput(0)]`
//! self-transfer must clear Drive's chain-time fee even though the static
//! protocol estimate under-states it (platform issue [#3040]).
//!
//! Spec: there is no PA-NNN entry for this — it pins platform issue
//! [#3040](https://github.com/dashpay/platform/issues/3040)
//! (`AddressFundsTransferTransition::calculate_min_required_fee` returns the
//! static `state_transition_min_fees` floor — ~6.5M for 1in/1out — while
//! Drive's chain-time fee includes storage + processing costs that scale with
//! the operation set, ~15.08M on paloma).
//!
//! ## The bug (#3040)
//!
//! The protocol's Phase-4 estimated-fee validator blesses a transition whose
//! fee-paying balance only covers the static ~6.5M estimate. Drive then
//! charges the higher chain-time fee (~15.08M) and rejects with
//! `AddressesNotEnoughFundsError`. The wallet faithfully passed the protocol
//! check, so a real user is blocked by a transition the protocol said was fine.
//!
//! ## Where the headroom lives (this is what the test guards)
//!
//! `[DeductFromInput(0)]` draws the fee from the fee target's *remaining*
//! input balance (its balance minus what the selection consumes from it). The
//! wallet sizes that remainder with the protocol's own `estimate_min_fee`; it
//! adds no client-side multiplier, because the estimate's owner is `rs-dpp` /
//! `rs-platform-version` and a wallet-side fudge would silently diverge once
//! #3040 is fixed there. The extra headroom Drive's chain-time fee needs is
//! supplied by THIS harness instead: `FUNDING_CREDITS` leaves `addr_1` holding
//! far more than `OUTPUT_CREDITS + chain-time fee`, so the fee target's
//! remainder after consumption clears Drive's charge.
//!
//! This test therefore goes RED if the funding headroom stops covering the
//! chain-time fee (bump `FUNDING_CREDITS`), or if the wallet starts consuming
//! the fee target's balance down to the static estimate. It stays a #3040 pin
//! until the protocol estimate matches Drive's charge.
//!
//! Note: the `[ReduceOutput(0)]` strategy has NO equivalent lever — its fee is
//! drawn from the caller-fixed output, so an output smaller than the
//! chain-time fee can never succeed. This test drives only the
//! `[DeductFromInput(0)]` path.
//!
//! TODO(paloma-quorum): live paloma validation of this test is currently
//! blocked by a transient devnet quorum-retirement gap — `setup()` fails in
//! identity discovery with "Quorum not found for type 107" (rust-dashcore#800),
//! before any transfer runs. The wallet's fee-headroom selection itself is
//! covered by the `select_inputs_deduct_from_input` unit tests
//! (`fee_headroom_violation_errors`,
//! `non_fee_target_below_min_input_redistributes`,
//! `fee_recompute_after_residue_fold_succeeds`). Re-run this case on paloma once
//! the quorum service catches up to confirm the funded headroom clears
//! chain-time.

use std::collections::BTreeMap;
use std::time::Duration;

use crate::framework::prelude::*;

/// Gross credits the bank submits when funding `addr_1`. Sized to comfortably
/// clear the bank's own chain-time fee AND leave `addr_1` holding well above
/// `OUTPUT_CREDITS + chain-time fee` — this is the harness-owned #3040
/// headroom the self-transfer's fee is drawn from.
const FUNDING_CREDITS: u64 = 100_000_000;

/// Lower bound on what `addr_1` must receive after the bank's fee deduction.
/// Must exceed `OUTPUT_CREDITS + ~15.08M` (Drive's observed 1in/1out chain-time
/// fee, ~2.3x the ~6.5M static estimate) with margin, so the fee target's
/// remainder covers the chain-time fee.
const FUNDING_FLOOR: u64 = 60_000_000;

/// The self-transfer output. Under `[DeductFromInput(0)]` the recipient
/// receives this amount EXACTLY (the fee comes from the input's change), so
/// `addr_2` must end with precisely `OUTPUT_CREDITS`.
const OUTPUT_CREDITS: u64 = 10_000_000;

/// Per-step deadline for balance observations.
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

#[tokio_shared_rt::test(shared)]
async fn pa_3040_deduct_from_input_clears_chain_time_fee_with_funded_headroom() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    let s = setup().await.expect("e2e setup failed");

    // Setup is happy path: fund `addr_1`, derive `addr_2` after the funding
    // syncs the cursor.
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
    wait_for_balance(&s.test_wallet, &addr_1, FUNDING_FLOOR, STEP_TIMEOUT)
        .await
        .expect("addr_1 funding never observed");

    let addr_2 = s
        .test_wallet
        .next_unused_address()
        .await
        .expect("derive addr_2");

    // Refresh the local balance map so the auto-selector sees addr_1's funded
    // balance (the funding gate is proof-verified chain state, not the cache).
    s.test_wallet
        .sync_balances()
        .await
        .expect("pre-transfer sync");

    // Under test: a `[DeductFromInput(0)]` self-transfer. The wallet consumes
    // only `OUTPUT_CREDITS` from `addr_1`, so the fee is drawn from the
    // harness-funded remainder, which covers Drive's higher chain-time fee.
    // If the remainder fell to the ~6.5M static estimate, Drive would reject
    // with `AddressesNotEnoughFundsError` — the #3040 red.
    let outputs: BTreeMap<_, _> = std::iter::once((addr_2, OUTPUT_CREDITS)).collect();
    s.test_wallet
        .transfer_deduct_from_input(outputs)
        .await
        .expect(
            "DeductFromInput(0) self-transfer must clear Drive's chain-time fee from the funded \
             remainder — if this fails with `AddressesNotEnoughFundsError`, FUNDING_CREDITS no \
             longer covers the chain-time fee (bump it) or #3040 has regressed",
        );

    wait_for_balance(&s.test_wallet, &addr_2, OUTPUT_CREDITS, STEP_TIMEOUT)
        .await
        .expect("addr_2 transfer never observed");

    s.test_wallet
        .sync_balances()
        .await
        .expect("post-transfer sync");
    let balances = s.test_wallet.balances().await;
    let received = balances.get(&addr_2).copied().unwrap_or(0);
    let remaining = balances.get(&addr_1).copied().unwrap_or(0);

    tracing::info!(
        target: "platform_wallet::e2e::cases::pa_3040",
        ?addr_1,
        ?addr_2,
        funded = FUNDING_CREDITS,
        received,
        remaining,
        "PA-3040: post-transfer snapshot — funded headroom cleared chain-time fee"
    );

    // Under `[DeductFromInput(0)]` the recipient receives the EXACT output —
    // the fee is charged to addr_1's change, not the output. This is the proof
    // the transition committed (the #3040 red would leave addr_2 at 0).
    assert_eq!(
        received, OUTPUT_CREDITS,
        "addr_2 must receive the exact OUTPUT_CREDITS ({OUTPUT_CREDITS}) under \
         [DeductFromInput(0)]; observed {received}. A 0 here means the broadcast was rejected \
         (the #3040 chain-time-fee failure the funded headroom is meant to clear)."
    );
    // The chain-time fee was charged to addr_1 (its remaining is below
    // funding − output), and it was non-zero — Drive always charges something.
    assert!(
        remaining < FUNDING_CREDITS.saturating_sub(OUTPUT_CREDITS),
        "addr_1 must have paid a chain-time fee from its change; remaining {remaining} should be \
         below FUNDING_CREDITS − OUTPUT_CREDITS ({})",
        FUNDING_CREDITS.saturating_sub(OUTPUT_CREDITS)
    );

    s.teardown().await.expect("teardown");
}
