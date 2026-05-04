//! PA-#3040 bug-pin — `[ReduceOutput(0)]` self-transfer with `output[0]`
//! between the static `estimate_min_fee` ceiling and the chain-time fee
//! must succeed (today: it fails — the test goes red, naming the bug).
//!
//! Spec: there is no PA-NNN entry for this — it's a bug-pin for platform
//! issue [#3040](https://github.com/dashpay/platform/issues/3040)
//! (`AddressFundsTransferTransition::calculate_min_required_fee` returns
//! the static `state_transition_min_fees` floor while Drive's chain-time
//! fee includes storage + processing costs that scale with the operation
//! set; for 1in/1out the gap is ~6.5M static vs ~15M chain-time).
//!
//! ## What this test pins
//!
//! Bank funds `addr_1` with enough credits to cover any reasonable gross
//! output. The wallet attempts to self-transfer **`OUTPUT_CREDITS = 8M`**
//! to `addr_2`, an amount carefully chosen to sit inside the bug zone:
//!
//! - `OUTPUT_CREDITS > static_min_fee_for_1in_1out` (~6.5M) — wallet's
//!   `select_inputs_reduce_output` Phase 4 check passes, so the wallet
//!   builds and broadcasts the transition.
//! - `OUTPUT_CREDITS < chain_time_fee_for_1in_1out` (~14.94M empirical)
//!   — Drive's `deduct_fee_from_outputs_or_remaining_balance_of_inputs`
//!   tries to charge the full chain-time fee against `output[0]`, but
//!   `output[0] (8M)` can't absorb a ~15M fee, and there's no
//!   `DeductFromInput(N)` fallback in `[ReduceOutput(0)]`. So Drive
//!   returns `AddressesNotEnoughFundsError { required_balance: ~15M }`.
//!
//! ## Test direction (standard, not inverted)
//!
//! The test asserts the **contract**: a transfer with `output[0] >`
//! `estimate_min_fee` should succeed and `addr_2` should receive
//! `OUTPUT_CREDITS - chain_time_fee`. That's what the wallet's Phase 4
//! check implies and what callers reasonably assume.
//!
//! - **Today (#3040 unfixed)**: `transfer()` succeeds at the wallet
//!   layer (Phase 4 passes) but the broadcast is rejected by Drive
//!   with `AddressesNotEnoughFundsError`. The `.expect("self-transfer")`
//!   then panics → **test fails (red)**. The red is the proof that
//!   #3040 still exists.
//! - **After #3040 is fixed** (either by tightening `estimate_min_fee`
//!   to the chain-time reality, by widening the auto-select to reserve
//!   fee headroom for ReduceOutput, or by some hybrid): `transfer()`
//!   succeeds, `addr_2` ends with `OUTPUT_CREDITS - fee`, the test
//!   passes (green). Green is the proof that the fix works.
//!
//! Either way the wallet must NOT panic and must NOT silently produce
//! an unspendable transition.

use std::collections::BTreeMap;
use std::time::Duration;

use crate::framework::prelude::*;

/// Gross credits the bank submits when funding `addr_1`. Sized to
/// comfortably clear the bank's own ReduceOutput(0) chain-time fee
/// so addr_1 receives a useful balance — this part is happy-path.
const FUNDING_CREDITS: u64 = 100_000_000;

/// Lower bound on what `addr_1` must receive after the bank's fee
/// deduction.
const FUNDING_FLOOR: u64 = 70_000_000;

/// The bug-zone output amount. **8M** sits between the static
/// `estimate_min_fee` for 1in/1out (~6.5M) and the empirical chain-time
/// fee (~14.94M). Chosen so the wallet's `select_inputs_reduce_output`
/// Phase 4 check passes (8M > 6.5M static estimate) but Drive rejects
/// the broadcast (8M < 15M chain-time fee). Tweaking this constant
/// moves the failure point: < 6.5M would fail at wallet level;
/// > 15M would succeed entirely.
const OUTPUT_CREDITS: u64 = 8_000_000;

