//! PA-006b — Two concurrent broadcasts of identical ST bytes.
//!
//! **RED-by-design (Found-032)**: `sync_balances()` does not populate the local
//! balance map for `addr_src` because the incremental DAPI delta returns 0 new
//! entries (`query_height >= metadata_height`). Both `addr_src_pre` and
//! `addr_src_post` resolve to `unwrap_or(0) = 0`, so `src_drain = 0` even though
//! one of the two concurrent broadcasts applied successfully on-chain.
//! The on-chain double-debit protection contract holds; this is a local tracking
//! defect. See TEST_SPEC.md Found-032.
//!
//! Spec: `tests/e2e/TEST_SPEC.md` §3 "Platform Addresses (PA)" → PA-006b.
//! Priority: P2.
//!
//! # Security contract
//!
//! Two parallel broadcasts of the SAME signed state-transition bytes (same
//! input, same nonce) MUST NOT double-debit the source address. This is the
//! on-chain invariant pinned here.
//!
//! # Deduplication layers — QA-V26-001
//!
//! Deduplication happens at two distinct layers with different granularity:
//!
//! * **CheckTx / mempool (per-node):** each Tenderdash node deduplicates
//!   in its own mempool. `StateTransition::broadcast` returns `Ok` at this
//!   granularity — it does NOT wait for block inclusion.
//! * **Consensus (global):** the proposer selects at most one copy of a
//!   transition for a block. The chain applies it exactly once.
//!
//! DAPI load-balances across ~28 testnet nodes. Two concurrent broadcasts of
//! identical bytes will frequently hit *different* nodes, each of which
//! accepts the transition into its local mempool (both `Ok`). Asserting
//! `ok_count == 1` at the broadcast layer was therefore incorrect
//! (QA-V26-001). The correct assertion is on the chain-side outcome: the
//! source balance must decrease by exactly one transfer's worth, never two.
//!
//! Differs from PA-006 (sequential replay) in that the two submissions hit
//! the network simultaneously. The `build_transfer_st_bytes` helper produces
//! ST bytes with a fresh on-chain nonce WITHOUT a live broadcast, so both
//! spawned tasks race for the same nonce slot.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use dpp::serialization::PlatformDeserializable;
use dpp::state_transition::StateTransition;

use crate::framework::prelude::*;
use crate::framework::wait::{
    wait_for_address_balance_chain_confirmed_n, CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
};

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
    // Funding precondition gated on the proof-verified chain view
    // (Found-025-immune), not the local sync map. #480 keeps PA-*
    // post-broadcast asserts on `.balances()`; this is only a
    // funding precondition, not a `.balances()` assertion, so the
    // local-map rationale does not apply here (QA-504).
    wait_for_address_balance_chain_confirmed_n(
        s.ctx.sdk(),
        &addr_src,
        FUNDING_FLOOR,
        CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
        STEP_TIMEOUT,
    )
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

    // ---- At least one broadcast must reach the network (QA-V26-001).
    //
    // Both returning Ok is valid: DAPI load-balances across multiple nodes and
    // each node's mempool deduplicates independently. The chain-side dedup
    // (consensus) is what prevents the double-debit — asserted below via the
    // post-sync balance drain. Catching the case where BOTH fail is still
    // valuable: it would indicate the broadcast layer is entirely unreachable.
    let ok_count = [&r_a, &r_b].iter().filter(|r| r.is_ok()).count();
    assert!(
        ok_count >= 1,
        "PA-006b: at least one concurrent broadcast must succeed (got 0); \
         r_a={r_a:?}, r_b={r_b:?}"
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

    // The drain includes the transfer amount plus the chain fee. We assert it
    // is in the range [TRANSFER_CREDITS, 2 * TRANSFER_CREDITS) — that is,
    // greater than the bare transfer (fee > 0) but strictly less than two
    // transfers' worth. The upper bound is the no-double-debit contract.
    let src_drain = addr_src_pre.saturating_sub(addr_src_post);
    assert!(
        (TRANSFER_CREDITS..2 * TRANSFER_CREDITS).contains(&src_drain),
        "Found-032 (RED-by-design): addr_src drain must reflect exactly ONE transfer \
         (including fee); expected [{TRANSFER_CREDITS}, {}), got {src_drain}. \
         sync_balances() does not populate the local balance map for addr_src when \
         the incremental DAPI delta returns 0 new entries — both pre/post balances \
         resolve to 0. On-chain behaviour is correct; this is a local tracking defect. \
         See TEST_SPEC.md Found-032.",
        2 * TRANSFER_CREDITS,
    );
    assert!(
        (TRANSFER_FLOOR..TRANSFER_CREDITS).contains(&addr_dst_post),
        "PA-006b: addr_dst must hold ONE transfer's post-fee net \
         (in [{TRANSFER_FLOOR}, {TRANSFER_CREDITS})); observed {addr_dst_post}"
    );

    s.teardown().await.expect("teardown");
}
