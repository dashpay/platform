//! PA-003 — Fee scaling: one-output vs. five-output transfers.
//! Spec: `tests/e2e/TEST_SPEC.md` §3 "Platform Addresses (PA)" → PA-003.
//! Priority: P1.
//!
//! Encodes fee scaling as an asserted property rather than a magic
//! number. From a single funded source address `addr_src` the wallet
//! issues two self-transfers, both drawing their inputs **exclusively
//! from `addr_src`** (`InputSelection::Explicit`):
//!   1. One destination output → record `fee_1`.
//!   2. Five destination outputs → record `fee_5`.
//!
//! `fee_N` is the **real chain-time fee** the broadcast transition
//! actually paid: under `[ReduceOutput(0)]` the encoded transition
//! balances pre-fee (`Σ inputs == Σ outputs`) and Drive charges the
//! entire chain-time fee against `output[0]` at execution. The only
//! credits the wallet loses are that fee, so
//! `real_fee = Σ gross outputs − Σ(destination balance deltas)`
//! (the canonical Dash `Σ inputs − Σ outputs`). This is the same
//! accounting PA-001 uses for `multi_fee`, applied symmetrically to
//! both shapes.
//!
//! Both transfers select the *same single input address* and the
//! *same per-output gross*. Every measured destination is pre-markered
//! (a small prior transfer establishes its address-funds record)
//! BEFORE its measured transfer, so both the 1-output and 5-output
//! measured transfers hit address-funds **UPDATE** storage ops — never
//! a one-off CREATE on the first credit to a virgin address. Output
//! count is therefore the genuine sole varied factor. The 5-output
//! transition serializes four extra P2PKH outputs (~28 bytes each)
//! plus four extra output-storage UPDATE operations, so its chain-time
//! storage+processing cost is strictly higher than the 1-output one.
//! We assert `fee_5 > fee_1` and an explicit sub-linear ceiling (the
//! four extra outputs share the input bytes, header, and signature, so
//! the fee must not scale linearly with output count).
//!
//! `OUTPUT_AMOUNT` is sized far above the static min-fee floor (the
//! `calculate_min_required_fee`-too-low gap tracked at
//! dashpay/platform#3040, ~15M chain-time for 1in/1out): both
//! transitions land well above the floor, so the floor cannot tie the
//! two shapes and the per-output term genuinely dominates the
//! comparison.

use std::collections::BTreeMap;
use std::time::Duration;

use dpp::address_funds::PlatformAddress;

use crate::framework::prelude::*;
use crate::framework::wait::{
    wait_for_address_balance_chain_confirmed_n, CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
};

/// Gross credits the bank submits when funding the source address.
/// Bank uses `[DeductFromInput(0)]`; the source receives
/// `FUNDING_CREDITS` exactly (the bank's input absorbs its own fee).
///
/// Sizing covers every credit `addr_src` must pay before the 5-output
/// measured transfer runs: 6 pre-marker transfers (`dest_1` + 5
/// `dests`) at `MARKER_AMOUNT` gross each (`6 × 30M = 180M`, auto-select
/// may draw every marker off `addr_src`), plus the 1-output transfer's
/// gross (`50M`), plus the 5-output transfer's gross (`5 × 50M = 250M`)
/// — `480M` total outflow. Chain-time fees are absorbed by `output[0]`
/// under the `Σ inputs == Σ outputs` invariant, not an extra `addr_src`
/// debit. `700M` leaves ~`220M` headroom so `addr_src` still holds ≥
/// the 5-output transfer's `250M` input when its explicit-input
/// transition is built.
const FUNDING_CREDITS: u64 = 700_000_000;

/// Lower bound on the source's post-fund balance before the test
/// proceeds. Bank uses `[DeductFromInput(0)]`, so `addr_src` should
/// receive `FUNDING_CREDITS` exactly; the floor leaves a small
/// allowance for any reconciliation drift.
const FUNDING_FLOOR: u64 = 650_000_000;

/// Per-output gross credit amount used in BOTH the 1-output and the
/// 5-output transfer, so the only variable between the two is the
/// output count. Sized well above the #3040 static min-fee floor so
/// both transitions clear it and the floor cannot tie the two shapes.
const OUTPUT_AMOUNT: u64 = 50_000_000;

/// Per-marker gross. One marker advances the receive-address cursor
/// and establishes each destination's address-funds record so the
/// measured transfer hits an UPDATE (not a one-off CREATE). Above the
/// empirical 1in/1out chain-time fee (~15M) so the marker output lands
/// with an observable post-fee balance.
const MARKER_AMOUNT: u64 = 30_000_000;

/// Lower bound on a destination's post-transfer balance. A non-zero
/// floor keeps the `wait_for_balance` polls deterministic.
const OUTPUT_FLOOR: u64 = 1_000_000;

/// Per-step deadline for balance observations.
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

