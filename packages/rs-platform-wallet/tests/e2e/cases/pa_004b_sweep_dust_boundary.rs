//! PA-004b — Sweep dust-threshold boundary (below-gate sub-case).
//! Spec: `tests/e2e/TEST_SPEC.md` §3 "Platform Addresses (PA)" → PA-004b.
//! Priority: P2.
//!
//! ## What this test pins
//!
//! `framework/cleanup.rs::teardown_one` gates the platform-address
//! sweep on `total_credits() >= min_input_amount(version)`. Below
//! that gate, no broadcast may be attempted — the wallet is
//! de-registered without touching its on-chain balance.
//!
//! Spec asked for a triplet (`gate − 1`, `gate`, `gate + 1`). What
//! we actually pin in this single case is the BELOW-gate path:
//!
//! - Setup such that `total_credits()` is well below the active
//!   `min_input_amount` (currently `100_000`).
//! - Call teardown.
//! - Assert `Ok(())`, registry cleared, on-chain balance NOT zero
//!   (no sweep transition was broadcast).
//!
//! The AT/ABOVE sub-cases are degenerate against the harness and the
//! testnet fee market:
//!
//!   1. `balance == gate` and `gate + 1`: at the active version's gate
//!      (`100_000` credits) the harness DOES attempt a sweep, but the
//!      sweep transition's chain-time fee (~`15_000_000` credits per
//!      PA-002's empirical analysis) far exceeds the available
//!      balance, so the broadcast fails and `teardown_one` returns
//!      `Err`. PA-004 already pins the "well-above-fee" path with
//!      `100_000_000` credits funded, which is the realistic operator
//!      contract; pinning "above gate but below chain-fee" would
//!      leave a permanently-stuck orphan on every run with no
//!      recovery path on testnet.
//!   2. `balance == gate` exactly: requires either a test-only
//!      `set_address_credit_balance` override (Option B in the brief)
//!      or a multi-step calibrate-and-trim against fluctuating
//!      chain-time fees. Both are more invasive than the BELOW-gate
//!      path which is the contract that distinguishes PA-004b from
//!      PA-004.
//!
//! Approach used: Option A from the brief — real bank funding + real
//! partial drain to land below the gate. ±tolerance is fine because
//! the assertion is BINARY (below or not), and `Σ inputs == Σ outputs`
//! is the post-fix invariant (commits `aaf8be74ee`, `9ea9e7033c`):
//! `Auto` selection draws exactly `Σ outputs` from inputs, so the
//! residual on `addr_1` after the trim transfer is deterministic up to
//! the chain-time fee that lands on the sink output (the
//! `[ReduceOutput(0)]` strategy charges fee against output[0], not
//! against the residual).

use std::collections::BTreeMap;
use std::time::Duration;

use dash_sdk::platform::address_sync::AddressSyncConfig;
use dpp::version::PlatformVersion;
use key_wallet::wallet::initialization::WalletAccountCreationOptions;

use crate::framework::cleanup::cleanup_dust_gate;
use crate::framework::prelude::*;
use crate::framework::wait::{
    wait_for_address_balance_chain_confirmed_n, CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
};

/// Gross credits the bank submits when funding `addr_1`. Sized well
/// above the chain-time fee (~`15_000_000`) so the trim transfer's
/// output[0] (the sink) clears chain-time fee with margin.
const FUNDING_CREDITS: u64 = 50_000_000;

/// Lower bound on what addr_1 must receive before the test proceeds.
/// Wide margin so the wait isn't sensitive to bank-fee fluctuations.
const FUNDING_FLOOR: u64 = 25_000_000;

/// Target residual for `addr_1` AFTER the trim transfer. Picked far
/// below the active `min_input_amount` (`100_000`) so a one-off bump
/// of the protocol's gate doesn't accidentally flip this case from
/// "below-gate" to "at/above-gate".
///
/// Pinned at `1_000` not `99_999` for two reasons:
///   - Defensive against an upstream gate decrease (any gate ≥ 1_000
///     keeps this case below).
///   - Auto-select's `Σ inputs == Σ outputs` invariant lands the
///     residual exactly at this value; a smaller target leaves less
///     stranded on testnet across runs.
const TARGET_RESIDUAL: u64 = 1_000;

