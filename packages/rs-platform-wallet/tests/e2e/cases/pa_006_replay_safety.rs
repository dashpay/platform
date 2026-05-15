//! PA-006 — Replay safety: same outputs, second submission rejected.
//! Spec: `tests/e2e/TEST_SPEC.md` §3 "Platform Addresses (PA)" → PA-006.
//! Priority: P1.
//!
//! Pins the protocol-level nonce / replay-protection contract: a
//! state-transition built and signed with the same `(input, nonce)`
//! tuple as a previously-broadcasted ST MUST be rejected on the
//! second submission. Without this, a "spam-click" UX (mobile
//! double-tap, network retry) could double-debit the source address.
//!
//! The harness's `transfer_capturing_st_bytes` helper runs two
//! parallel builders against the same `(inputs, outputs)`: build #1
//! is serialized for capture (NEVER broadcasted from this helper);
//! build #2 is broadcasted via the canonical wallet path. The on-
//! chain nonce advances exactly once. We then re-broadcast build #1's
//! captured bytes — its nonce is now stale.
//!
//! Why this DOES NOT need #3040 dodging: the captured ST is built
//! against an explicit input map, so the chain-time fee absorption
//! happens via `[ReduceOutput(0)]` on the same output value the
//! production transfer just shipped. As long as the output amount
//! clears the chain-time fee floor, both build #1 and build #2 have
//! valid fee shape; the replay rejection is purely about nonce reuse.

use std::collections::BTreeMap;
use std::time::Duration;

use dpp::serialization::PlatformDeserializable;
use dpp::state_transition::StateTransition;

use crate::framework::prelude::*;
use crate::framework::wait::{
    wait_for_address_balance_chain_confirmed_n, CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
};

/// Gross credits the bank submits when funding `addr_src`. Bank uses
/// `[ReduceOutput(0)]`; addr_src receives `FUNDING_CREDITS − bank_fee`.
const FUNDING_CREDITS: u64 = 100_000_000;

/// Lower bound on what addr_src must receive before the test
/// proceeds.
const FUNDING_FLOOR: u64 = 70_000_000;

/// Gross credits the test ships in its 1in/1out transfer. Sized
/// well above the empirical chain-time fee (~15M) so the dual-build
/// helper's signing pass finds enough headroom on both builds.
const TRANSFER_CREDITS: u64 = 50_000_000;

/// Lower bound on `addr_dst`'s post-fee balance.
const TRANSFER_FLOOR: u64 = 1_000_000;

/// Per-step deadline for balance observations.
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

#[tokio_shared_rt::test(shared)]
async fn pa_006_replay_safety() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    let s = setup().await.expect("e2e setup failed");

    // ---- Fund a single source `addr_src`. ----
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
    // the dual-build transfer that consumes addr_src as an explicit
    // input.
    wait_for_address_balance_chain_confirmed_n(
        s.ctx.sdk(),
        &addr_src,
        FUNDING_FLOOR,
        CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
        STEP_TIMEOUT,
    )
    .await
    .expect("addr_src funding never observed");

    // Capture pre-broadcast snapshot of addr_src so we can verify
    // a failed re-broadcast leaves the wallet's view unchanged.
    s.test_wallet
        .sync_balances()
        .await
        .expect("pre-broadcast sync");
    let pre_balances = s.test_wallet.balances().await;
    let addr_src_pre = pre_balances.get(&addr_src).copied().unwrap_or(0);

    // Derive an unused destination via prep transfer to advance cursor.
    let addr_dst = s
        .test_wallet
        .next_unused_address()
        .await
        .expect("derive addr_dst");
    assert_ne!(addr_src, addr_dst);

    // ---- Capture ST bytes via the dual-build helper, broadcast once. ----
    // We use the explicit-inputs path so we control which address backs
    // the transfer; auto-select would pick a different input set on
    // each build.
    let inputs: BTreeMap<_, _> = std::iter::once((addr_src, addr_src_pre)).collect();
    let outputs: BTreeMap<_, _> = std::iter::once((addr_dst, TRANSFER_CREDITS)).collect();
    let (_cs, captured_bytes) = s
        .test_wallet
        .transfer_capturing_st_bytes(outputs, inputs)
        .await
        .expect("dual-build transfer + capture");
    wait_for_balance(&s.test_wallet, &addr_dst, TRANSFER_FLOOR, STEP_TIMEOUT)
        .await
        .expect("addr_dst never observed first transfer");

    // ---- Re-broadcast the captured bytes. Expect protocol rejection. ----
    let replay_st = StateTransition::deserialize_from_bytes(&captured_bytes)
        .expect("deserialize captured ST bytes");

    use dash_sdk::platform::transition::broadcast::BroadcastStateTransition;
    let sdk_ref: &dash_sdk::Sdk = s.ctx.sdk().as_ref();
    let replay_result = replay_st.broadcast(sdk_ref, None).await;

    tracing::info!(
        target: "platform_wallet::e2e::cases::pa_006",
        ?replay_result,
        "replay broadcast result"
    );

    // PA-006 contract: the second submission MUST fail with a
    // stale-nonce / already-exists shape. We pin the *class* (not the
    // exact wording) by matching on the SDK's typed
    // `Error::AlreadyExists` variant first, then by string-keyword
    // fallback to catch consensus-error wrappers that surface
    // "already exists" / "InvalidIdentityNonce" / "stale nonce" /
    // "duplicate" in the rendered display string.
    let replay_err = match replay_result {
        Ok(_) => panic!("PA-006: replayed ST broadcast must be rejected; got Ok"),
        Err(err) => err,
    };
    let err_string = format!("{replay_err}").to_lowercase();
    let dbg_string = format!("{replay_err:?}").to_lowercase();
    let class_match = matches!(replay_err, dash_sdk::Error::AlreadyExists(_))
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
        "PA-006: replay error must be of stale-nonce / already-exists class; \
         got display={replay_err}, debug={replay_err:?}"
    );

    // Wallet's view of `addr_src` and `addr_dst` must reflect ONE
    // applied transfer, not two — i.e. the replay didn't corrupt
    // the cache or the chain.
    s.test_wallet
        .sync_balances()
        .await
        .expect("post-replay sync");
    let post_balances = s.test_wallet.balances().await;
    let addr_src_post = post_balances.get(&addr_src).copied().unwrap_or(0);
    let addr_dst_post = post_balances.get(&addr_dst).copied().unwrap_or(0);

    // addr_src lost exactly the gross transfer amount (Σ inputs ==
    // Σ outputs invariant; fee is absorbed from output[0]). If the
    // replay had succeeded we'd have lost 2×TRANSFER_CREDITS.
    let src_drain = addr_src_pre.saturating_sub(addr_src_post);
    assert_eq!(
        src_drain, TRANSFER_CREDITS,
        "PA-006: addr_src must show exactly ONE transfer's drain \
         (TRANSFER_CREDITS={TRANSFER_CREDITS}); observed drain={src_drain}, \
         which would imply the replay was applied on top of the original"
    );
    assert!(
        (TRANSFER_FLOOR..TRANSFER_CREDITS).contains(&addr_dst_post),
        "PA-006: addr_dst must hold ONE transfer's post-fee net \
         (in [{TRANSFER_FLOOR}, {TRANSFER_CREDITS})); observed {addr_dst_post}"
    );

    s.teardown().await.expect("teardown");
}
