//! PA-001 — Multi-output platform-address transfer (one tx, N outputs).
//! Spec: `tests/e2e/TEST_SPEC.md` §3 "Platform Addresses (PA)" → PA-001.
//! Priority: P0.
//!
//! Bank funds `addr_1`. The wallet derives a pair of fresh receive
//! addresses (`addr_2`, `addr_3`) — each `next_unused_address` call
//! reserves its index on hand-out (Found-026 `bc87e4dec9`), so the
//! addresses are already pairwise distinct (PA-005 Invariant 1). The
//! "prep" transfer below now only funds `addr_2`. The PA-001
//! transfer itself then sends `OUTPUT_A_CREDITS` and
//! `OUTPUT_B_CREDITS` to {`addr_2`, `addr_3`} in a single transition.
//!
//! Under the default `[ReduceOutput(0)]` strategy the **lex-smallest**
//! output absorbs the chain-time fee — assertions pin the lex-larger
//! output's gross arrival exactly, and bound the lex-smaller's
//! gross-minus-fee value. The `Σ inputs == Σ outputs` invariant is
//! checked against `addr_1`'s residual change.
//!
//! Why bumped output amounts: see PA-002's `#3040` note. For 1in/2out
//! the empirical chain-time fee is larger (~20M) than 1in/1out, so
//! `OUTPUT_A_CREDITS` (the lex-smallest output's gross) sits well
//! above that ceiling.

use std::collections::BTreeMap;
use std::time::Duration;

use crate::framework::prelude::*;
use crate::framework::wait::{
    wait_for_address_balance_chain_confirmed_n, CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
};

/// Gross credits the bank submits when funding `addr_1`. Bank uses
/// `[ReduceOutput(0)]`; addr_1 receives `FUNDING_CREDITS − bank_fee`.
/// Sized to cover (a) the prep transfer that marks `addr_2` used,
/// (b) the multi-output transfer's gross sum
/// (`OUTPUT_A_CREDITS + OUTPUT_B_CREDITS`), and (c) chain-time fees on
/// every transition the harness drives.
const FUNDING_CREDITS: u64 = 250_000_000;

/// Lower bound on what addr_1 must receive after the bank's fee
/// deduction before the test proceeds.
const FUNDING_FLOOR: u64 = 200_000_000;

/// Marker transfer to advance the receive-address cursor past
/// `addr_2`. Sized above the empirical 1in/1out chain-time fee
/// (~15M, see #3040) so `addr_2` lands with a non-zero post-fee
/// balance and `wait_for_balance(addr_2, …)` can observe it.
const PREP_CREDITS: u64 = 30_000_000;

/// Lower bound on `addr_2`'s balance after the prep transfer settles
/// (gross PREP minus 1in/1out chain-time fee).
const PREP_FLOOR: u64 = 1_000_000;

/// Gross credits sent to the lex-smallest of the two destination
/// addresses. `[ReduceOutput(0)]` charges the chain-time fee against
/// this output, so its on-chain delta is `OUTPUT_A_CREDITS − fee`.
/// Sized well above the empirical 1in/2out fee (~20M) to dodge #3040.
const OUTPUT_A_CREDITS: u64 = 50_000_000;

/// Gross credits sent to the lex-larger of the two destination
/// addresses. This output is **not** reduced by the chain-time fee;
/// its on-chain delta must equal this gross value exactly.
const OUTPUT_B_CREDITS: u64 = 60_000_000;

/// Lower bound on the lex-smaller output's post-fee delta.
const OUTPUT_A_FLOOR: u64 = 1_000_000;

/// Upper bound on the chain-time fee for a 1in/2out transition. The
/// empirical fee at the time PA-001 was written sits around ~20M
/// credits (per platform #3040's static-vs-chain-time gap analysis);
/// pinning the assertion here at 30M leaves room for protocol-version
/// drift while still surfacing a fee-explosion regression. A failure
/// of this bound means either (a) the protocol's fee schedule shifted
/// significantly, in which case update this constant deliberately, or
/// (b) a wallet-side or dpp-side regression is over-charging — which
/// is precisely what a tight bound is meant to catch.
const MULTI_FEE_CEILING: u64 = 30_000_000;

/// Per-step deadline for balance observations.
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

