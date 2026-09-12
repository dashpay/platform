//! PA-002b — Zero-change exact-equality (`Σ outputs + fee == input balance`).
//! Spec: `tests/e2e/TEST_SPEC.md` §3 "Platform Addresses (PA)" → PA-002b.
//! Priority: P1.
//!
//! Pins the `Σ inputs == Σ outputs` invariant the wallet just shipped
//! regressions on (commits `aaf8be74ee` and `9ea9e7033c`). With the
//! default `[ReduceOutput(0)]` strategy:
//!   - `Σ inputs == Σ outputs` is the protocol-level identity (fee
//!     leaves output[0]'s amount at chain time, NOT input balance).
//!   - The auto-selector is supposed to consume input balance up to
//!     `Σ outputs` exactly, leaving change as `bal_input − Σ outputs`.
//!   - At the boundary `bal_input == Σ outputs`, no change is left
//!     and the source address must end at exactly 0.
//!
//! This test forces that boundary by transferring the full balance
//! of addr_1 (read post-fund-fee) to addr_2 in a single 1-output
//! transfer using `InputSelection::Explicit({addr_1: bal_1})` so the
//! auto-selector's "min covering prefix" logic isn't in the way.
//!
//! Without an exact-equality boundary case, this bug-class re-emerges
//! silently the next time the change-output predicate is touched.

use std::collections::BTreeMap;
use std::time::Duration;

use crate::framework::prelude::*;
use crate::framework::wait::{
    wait_for_address_balance_chain_confirmed_n, CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
};

/// Gross credits the bank submits when funding `addr_1`. Bank uses
/// `[ReduceOutput(0)]`; addr_1 receives `FUNDING_CREDITS − bank_fee`.
/// Sized well above the chain-time fee (~15M for 1in/1out) so the
/// post-fee balance has plenty of headroom for the test's own
/// transfer fee.
const FUNDING_CREDITS: u64 = 80_000_000;

/// Lower bound on what addr_1 must receive before the test proceeds.
const FUNDING_FLOOR: u64 = 50_000_000;

/// Per-step deadline for balance observations.
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

#[tokio_shared_rt::test(shared)]
async fn pa_002b_zero_change_exact_equality() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    let s = setup().await.expect("e2e setup failed");

    // ---- Fund addr_1, snapshot the post-fee balance. ----
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
    // the zero-change transfer that fully spends addr_1.
    wait_for_address_balance_chain_confirmed_n(
        s.ctx.sdk(),
        &addr_1,
        FUNDING_FLOOR,
        CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
        STEP_TIMEOUT,
    )
    .await
    .expect("addr_1 funding never observed");

    s.test_wallet.sync_balances().await.expect("pre-tx sync");
    let pre_balances = s.test_wallet.balances().await;
    let bal_1 = pre_balances.get(&addr_1).copied().unwrap_or(0);
    assert!(
        bal_1 >= FUNDING_FLOOR,
        "PA-002b: addr_1 must hold ≥ FUNDING_FLOOR before transfer; got {bal_1}"
    );

    // ---- Derive addr_2 via prep transfer (cursor advance). ----
    let addr_2 = s
        .test_wallet
        .next_unused_address()
        .await
        .expect("derive addr_2");
    assert_ne!(addr_1, addr_2);

    // ---- Construct the zero-change boundary transfer. ----
    // `Explicit({addr_1: bal_1})` declares addr_1 as the sole input
    // consuming its entire balance. `outputs = {addr_2: bal_1}` matches
    // the input sum exactly — no change. With `[ReduceOutput(0)]`,
    // chain-time fee leaves output[0]'s amount, so addr_2 receives
    // `bal_1 − fee`. addr_1 must end at exactly 0.
    let inputs: BTreeMap<_, _> = std::iter::once((addr_1, bal_1)).collect();
    let outputs: BTreeMap<_, _> = std::iter::once((addr_2, bal_1)).collect();

    s.test_wallet
        .transfer_with_inputs(outputs, inputs)
        .await
        .expect("zero-change exact-equality transfer");

    // ---- Wait for addr_2 to observe ANY positive balance. ----
    // Tight floor — we want to see addr_2 receive its post-fee net.
    wait_for_balance(&s.test_wallet, &addr_2, 1_000_000, STEP_TIMEOUT)
        .await
        .expect("addr_2 zero-change transfer never observed");

    s.test_wallet.sync_balances().await.expect("post-tx sync");
    let post_balances = s.test_wallet.balances().await;
    let addr_1_post = post_balances.get(&addr_1).copied().unwrap_or(0);
    let addr_2_post = post_balances.get(&addr_2).copied().unwrap_or(0);

    let observed_fee = bal_1.saturating_sub(addr_2_post);

    tracing::info!(
        target: "platform_wallet::e2e::cases::pa_002b",
        bal_1_pre = bal_1,
        addr_1_post,
        addr_2_post,
        observed_fee,
        "zero-change boundary snapshot"
    );

    // ---- PA-002b contract: addr_1 ends at EXACTLY zero. ----
    // The whole point of this test is the boundary at `bal_input ==
    // Σ outputs`. A regression that keeps a 1-credit residual on
    // addr_1 (off-by-one in the change predicate) fails this assertion.
    assert_eq!(
        addr_1_post, 0,
        "PA-002b: addr_1 must hold EXACTLY 0 credits after a \
         zero-change transfer (Σ inputs == Σ outputs invariant); \
         observed {addr_1_post} — change-output predicate regression?"
    );

    // ---- addr_2 received `bal_1 − fee`. fee in plausible range. ----
    assert!(
        addr_2_post < bal_1,
        "PA-002b: addr_2 must receive less than gross input \
         (fee absorbed via [ReduceOutput(0)]); observed {addr_2_post} ≥ {bal_1}"
    );
    assert!(
        observed_fee > 0,
        "PA-002b: fee must be positive (sanity check)"
    );
    // Σ inputs == Σ outputs (gross): addr_1 was drained by exactly
    // `bal_1`. The fee left addr_2's amount, not addr_1's contribution.
    let drain = bal_1.saturating_sub(addr_1_post);
    assert_eq!(
        drain, bal_1,
        "PA-002b: addr_1 drain ({drain}) must equal full pre-balance \
         ({bal_1}) under zero-change boundary"
    );

    s.teardown().await.expect("teardown");
}