/// Real chain-time fee of a single self-transfer that drew its inputs
/// only from `addr_src`. Under `[ReduceOutput(0)]` the encoded
/// transition balances pre-fee, so the wallet's only credit loss is
/// the chain-time fee Drive charged against `output[0]`. It surfaces
/// as the shortfall of the destination deltas against the gross sum:
/// `fee = Σ gross outputs − Σ(post − pre) over destinations`.
fn real_fee(
    pre: &BTreeMap<PlatformAddress, u64>,
    post: &BTreeMap<PlatformAddress, u64>,
    dests: &[PlatformAddress],
    gross_per_output: u64,
) -> u64 {
    let mut total_delta = 0u64;
    for d in dests {
        let before = pre.get(d).copied().unwrap_or(0);
        let after = post.get(d).copied().unwrap_or(0);
        total_delta = total_delta.saturating_add(after.saturating_sub(before));
    }
    let gross = gross_per_output.saturating_mul(dests.len() as u64);
    gross.saturating_sub(total_delta)
}

#[tokio_shared_rt::test(shared)]
async fn pa_003_fee_scaling() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    let s = setup().await.expect("e2e setup failed");

    // ---- Fund a single source `addr_src` with enough headroom for ----
    // ---- BOTH the 1-output and 5-output transfers. ----
    let addr_src = s
        .test_wallet
        .next_unused_address()
        .await
        .expect("derive addr_src");
    s.ctx
        .bank()
        .fund_address(&addr_src, FUNDING_CREDITS)
        .await
        .expect("bank.fund_address");
    // Funding precondition gated on the proof-verified chain view
    // (Found-025-immune): a stale local-map 0 would hang this before
    // the marker / explicit-input transfers that consume addr_src.
    wait_for_address_balance_chain_confirmed_n(
        s.ctx.sdk(),
        &addr_src,
        FUNDING_FLOOR,
        CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
        STEP_TIMEOUT,
    )
    .await
    .expect("addr_src funding never observed");

    // Chain-confirmed gate is sdk-only and never warms the wallet's local
    // balance map; refresh it before the marker transfer consumes addr_src.
    s.test_wallet.sync_balances().await.expect("pre-tx sync");

    // ---- 1-output transfer: derive `dest_1`, pre-marker it, then ----
    // ---- transfer from `addr_src` only and capture the real fee. ----
    let dest_1 = s
        .test_wallet
        .next_unused_address()
        .await
        .expect("derive dest_1");
    assert_ne!(addr_src, dest_1, "dest_1 must differ from addr_src");

    // Pre-marker `dest_1` so its measured transfer hits an address-funds
    // UPDATE — symmetric with the five pre-markered destinations below.
    // Without this the 1-output measured transfer would pay a one-off
    // CREATE on `dest_1`'s first credit, inflating `fee_1` for a reason
    // unrelated to output count and biasing `fee_5 > fee_1`.
    let marker_1: BTreeMap<_, _> = std::iter::once((dest_1, MARKER_AMOUNT)).collect();
    s.test_wallet
        .transfer(marker_1)
        .await
        .expect("dest_1 marker transfer");
    wait_for_balance(&s.test_wallet, &dest_1, OUTPUT_FLOOR, STEP_TIMEOUT)
        .await
        .expect("dest_1 marker never observed");

    s.test_wallet.sync_balances().await.expect("pre-1-out sync");
    let pre_1 = s.test_wallet.balances().await;

    // Explicit single-address input: the 1-output transfer draws only
    // from `addr_src`, matching the 5-output transfer's input set so
    // output count is the only varied factor. The map value is the
    // contribution `addr_src` must cover — the transfer's gross
    // (`OUTPUT_AMOUNT`), which `addr_src` always holds post-markers.
    let outputs_1: BTreeMap<_, _> = std::iter::once((dest_1, OUTPUT_AMOUNT)).collect();
    let inputs_1: BTreeMap<_, _> = std::iter::once((addr_src, OUTPUT_AMOUNT)).collect();
    s.test_wallet
        .transfer_with_inputs(outputs_1, inputs_1)
        .await
        .expect("1-output transfer");
    wait_for_balance(&s.test_wallet, &dest_1, OUTPUT_FLOOR, STEP_TIMEOUT)
        .await
        .expect("dest_1 transfer never observed");

    s.test_wallet
        .sync_balances()
        .await
        .expect("post-1-out sync");
    let post_1 = s.test_wallet.balances().await;
    let fee_1 = real_fee(&pre_1, &post_1, &[dest_1], OUTPUT_AMOUNT);

    // ---- Derive five distinct destinations. `next_unused_address`
    // reserves its index on hand-out (Found-026 `bc87e4dec9`), so the
    // five derivations are already distinct. The marker transfer is now
    // only needed to establish each destination's address-funds record
    // so the measured 5-output transfer hits UPDATE storage ops —
    // symmetric with the pre-markered `dest_1` (the QA-003 fee setup;
    // unrelated to the cursor). Markers do not affect the measured
    // fees: `real_fee` nets post against a pre snapshot. ----
    let mut dests = Vec::with_capacity(5);
    for i in 0..5 {
        let d = s
            .test_wallet
            .next_unused_address()
            .await
            .unwrap_or_else(|err| panic!("derive dest_{i}: {err:?}"));
        let marker_outputs: BTreeMap<_, _> = std::iter::once((d, MARKER_AMOUNT)).collect();
        s.test_wallet
            .transfer(marker_outputs)
            .await
            .unwrap_or_else(|err| panic!("marker transfer for dest_{i}: {err:?}"));
        wait_for_balance(&s.test_wallet, &d, OUTPUT_FLOOR, STEP_TIMEOUT)
            .await
            .unwrap_or_else(|err| panic!("dest_{i} marker never observed: {err:?}"));
        dests.push(d);
    }
    for (i, d_i) in dests.iter().enumerate() {
        for d_j in dests.iter().skip(i + 1) {
            assert_ne!(d_i, d_j, "duplicate dests in five-output set");
        }
    }

    // ---- 5-output transfer: same explicit single-address input set
    // (`addr_src` only) and same per-output gross as the 1-output
    // transfer. Output count is the only deliberately varied factor. ----
    s.test_wallet.sync_balances().await.expect("pre-5-out sync");
    let pre_5 = s.test_wallet.balances().await;

    // Explicit input weight is this transfer's gross (`5 ×
    // OUTPUT_AMOUNT`) — what `addr_src` must contribute. `FUNDING_CREDITS`
    // headroom guarantees `addr_src` still holds ≥ this after all six
    // markers and the 1-output transfer.
    let gross_5 = OUTPUT_AMOUNT.saturating_mul(5);
    let outputs_5: BTreeMap<_, _> = dests.iter().map(|d| (*d, OUTPUT_AMOUNT)).collect();
    let inputs_5: BTreeMap<_, _> = std::iter::once((addr_src, gross_5)).collect();
    s.test_wallet
        .transfer_with_inputs(outputs_5, inputs_5)
        .await
        .expect("5-output transfer");

    // Wait on the LEX-LARGEST destination — `[ReduceOutput(0)]` only
    // deducts the fee from output[0] (lex-smallest), so the lex-largest
    // arrives at its pre balance + gross exactly.
    let lex_largest = *dests.iter().max().expect("dests non-empty");
    let lex_largest_pre = pre_5.get(&lex_largest).copied().unwrap_or(0);
    wait_for_balance(
        &s.test_wallet,
        &lex_largest,
        lex_largest_pre.saturating_add(OUTPUT_AMOUNT),
        STEP_TIMEOUT,
    )
    .await
    .expect("lex-largest dest never observed");

    s.test_wallet
        .sync_balances()
        .await
        .expect("post-5-out sync");
    let post_5 = s.test_wallet.balances().await;
    let fee_5 = real_fee(&pre_5, &post_5, &dests, OUTPUT_AMOUNT);

    tracing::info!(
        target: "platform_wallet::e2e::cases::pa_003",
        fee_1,
        fee_5,
        ratio_5_over_1 = ?(fee_5 as f64 / fee_1 as f64),
        "fee scaling snapshot (real chain-time fees)"
    );

    // ---- PA-003 contract assertions ----
    assert!(fee_1 > 0, "1-output fee must be positive; got {fee_1}");
    assert!(fee_5 > 0, "5-output fee must be positive; got {fee_5}");
    // Both transfers draw inputs from the same single address
    // (`addr_src`) with the same per-output gross, so output count is
    // the only varied factor. Both grosses are far above the #3040
    // static min-fee floor (~15M), so neither transition lands on the
    // floor and the floor cannot tie the two shapes. The 5-output
    // transition serializes four extra P2PKH outputs (~28 bytes each)
    // plus four extra output-storage operations, so its chain-time
    // storage+processing cost is strictly higher. More outputs ⇒
    // strictly more fee.
    assert!(
        fee_5 > fee_1,
        "5-output real chain-time fee must exceed 1-output's (four extra \
         outputs ⇒ strictly more storage+processing cost); \
         fee_1={fee_1}, fee_5={fee_5}"
    );
    // Sub-linear: the four extra outputs share the transition's input
    // bytes, header, and signature, so 5× outputs does NOT mean 5× fee.
    // This bound surfaces a regression where the fee schedule starts
    // charging per-output linearly.
    assert!(
        fee_5 < fee_1.saturating_mul(5),
        "5-output fee ({fee_5}) must be sub-linear in output count \
         (1-output fee {fee_1} × 5 = {})",
        fee_1.saturating_mul(5)
    );
    // Explicit linear-fee-schedule tripwire (spec PA-003 regression
    // guard). With both measured transfers hitting UPDATE storage ops,
    // four extra P2PKH outputs add a bounded marginal cost. A schedule
    // that turned per-output linear would push `fee_5 − fee_1` well
    // past this ceiling. The ceiling is loose enough to absorb the
    // #3040 chain-time gap; tighten it deliberately once #3040 is
    // resolved.
    const FEE_DELTA_CEILING: u64 = 25_000_000;
    let fee_delta = fee_5.saturating_sub(fee_1);
    assert!(
        fee_delta < FEE_DELTA_CEILING,
        "5-output fee minus 1-output fee ({fee_delta}) exceeds the \
         regression-guard ceiling ({FEE_DELTA_CEILING}); the fee \
         schedule shifted significantly or four extra outputs are \
         being charged near-linearly — investigate before bumping"
    );

    s.teardown().await.expect("teardown");
}