/// Per-step deadline for balance observations.
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

#[tokio_shared_rt::test(shared)]
async fn pa_004b_sweep_below_dust_gate_no_broadcast() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    let s = setup().await.expect("e2e setup failed");

    // Read the active version's gate from the same source `cleanup.rs`
    // uses, so a protocol-version bump shifts both ends in lockstep.
    let dust_gate = cleanup_dust_gate(PlatformVersion::latest());
    assert!(
        TARGET_RESIDUAL < dust_gate,
        "PA-004b: TARGET_RESIDUAL ({TARGET_RESIDUAL}) must be < cleanup_dust_gate \
         ({dust_gate}); a protocol-version bump moved the gate below our target"
    );

    let ctx = s.ctx;
    let test_wallet_id = s.test_wallet.id();
    let seed_bytes = s.test_wallet.seed_bytes();
    let network = ctx.bank().network();

    // ---- Step 1: bank-fund addr_1 with comfortable headroom. ----
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
    // the trim transfer that consumes addr_1's funding.
    wait_for_address_balance_chain_confirmed_n(
        s.ctx.sdk(),
        &addr_1,
        FUNDING_FLOOR,
        CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
        STEP_TIMEOUT,
    )
    .await
    .expect("addr_1 funding never observed");

    // Refresh and snapshot the precise post-fund balance — needed for
    // the trim's auto-select sizing.
    s.test_wallet
        .sync_balances()
        .await
        .expect("sync after fund");
    let balances = s.test_wallet.balances().await;
    let addr_1_balance = balances.get(&addr_1).copied().unwrap_or(0);
    assert!(
        addr_1_balance >= FUNDING_FLOOR,
        "PA-004b: addr_1 post-fund balance ({addr_1_balance}) below FUNDING_FLOOR \
         ({FUNDING_FLOOR}); abort"
    );

    // ---- Step 2: trim addr_1 to TARGET_RESIDUAL via a transfer to the
    // bank's primary receive address. Auto-select with `[ReduceOutput(0)]`
    // draws exactly `Σ outputs` from inputs (commits aaf8be74ee /
    // 9ea9e7033c). Sending `addr_1_balance - TARGET_RESIDUAL` therefore
    // leaves precisely `TARGET_RESIDUAL` on addr_1; chain-time fee
    // lands on output[0] (the sink), not on the residual.
    let trim_amount = addr_1_balance
        .checked_sub(TARGET_RESIDUAL)
        .expect("FUNDING_CREDITS sized so the trim subtract cannot underflow");
    let sink = *ctx.bank().primary_receive_address();
    let mut outputs: BTreeMap<_, _> = BTreeMap::new();
    outputs.insert(sink, trim_amount);

    s.test_wallet
        .transfer(outputs)
        .await
        .expect("trim transfer to sink");

    // The transfer call awaits broadcast confirmation, so on return
    // the wallet's cached balance for addr_1 should already reflect
    // the residual. Sync explicitly so the assertion below pins
    // post-broadcast state.
    s.test_wallet
        .sync_balances()
        .await
        .expect("sync after trim");
    let post_trim = s.test_wallet.balances().await;
    let addr_1_residual = post_trim.get(&addr_1).copied().unwrap_or(0);

    // Sum over the test wallet's own addresses ONLY. `addr_1` is the
    // only address this test ever derived on `s.test_wallet`, so the
    // test-wallet total is `addr_1_residual` by construction. We do
    // NOT read `total_credits()` here — its aggregate is inflated by
    // V27-007 (`PlatformAddressWallet::transfer` writes the bank's
    // primary receive address into the source wallet's local ledger
    // when we trim to the bank), pulling in credits the test wallet
    // does not own. The bank is process-shared; its balance is not
    // part of the PA-004b contract.
    let test_wallet_total = addr_1_residual;

    tracing::info!(
        target: "platform_wallet::e2e::cases::pa_004b",
        ?addr_1,
        addr_1_residual,
        test_wallet_total,
        dust_gate,
        "post-trim wallet state"
    );

    // The residual on addr_1 must equal TARGET_RESIDUAL exactly under
    // the post-fix `Σ inputs == Σ outputs` invariant. Pinning equality
    // (not `<= TARGET_RESIDUAL + tol`) here is what catches a future
    // regression of the auto-select fix.
    assert_eq!(
        addr_1_residual, TARGET_RESIDUAL,
        "PA-004b: trim transfer should leave addr_1 with exactly TARGET_RESIDUAL \
         ({TARGET_RESIDUAL}); auto-select Σ inputs == Σ outputs invariant violated"
    );

    // The test wallet's total (over OWNED addresses only) must be
    // below the gate. This is the precondition the cleanup-gate test
    // rests on.
    assert!(
        test_wallet_total < dust_gate,
        "PA-004b: post-trim test-wallet total ({test_wallet_total}) must be < dust_gate \
         ({dust_gate}); a stray balance on a non-addr_1 address owned by the test \
         wallet violates the precondition for the below-gate cleanup contract"
    );

    // ---- Step 3: teardown. ----
    // The gate is below dust_gate; cleanup.rs MUST NOT broadcast a
    // sweep transition. teardown_one calls sync_balances first then
    // checks `total >= dust_gate`. With total = TARGET_RESIDUAL,
    // sweep_platform_addresses is skipped; identity / core /
    // asset_lock / shielded sweeps are all noops; registry.remove
    // and manager.remove_wallet run unconditionally.
    s.teardown()
        .await
        .expect("teardown should succeed when total < dust_gate (no broadcast attempted)");

    // ---- Step 4: contract assertions. ----
    // (a) registry entry is removed.
    assert!(
        ctx.registry().get_status(test_wallet_id).is_none(),
        "PA-004b: registry must drop the test wallet entry on successful below-gate \
         teardown (no sweep was attempted, but the wallet's lifecycle still completes)"
    );

    // (b) on-chain addr_1 balance is unchanged (NOT zero). This is the
    // distinguishing assertion vs PA-004 — there, the sweep DID run and
    // post-balance is zero. Here, no sweep attempt happened, so the
    // residual stayed on chain.
    //
    // Re-derive the wallet from the captured seed to bypass any cached
    // state of the gone TestWallet. Read straight off chain.
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
    // Use full_rescan_after_time_s=0 — forces a full historical scan.
    // sync_balances(None) on a fresh re-derived wallet anchors the "recent
    // zone" query at current chain tip; if addr_1's balance was committed
    // below the recent window, sync returns empty and skips the compacted
    // scan. See QA-014 investigation /tmp/qa-014-pa-009-rederive-sync-gap.md.
    post_sweep
        .platform()
        .sync_balances(Some(AddressSyncConfig {
            full_rescan_after_time_s: 0,
            ..AddressSyncConfig::default()
        }))
        .await
        .expect("post-sweep sync");
    let post_sweep_balances = post_sweep.platform().addresses_with_balances().await;
    let addr_1_post = post_sweep_balances
        .iter()
        .find(|(a, _)| a == &addr_1)
        .map(|(_, b)| *b)
        .unwrap_or(0);

    tracing::info!(
        target: "platform_wallet::e2e::cases::pa_004b",
        ?addr_1,
        addr_1_post,
        "post-teardown on-chain balance for residual address"
    );

    assert_eq!(
        addr_1_post, TARGET_RESIDUAL,
        "PA-004b: on-chain addr_1 balance must equal TARGET_RESIDUAL ({TARGET_RESIDUAL}) \
         after a below-gate teardown — i.e. NO sweep transition was broadcast. \
         A zero here means the gate was bypassed and a sweep DID run; a value other \
         than {TARGET_RESIDUAL} means something else moved on-chain"
    );

    // Best-effort manager unregister of the re-derived wallet so
    // subsequent tests don't see it. Failure is fine — the wallet has
    // no more work to do.
    if let Err(err) = ctx.manager().remove_wallet(&test_wallet_id).await {
        tracing::debug!(
            target: "platform_wallet::e2e::cases::pa_004b",
            error = %err,
            "post-teardown unregister of re-derived wallet failed (best-effort)"
        );
    }
}
