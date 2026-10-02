//! PA-004 — Sweep-back: drain test wallet, observe registry cleanup
//! and the swept address's on-chain zero balance.
//! Spec: `tests/e2e/TEST_SPEC.md` §3 "Platform Addresses (PA)" → PA-004.
//! Priority: P0.
//!
//! Validates the cleanup invariant the README promises in
//! §"Panic-safe cleanup". Without this test, a regression in
//! `cleanup.rs::teardown_one` would silently leak credits across
//! runs — bank slowly drains, eventually trips the under-funded
//! panic, no test ever names the cause.
//!
//! Flow:
//! 1. Bank-fund `addr_1` with [`FUNDING_CREDITS`]; wait for the test
//!    wallet to observe.
//! 2. Capture the seed bytes (need them post-teardown to re-derive a
//!    read-only view of the on-chain state).
//! 3. Call `setup_guard.teardown()` — sweep path drains the test
//!    wallet back to the bank's primary receive address. The SDK's
//!    `transfer()` call inside `teardown_one` blocks until the sweep
//!    transition has been broadcast and confirmed.
//! 4. Assert the registry no longer holds the wallet entry — the
//!    primary contract teardown promises.
//! 5. Re-derive a fresh `PlatformWallet` from the captured seed
//!    bytes, sync it, and assert `addr_1`'s on-chain balance is zero.
//!    This is the on-chain proof the sweep actually drained the
//!    address — the registry contract alone could pass even if
//!    `teardown_one` removed the entry without broadcasting (silent
//!    regression of step 5 in the cleanup pipeline). The re-derived
//!    wallet sees only what the chain reports, no cached state.
//!
//! ## Why no bank-balance delta assertion
//!
//! The harness shares one bank wallet across every test in the
//! process. Other tests' sweep transitions can land on the bank's
//! primary receive address inside this test's window (the chain
//! settles them asynchronously), so `bank.total_credits()` measured
//! before vs. after this test's sweep is not a clean delta. PA-004
//! therefore restricts itself to invariants observable on (a) the
//! per-test registry entry and (b) the swept address's on-chain
//! balance. Cross-test bank-balance accounting is out of scope for
//! a single P0 case; an aggregate "bank drain across a run" probe
//! would belong in a separate harness self-test.
//!
//! Why `FUNDING_CREDITS` is bumped: see PA-002's `#3040` note. With
//! the default `[ReduceOutput(0)]` strategy each transition's
//! `output[0]` must clear the chain-time fee (~15M for 1in/1out), and
//! the sweep transition is itself a 1in/1out shape.

use std::time::Duration;

use key_wallet::wallet::initialization::WalletAccountCreationOptions;

use crate::framework::prelude::*;
use crate::framework::wait::{
    wait_for_address_balance_chain_confirmed_n, CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
};

/// Gross credits the bank submits when funding `addr_1`. Bank uses
/// `[ReduceOutput(0)]`; addr_1 receives `FUNDING_CREDITS − bank_fee`.
const FUNDING_CREDITS: u64 = 100_000_000;

/// Lower bound on what addr_1 must receive before the test proceeds.
const FUNDING_FLOOR: u64 = 70_000_000;

/// Per-step deadline for balance observations.
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

#[tokio_shared_rt::test(shared)]
async fn pa_004_sweep_back_drains_to_bank() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    let s = setup().await.expect("e2e setup failed");
    // Capture ctx, wallet id, seed, and the bank's network before
    // teardown consumes the guard. The seed is needed to re-derive
    // a read-only view of `addr_1` for the on-chain balance check
    // after the sweep removes the wallet from the manager.
    let ctx = s.ctx;
    let test_wallet_id = s.test_wallet.id();
    let seed_bytes = s.test_wallet.seed_bytes();
    let network = ctx.bank().network();

    // Fund addr_1, wait for test wallet to observe. This is the
    // value teardown will sweep back.
    let addr_1 = s
        .test_wallet
        .next_unused_address()
        .await
        .expect("derive addr_1");
    ctx.bank()
        .fund_address(&addr_1, FUNDING_CREDITS)
        .await
        .expect("bank.fund_address");
    // Funding precondition gated on the proof-verified chain view
    // (Found-025-immune): a stale local-map 0 would hang this before
    // teardown's sweep transition that drains addr_1.
    wait_for_address_balance_chain_confirmed_n(
        s.ctx.sdk(),
        &addr_1,
        FUNDING_FLOOR,
        CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
        STEP_TIMEOUT,
    )
    .await
    .expect("addr_1 funding never observed");

    let pre_status = ctx.registry().get_status(test_wallet_id);
    assert_eq!(
        pre_status,
        Some(crate::framework::registry::EntryStatus::Active),
        "registry must hold the test wallet as `Active` before teardown"
    );

    // Teardown sweeps the wallet's balance back to the bank and
    // removes the registry entry. The SDK call inside
    // `cleanup::teardown_one` blocks until the sweep transition has
    // been broadcast and confirmed — by the time `teardown` returns,
    // the registry deletion has been persisted.
    s.teardown().await.expect("teardown sweep");

    tracing::info!(
        target: "platform_wallet::e2e::cases::pa_004",
        ?addr_1,
        wallet_id = %hex::encode(test_wallet_id),
        funding = FUNDING_CREDITS,
        "teardown completed; verifying registry cleanup"
    );

    // PA-004 contract 1: registry entry is gone after teardown.
    // `cleanup::teardown_one` only removes the entry on a successful
    // sweep, so a `None` here implies the on-chain transition landed.
    assert!(
        ctx.registry().get_status(test_wallet_id).is_none(),
        "registry must drop the test wallet entry on successful teardown; \
         a residual entry indicates the sweep transition failed"
    );

    // PA-004 contract 2: addr_1's on-chain balance is zero after the
    // sweep. Re-derive the wallet from its seed, sync, and read the
    // balance straight off the chain. The re-derivation deliberately
    // bypasses the cached state of the now-gone TestWallet so the
    // assertion can't pass on stale memory — only on-chain truth.
    let post_sweep = ctx
        .manager()
        .create_wallet_from_seed_bytes(
            network,
            &seed_bytes,
            WalletAccountCreationOptions::Default,
            None,
        )
        .await
        .expect("re-derive post-sweep view of test wallet");
    post_sweep.platform().initialize().await;
    post_sweep
        .platform()
        .sync_balances(None)
        .await
        .expect("post-sweep sync");
    let post_sweep_balances = post_sweep.platform().addresses_with_balances().await;
    let addr_1_post = post_sweep_balances
        .iter()
        .find(|(a, _)| a == &addr_1)
        .map(|(_, b)| *b)
        .unwrap_or(0);
    tracing::info!(
        target: "platform_wallet::e2e::cases::pa_004",
        ?addr_1,
        addr_1_post,
        "post-sweep on-chain balance for funded address"
    );
    assert_eq!(
        addr_1_post, 0,
        "addr_1 on-chain balance must be zero after sweep \
         (sweep transition must have actually drained the address, \
         not just removed the registry entry)"
    );

    // Best-effort cleanup: drop the re-derived wallet from the
    // manager so subsequent tests don't see it. Failure is fine —
    // the wallet has zero balance and no remaining work.
    if let Err(err) = ctx.manager().remove_wallet(&test_wallet_id).await {
        tracing::debug!(
            target: "platform_wallet::e2e::cases::pa_004",
            error = %err,
            "post-sweep cleanup of re-derived wallet failed (best-effort, non-fatal)"
        );
    }
}