#[tokio_shared_rt::test(shared)]
async fn pa_001_multi_output_transfer() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    let s = setup().await.expect("e2e setup failed");

    // ---- Setup: derive 3 distinct addresses, only addr_1 funded ----

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
    // Funding precondition gated on the proof-verified chain view
    // (Found-025-immune): a stale local-map 0 would hang this before
    // the prep transfer that consumes addr_1's funding.
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
    // balance map; refresh it before the prep transfer consumes addr_1.
    s.test_wallet.sync_balances().await.expect("pre-tx sync");

    let addr_2 = s
        .test_wallet
        .next_unused_address()
        .await
        .expect("derive addr_2");
    assert_ne!(addr_1, addr_2, "addr_2 must differ from addr_1");

    // Prep transfer to fund `addr_2`; `addr_2` absorbs the chain-time
    // fee (it's the sole output). Hand-outs are already distinct
    // post-Found-026 (`bc87e4dec9`) — see PA-005 Invariant 1.
    let prep_outputs: BTreeMap<_, _> = std::iter::once((addr_2, PREP_CREDITS)).collect();
    s.test_wallet
        .transfer(prep_outputs)
        .await
        .expect("prep transfer to addr_2");
    wait_for_balance(&s.test_wallet, &addr_2, PREP_FLOOR, STEP_TIMEOUT)
        .await
        .expect("addr_2 prep transfer never observed");

    let addr_3 = s
        .test_wallet
        .next_unused_address()
        .await
        .expect("derive addr_3");
    assert_ne!(addr_1, addr_3, "addr_3 must differ from addr_1");
    assert_ne!(addr_2, addr_3, "addr_3 must differ from addr_2");

    // ---- The PA-001 transfer: one transition, two outputs ----

    // Capture the pre-multi balance snapshot so we can compute deltas
    // (addr_2 already holds the prep remainder).
    s.test_wallet.sync_balances().await.expect("pre-multi sync");
    let pre_balances = s.test_wallet.balances().await;
    let addr_1_pre = pre_balances.get(&addr_1).copied().unwrap_or(0);
    let addr_2_pre = pre_balances.get(&addr_2).copied().unwrap_or(0);
    let addr_3_pre = pre_balances.get(&addr_3).copied().unwrap_or(0);

    // Route the smaller output (OUTPUT_A) to whichever destination
    // sorts first lexicographically — that's the one ReduceOutput(0)
    // charges the fee against.
    let (lex_lo, lex_hi) = if addr_2 < addr_3 {
        (addr_2, addr_3)
    } else {
        (addr_3, addr_2)
    };
    let multi_outputs: BTreeMap<_, _> = [(lex_lo, OUTPUT_A_CREDITS), (lex_hi, OUTPUT_B_CREDITS)]
        .into_iter()
        .collect();
    s.test_wallet
        .transfer(multi_outputs)
        .await
        .expect("multi-output self-transfer");

    // Wait for both destinations. The lex-larger output arrives at
    // exactly its gross amount (no fee deduction); the lex-smaller
    // arrives at gross-minus-fee. Compute the per-address delta
    // expectation against the pre-multi snapshot.
    let lex_hi_pre = if lex_hi == addr_2 {
        addr_2_pre
    } else {
        addr_3_pre
    };
    let lex_lo_pre = if lex_lo == addr_2 {
        addr_2_pre
    } else {
        addr_3_pre
    };
    wait_for_balance(
        &s.test_wallet,
        &lex_hi,
        lex_hi_pre.saturating_add(OUTPUT_B_CREDITS),
        STEP_TIMEOUT,
    )
    .await
    .expect("lex_hi never observed");
    wait_for_balance(
        &s.test_wallet,
        &lex_lo,
        lex_lo_pre.saturating_add(OUTPUT_A_FLOOR),
        STEP_TIMEOUT,
    )
    .await
    .expect("lex_lo never observed");

    s.test_wallet
        .sync_balances()
        .await
        .expect("post-multi sync");
    let post_balances = s.test_wallet.balances().await;
    let addr_1_post = post_balances.get(&addr_1).copied().unwrap_or(0);
    let lex_lo_post = post_balances.get(&lex_lo).copied().unwrap_or(0);
    let lex_hi_post = post_balances.get(&lex_hi).copied().unwrap_or(0);

    let lo_delta = lex_lo_post.saturating_sub(lex_lo_pre);
    let hi_delta = lex_hi_post.saturating_sub(lex_hi_pre);
    let multi_fee = OUTPUT_A_CREDITS.saturating_sub(lo_delta);
    let addr_1_drain = addr_1_pre.saturating_sub(addr_1_post);

    tracing::info!(
        target: "platform_wallet::e2e::cases::pa_001",
        ?addr_1,
        ?lex_lo,
        ?lex_hi,
        addr_1_pre,
        addr_1_post,
        lo_delta,
        hi_delta,
        multi_fee,
        "post-multi-output balance snapshot"
    );

    // PA-001 contract: lex-larger output arrives at gross exactly
    // (ReduceOutput(0) only deducts from output[0]).
    assert_eq!(
        hi_delta, OUTPUT_B_CREDITS,
        "lex-larger output must arrive at gross amount exactly \
         (lex-smaller absorbs fee under [ReduceOutput(0)])"
    );
    // Lex-smaller output absorbed the chain-time fee.
    assert!(
        (OUTPUT_A_FLOOR..OUTPUT_A_CREDITS).contains(&lo_delta),
        "lex-smaller output delta must be gross-minus-fee in \
         [{OUTPUT_A_FLOOR}, {OUTPUT_A_CREDITS}); observed {lo_delta}"
    );
    assert!(
        multi_fee > 0,
        "multi-output transfer must charge a non-zero fee"
    );
    assert!(
        multi_fee < MULTI_FEE_CEILING,
        "multi-output fee {multi_fee} exceeds the regression-guard ceiling \
         {MULTI_FEE_CEILING} — either the protocol fee schedule shifted \
         (update MULTI_FEE_CEILING deliberately) or a fee-explosion \
         regression has landed on either the wallet or dpp side"
    );
    // Σ inputs == Σ outputs (gross): addr_1 was drained by exactly
    // the gross output total. The actual fee left output[0]'s
    // amount, not addr_1's contribution.
    let gross_outputs = OUTPUT_A_CREDITS.saturating_add(OUTPUT_B_CREDITS);
    assert_eq!(
        addr_1_drain, gross_outputs,
        "addr_1 drain must equal `Σ outputs` (gross) — Σ inputs == Σ outputs \
         invariant; expected {gross_outputs}, observed {addr_1_drain}"
    );

    s.teardown().await.expect("teardown");
}
