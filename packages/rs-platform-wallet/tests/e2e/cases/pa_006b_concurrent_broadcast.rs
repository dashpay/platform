//! PA-006b — Two concurrent broadcasts of identical ST bytes.
//! Spec: `tests/e2e/TEST_SPEC.md` §3 "Platform Addresses (PA)" → PA-006b.
//! Priority: P2.
//!
//! Pins the SDK / DAPI race-condition contract: two parallel
//! broadcasts of the SAME signed state-transition bytes (same input,
//! same nonce) MUST resolve to exactly one accepted transition. The
//! other gets a stale-nonce / already-exists error class. Without
//! this, a race in the mempool de-duplication path could let both
//! land and double-debit the source address.
//!
//! Differs from PA-006 (sequential replay) in that the two
//! submissions hit the network in flight at the same time. The
//! mempool's de-dup logic must serialize them deterministically.
//!
//! Uses the harness's `build_transfer_st_bytes` helper (added
//! alongside this case) — produces ST bytes with a fresh on-chain
//! nonce WITHOUT broadcasting a parallel production build, so both
//! `tokio::spawn`ed broadcasts race for the same first-write slot.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use dpp::serialization::PlatformDeserializable;
use dpp::state_transition::StateTransition;

use crate::framework::prelude::*;

/// Gross credits the bank submits when funding `addr_src`.
const FUNDING_CREDITS: u64 = 100_000_000;

/// Lower bound on what addr_src must hold before the test proceeds.
const FUNDING_FLOOR: u64 = 70_000_000;

/// Gross credits transferred. Sized above empirical 1in/1out
/// chain-time fee (~15M) to dodge #3040.
const TRANSFER_CREDITS: u64 = 50_000_000;

/// Lower bound on `addr_dst`'s post-fee balance.
const TRANSFER_FLOOR: u64 = 1_000_000;

/// Per-step deadline for balance observations.
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

#[tokio_shared_rt::test(shared)]
async fn pa_006b_concurrent_identical_broadcasts() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    let s = setup().await.expect("e2e setup failed");

    // ---- Fund a source address. ----
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
    wait_for_balance(&s.test_wallet, &addr_src, FUNDING_FLOOR, STEP_TIMEOUT)
        .await
        .expect("addr_src funding never observed");

    s.test_wallet
        .sync_balances()
        .await
        .expect("pre-broadcast sync");
    let pre_balances = s.test_wallet.balances().await;
    let addr_src_pre = pre_balances.get(&addr_src).copied().unwrap_or(0);

    let addr_dst = s
        .test_wallet
        .next_unused_address()
        .await
        .expect("derive addr_dst");
    assert_ne!(addr_src, addr_dst);

    // ---- Build (do not broadcast) the ST bytes once. ----
    let inputs: BTreeMap<_, _> = std::iter::once((addr_src, addr_src_pre)).collect();
    let outputs: BTreeMap<_, _> = std::iter::once((addr_dst, TRANSFER_CREDITS)).collect();
    let bytes = s
        .test_wallet
        .build_transfer_st_bytes(outputs, inputs)
        .await
        .expect("build_transfer_st_bytes");

    // Wrap the bytes in an `Arc<Vec<u8>>` so two spawn'd tasks share
    // them without contending on a clone budget.
    let bytes = Arc::new(bytes);

    // ---- Two concurrent broadcasts of the SAME bytes. ----
    use dash_sdk::platform::transition::broadcast::BroadcastStateTransition;

    let sdk_a = Arc::clone(s.ctx.sdk());
    let b1 = Arc::clone(&bytes);
    let task_a = tokio::spawn(async move {
        let st = StateTransition::deserialize_from_bytes(&b1)
            .expect("task_a: deserialize captured ST bytes");
        st.broadcast(sdk_a.as_ref(), None).await
    });

    let sdk_b = Arc::clone(s.ctx.sdk());
    let b2 = Arc::clone(&bytes);
    let task_b = tokio::spawn(async move {
        let st = StateTransition::deserialize_from_bytes(&b2)
            .expect("task_b: deserialize captured ST bytes");
        st.broadcast(sdk_b.as_ref(), None).await
    });

    let r_a = task_a.await.expect("task_a join");
    let r_b = task_b.await.expect("task_b join");

    tracing::info!(
        target: "platform_wallet::e2e::cases::pa_006b",
        ?r_a,
        ?r_b,
        "concurrent broadcast outcomes"
    );

    // ---- Exactly one MUST succeed; the other MUST fail with the
    // documented stale-nonce / duplicate-broadcast / already-exists
    // class. Loose `is_err` would let any error type slip past — pin
    // the class so a regression that surfaces a transport timeout or
    // a panic-shaped error is caught. Match on SDK's typed
    // `Error::AlreadyExists` first; fall back to keyword search on
    // the rendered string (consensus errors surface "InvalidIdentityNonce",
    // "stale nonce", "duplicate" via the wrapping error). ----
    let ok_count = [&r_a, &r_b].iter().filter(|r| r.is_ok()).count();
    assert_eq!(
        ok_count, 1,
        "PA-006b: exactly one concurrent broadcast must succeed; got {ok_count} \
         (r_a={r_a:?}, r_b={r_b:?})"
    );
    let losing_err = if r_a.is_err() {
        r_a.as_ref().expect_err("r_a is the loser")
    } else {
        r_b.as_ref().expect_err("r_b is the loser")
    };
    let err_string = format!("{losing_err}").to_lowercase();
    let dbg_string = format!("{losing_err:?}").to_lowercase();
    let class_match = matches!(losing_err, dash_sdk::Error::AlreadyExists(_))
        || [
            "already exists",
            "alreadyexists",
            "stale nonce",
            "invalididentitynonce",
            "duplicate",
        ]
        .iter()
        .any(|needle| err_string.contains(needle) || dbg_string.contains(needle));
    assert!(
        class_match,
        "PA-006b: losing concurrent broadcast must fail with a stale-nonce / \
         already-exists / duplicate class error; got display={losing_err}, \
         debug={losing_err:?}"
    );

    // ---- Wallet state reflects EXACTLY ONE applied transfer. ----
    wait_for_balance(&s.test_wallet, &addr_dst, TRANSFER_FLOOR, STEP_TIMEOUT)
        .await
        .expect("addr_dst never observed transfer");
    s.test_wallet
        .sync_balances()
        .await
        .expect("post-broadcast sync");
    let post_balances = s.test_wallet.balances().await;
    let addr_src_post = post_balances.get(&addr_src).copied().unwrap_or(0);
    let addr_dst_post = post_balances.get(&addr_dst).copied().unwrap_or(0);

    let src_drain = addr_src_pre.saturating_sub(addr_src_post);
    assert_eq!(
        src_drain, TRANSFER_CREDITS,
        "PA-006b: addr_src must show exactly ONE transfer's drain \
         (TRANSFER_CREDITS={TRANSFER_CREDITS}); observed drain={src_drain}, \
         which would imply both concurrent broadcasts landed (mempool race)"
    );
    assert!(
        (TRANSFER_FLOOR..TRANSFER_CREDITS).contains(&addr_dst_post),
        "PA-006b: addr_dst must hold ONE transfer's post-fee net \
         (in [{TRANSFER_FLOOR}, {TRANSFER_CREDITS})); observed {addr_dst_post}"
    );

    s.teardown().await.expect("teardown");
}
