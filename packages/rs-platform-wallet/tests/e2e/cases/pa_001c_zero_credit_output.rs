//! PA-001c — Zero-credit single-output transfer.
//! Spec: `tests/e2e/TEST_SPEC.md` §3 "Platform Addresses (PA)" → PA-001c.
//! Priority: P2.
//!
//! Pins the wallet's contract at the zero-amount boundary. Two
//! permitted contracts per spec; PA-001c surfaces and pins
//! whichever the wallet implements:
//!   (a) **Reject**: typed validation error of "amount must be
//!       positive" shape; no broadcast; balances unchanged.
//!   (b) **Accept**: transfer broadcasts; addr_2 ends at 0;
//!       addr_1 only loses the fee.
//!
//! Empirically the wallet's `transfer()` validates `outputs.is_empty()`
//! up front (`transfer.rs:40`) but does NOT validate per-output
//! amounts — a zero-amount entry is forwarded to the SDK, which in
//! turn submits a zero-output to the protocol. We expect this to
//! either hit a Drive-side validation rule or land as a fee-only
//! transfer. Either way, the wallet MUST NOT panic.

use std::collections::BTreeMap;
use std::time::Duration;

use crate::framework::prelude::*;
use crate::framework::wait::{
    wait_for_address_balance_chain_confirmed_n, CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
};

/// Gross credits the bank submits when funding `addr_1`. Bank uses
/// `[ReduceOutput(0)]`; addr_1 receives `FUNDING_CREDITS − bank_fee`.
const FUNDING_CREDITS: u64 = 30_000_000;

/// Lower bound on what addr_1 must receive before the test proceeds.
const FUNDING_FLOOR: u64 = 1_000_000;

/// Per-step deadline for balance observations.
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

#[tokio_shared_rt::test(shared)]
async fn pa_001c_zero_credit_single_output() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    let s = setup().await.expect("e2e setup failed");

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
    // the zero-credit transfer that consumes addr_1's funding.
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
    let addr_1_pre = pre_balances.get(&addr_1).copied().unwrap_or(0);

    let addr_2 = s
        .test_wallet
        .next_unused_address()
        .await
        .expect("derive addr_2");
    assert_ne!(addr_1, addr_2);

    // ---- The PA-001c boundary call: 0-credit output. ----
    let outputs: BTreeMap<_, _> = std::iter::once((addr_2, 0u64)).collect();
    let result = s.test_wallet.transfer(outputs).await;

    tracing::info!(
        target: "platform_wallet::e2e::cases::pa_001c",
        ?result,
        "zero-credit transfer outcome"
    );

    s.test_wallet.sync_balances().await.expect("post-tx sync");
    let post_balances = s.test_wallet.balances().await;
    let addr_1_post = post_balances.get(&addr_1).copied().unwrap_or(0);
    let addr_2_post = post_balances.get(&addr_2).copied().unwrap_or(0);

    match result {
        // Contract (a): rejected with a typed error. The wallet's
        // contract here is "no panic, no broadcast" — both are
        // observable through the post-tx balance snapshot.
        Err(err) => {
            tracing::info!(
                target: "platform_wallet::e2e::cases::pa_001c",
                error = %err,
                "zero-credit transfer rejected (contract a)"
            );
            // Balances unchanged on rejection.
            assert_eq!(
                addr_1_post, addr_1_pre,
                "PA-001c contract (a): rejected zero-credit transfer must \
                 leave addr_1 balance unchanged ({addr_1_pre} → {addr_1_post})"
            );
            assert_eq!(
                addr_2_post, 0,
                "PA-001c contract (a): rejected transfer must leave addr_2 at 0; \
                 observed {addr_2_post}"
            );
        }
        // Contract (b): accepted as fee-only. addr_2 stays at 0;
        // addr_1 decreased by the chain-time fee.
        Ok(_changeset) => {
            tracing::info!(
                target: "platform_wallet::e2e::cases::pa_001c",
                addr_1_pre,
                addr_1_post,
                addr_2_post,
                "zero-credit transfer accepted (contract b)"
            );
            assert_eq!(
                addr_2_post, 0,
                "PA-001c contract (b): accepted zero-credit transfer must leave \
                 addr_2 balance at exactly 0; observed {addr_2_post}"
            );
            assert!(
                addr_1_post < addr_1_pre,
                "PA-001c contract (b): accepted fee-only transfer must \
                 reduce addr_1 balance by the fee; observed {addr_1_post} ≥ {addr_1_pre}"
            );
            // Sanity: the drain must equal exactly the fee
            // (= addr_1_pre - addr_1_post). The drain should be
            // strictly less than addr_1_pre (no over-charging).
            let drain = addr_1_pre.saturating_sub(addr_1_post);
            assert!(
                drain < addr_1_pre,
                "PA-001c contract (b): fee drain ({drain}) must be \
                 less than full balance ({addr_1_pre})"
            );
        }
    }

    s.teardown().await.expect("teardown");
}