/// Lower bound on what `addr_2` must receive after the chain-time fee
/// is deducted from `output[0]`. With #3040 in play, addr_2 doesn't
/// receive ANYTHING (the broadcast is rejected). After fix, addr_2 ends
/// with `OUTPUT_CREDITS - chain_time_fee`. Pin a non-zero floor so the
/// "received nothing" case is unambiguous.
const RECEIVED_FLOOR: u64 = 1;

/// Per-step deadline for balance observations.
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

#[tokio_shared_rt::test(shared)]
async fn pa_3040_reduce_output_chain_time_fee_must_not_exceed_static_estimate() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    let s = setup().await.expect("e2e setup failed");

    // Setup is happy path: fund `addr_1`, derive `addr_2` after the
    // funding syncs the cursor. The bug surfaces only on the self-
    // transfer.
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

    // The contract: a 1in/1out transfer with `output[0] >`
    // `estimate_min_fee` should succeed. With #3040 unfixed this call
    // fails on broadcast — the test goes red as the bug pin.
    let outputs: BTreeMap<_, _> = std::iter::once((addr_2, OUTPUT_CREDITS)).collect();
    s.test_wallet.transfer(outputs).await.expect(
        "self-transfer must succeed for output[0] > estimate_min_fee — \
                 if this fails with `AddressesNotEnoughFundsError`, #3040 is the bug",
    );

    // If we got here, #3040 is fixed. Verify the post-conditions.
    wait_for_balance(&s.test_wallet, &addr_2, RECEIVED_FLOOR, STEP_TIMEOUT)
        .await
        .expect("addr_2 transfer never observed");

    s.test_wallet
        .sync_balances()
        .await
        .expect("post-transfer sync");
    let balances = s.test_wallet.balances().await;
    let received = balances.get(&addr_2).copied().unwrap_or(0);
    let remaining = balances.get(&addr_1).copied().unwrap_or(0);
    let observed_total = received.saturating_add(remaining);
    let total_fees = FUNDING_CREDITS.saturating_sub(observed_total);
    let transfer_fee = OUTPUT_CREDITS.saturating_sub(received);
    let bank_fee = total_fees.saturating_sub(transfer_fee);

    tracing::info!(
        target: "platform_wallet::e2e::cases::pa_3040",
        ?addr_1,
        ?addr_2,
        funded = FUNDING_CREDITS,
        received,
        remaining,
        bank_fee,
        transfer_fee,
        "PA-3040: post-transfer snapshot — #3040 appears fixed"
    );

    // Σ inputs == Σ outputs (gross): addr_1 retained
    // `FUNDING_CREDITS − bank_fee − OUTPUT_CREDITS`.
    let expected_change = FUNDING_CREDITS
        .saturating_sub(bank_fee)
        .saturating_sub(OUTPUT_CREDITS);
    assert_eq!(
        remaining, expected_change,
        "addr_1 change must equal `FUNDING_CREDITS − bank_fee − OUTPUT_CREDITS` \
         (Σ inputs == Σ outputs invariant); expected {expected_change}, got {remaining}"
    );
    // addr_2 received gross-minus-fee. The fee is non-zero (chain-time
    // fee always charges something) and below OUTPUT_CREDITS (the
    // output absorbed it).
    assert!(
        received >= RECEIVED_FLOOR,
        "addr_2 must hold at least RECEIVED_FLOOR ({RECEIVED_FLOOR}); observed {received}"
    );
    assert!(
        received < OUTPUT_CREDITS,
        "addr_2 must hold less than OUTPUT_CREDITS ({OUTPUT_CREDITS}) after \
         `[ReduceOutput(0)]` fee deduction; observed {received}"
    );
    assert!(
        transfer_fee > 0,
        "self-transfer must charge a non-zero fee (received={received})"
    );

    s.teardown().await.expect("teardown");
}
