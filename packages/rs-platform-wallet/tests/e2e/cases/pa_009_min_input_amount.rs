//! PA-009 — `min_input_amount` boundary for cleanup (version-source pin).
//! Spec: `tests/e2e/TEST_SPEC.md` §3 "Platform Addresses (PA)" → PA-009.
//! Priority: P2.
//!
//! ## What this test pins
//!
//! `framework/cleanup.rs::min_input_amount(version)` reads
//! `version.dpp.state_transitions.address_funds.min_input_amount`.
//! That field — and ONLY that field — drives the cleanup gate. PA-009
//! pins three properties:
//!
//! 1. The cleanup gate value equals
//!    `PlatformVersion::latest().dpp.state_transitions.address_funds.min_input_amount`.
//!    A future refactor that hardcodes the gate (e.g. `5_000_000`)
//!    would still pass PA-004 / PA-004b, but must fail this assertion.
//! 2. With a wallet total below the gate, teardown returns `Ok` and
//!    no broadcast is attempted (asserted via on-chain balance ≠ 0
//!    after teardown).
//! 3. The gate is positive — protects against an upstream bump that
//!    sets `min_input_amount = 0` and silently disables the gate.
//!
//! ## Why not the spec's literal triplet
//!
//! The spec asks for sub-cases at `min − 1`, `min`, and `min + 1`.
//! PA-004b's module docs explain why the AT/JUST-ABOVE sub-cases are
//! degenerate against the testnet fee market: at the active version's
//! gate (`100_000`), the sweep transition's chain-time fee
//! (~`15_000_000`) far exceeds the available balance, so the sweep
//! ALWAYS fails at chain-time once the gate is crossed and below the
//! fee. The sub-case `balance == min + 1` therefore can't be
//! distinguished from "broadcast attempted, broadcast failed" without
//! either a much larger balance (already covered by PA-004 at
//! `100_000_000`) or a test-only chain-time-fee override (large
//! production change, ruled out by the brief).
//!
//! What PA-009 uniquely contributes vs PA-004b is the version-source
//! assertion (1 above): asserting the gate's value tracks the active
//! `PlatformVersion`, not a stale constant.
//!
//! ## Approach
//!
//! Same Option-A trim pattern as PA-004b — fund, partial-drain to
//! a deterministic residual far below the gate, teardown, observe
//! that no broadcast happened. Distinct test-wallet from PA-004b
//! (each `setup` returns a fresh wallet) so the registry / manager
//! state of one cannot leak into the other.

use std::collections::BTreeMap;
use std::time::Duration;

use dpp::version::PlatformVersion;
use key_wallet::wallet::initialization::WalletAccountCreationOptions;

use crate::framework::cleanup::cleanup_dust_gate;
use crate::framework::prelude::*;

/// Gross credits the bank submits when funding `addr_1`. Same shape
/// as PA-004b; sized well above chain-time fee (~`15_000_000`) so
/// the trim transfer's sink output clears chain-time with margin.
const FUNDING_CREDITS: u64 = 50_000_000;

/// Lower bound on what addr_1 must receive before the test proceeds.
const FUNDING_FLOOR: u64 = 25_000_000;

/// Target residual on `addr_1` after the trim. Identical to PA-004b's
/// constant — both cases pin the BELOW-gate path.
const TARGET_RESIDUAL: u64 = 1_000;

/// Per-step deadline for balance observations.
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

// TODO(QA-V27-007): Re-enable when production fix lands. The assertion at the
// post-trim balance check sees the bank's full balance (~40.8 tDASH) instead
// of the test wallet's residual because PlatformAddressWallet::transfer at
// transfer.rs:160 calls set_address_credit_balance for every address in the
// transition — with no ownership check. Pollutes the source wallet's local
// ledger when transferring to externally-owned addresses (e.g., bank). Same
// unguarded primitive at withdrawal.rs:141 and fund_from_asset_lock.rs:129.
// Severity: HIGH for tests/SDK consumers; MEDIUM-LOW in production sweep
// path (signing prevents on-chain leak). Fix sketch (~6 LOC ownership filter)
// in TEST_SPEC.md V27-007 section.
#[tokio_shared_rt::test(shared)]
#[ignore = "FAILING — production bug in PlatformAddressWallet::transfer pollutes local ledger with non-owned addresses. See TEST_SPEC.md (V27-007) and TODO comment below."]
async fn pa_009_cleanup_gate_tracks_platform_version_min_input_amount() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    // ---- Property (1): cleanup gate equals the active PlatformVersion's
    // min_input_amount. This is what distinguishes PA-009 from PA-004b. ----
    let version = PlatformVersion::latest();
    let cleanup_gate = cleanup_dust_gate(version);
    let version_field = version.dpp.state_transitions.address_funds.min_input_amount;
    assert_eq!(
        cleanup_gate, version_field,
        "PA-009: cleanup_dust_gate must equal \
         PlatformVersion::latest().dpp.state_transitions.address_funds.min_input_amount; \
         got cleanup_gate={cleanup_gate}, version_field={version_field}. \
         A divergence means the cleanup path has drifted from the protocol's \
         own gate definition."
    );

    // ---- Property (3): gate must be positive. A zero would silently
    // disable the gate, sweeping every wallet regardless of balance. ----
    assert!(
        cleanup_gate > 0,
        "PA-009: cleanup gate must be positive; \
         a zero gate would silently sweep every wallet"
    );

    // Sanity: TARGET_RESIDUAL < gate so the below-gate path is
    // exercised. Same drift guard PA-004b carries.
    assert!(
        TARGET_RESIDUAL < cleanup_gate,
        "PA-009: TARGET_RESIDUAL ({TARGET_RESIDUAL}) must be < cleanup_gate \
         ({cleanup_gate}); a protocol-version bump moved the gate below our target"
    );

    let s = setup().await.expect("e2e setup failed");
    let ctx = s.ctx;
    let test_wallet_id = s.test_wallet.id();
    let seed_bytes = s.test_wallet.seed_bytes();
    let network = ctx.bank().network();

    // ---- Step 1: bank-fund addr_1. ----
    let addr_1 = s
        .test_wallet
        .next_unused_address()
        .await
        .expect("derive addr_1");
    ctx.bank()
        .fund_address(&addr_1, FUNDING_CREDITS)
        .await
        .expect("bank.fund_address");
    wait_for_balance(&s.test_wallet, &addr_1, FUNDING_FLOOR, STEP_TIMEOUT)
        .await
        .expect("addr_1 funding never observed");

    s.test_wallet
        .sync_balances()
        .await
        .expect("sync after fund");
    let balances = s.test_wallet.balances().await;
    let addr_1_balance = balances.get(&addr_1).copied().unwrap_or(0);
    assert!(
        addr_1_balance >= FUNDING_FLOOR,
        "PA-009: addr_1 post-fund balance ({addr_1_balance}) below FUNDING_FLOOR \
         ({FUNDING_FLOOR}); abort"
    );

    // ---- Step 2: trim addr_1 to TARGET_RESIDUAL via auto-select transfer
    // to the bank's primary receive address. Sink choice matches PA-004b. ----
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

    s.test_wallet
        .sync_balances()
        .await
        .expect("sync after trim");
    let total_post_trim = s.test_wallet.total_credits().await;
    let post_trim = s.test_wallet.balances().await;
    let addr_1_residual = post_trim.get(&addr_1).copied().unwrap_or(0);

    tracing::info!(
        target: "platform_wallet::e2e::cases::pa_009",
        ?addr_1,
        addr_1_residual,
        total_post_trim,
        cleanup_gate,
        version_field,
        "post-trim wallet state"
    );

    assert_eq!(
        addr_1_residual, TARGET_RESIDUAL,
        "PA-009: trim transfer must leave addr_1 with exactly TARGET_RESIDUAL \
         ({TARGET_RESIDUAL}) under the auto-select Σ inputs == Σ outputs invariant"
    );
    assert!(
        total_post_trim < cleanup_gate,
        "PA-009: post-trim wallet total ({total_post_trim}) must be < cleanup_gate \
         ({cleanup_gate}); a stray balance on a non-addr_1 address violates the \
         precondition for the below-gate cleanup contract"
    );

    // ---- Step 3: teardown — must NOT broadcast. ----
    s.teardown()
        .await
        .expect("teardown should succeed when total < cleanup_gate");

    // ---- Property (2): below-gate teardown leaves on-chain balance intact. ----
    assert!(
        ctx.registry().get_status(test_wallet_id).is_none(),
        "PA-009: registry must drop the test wallet entry on successful below-gate teardown"
    );

    let post_sweep = ctx
        .manager()
        .create_wallet_from_seed_bytes(
            network,
            seed_bytes,
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
        target: "platform_wallet::e2e::cases::pa_009",
        ?addr_1,
        addr_1_post,
        "post-teardown on-chain balance for residual address"
    );

    assert_eq!(
        addr_1_post, TARGET_RESIDUAL,
        "PA-009: on-chain addr_1 balance must equal TARGET_RESIDUAL ({TARGET_RESIDUAL}) \
         after a below-gate teardown — proves no sweep transition was broadcast. \
         The cleanup gate (sourced from PlatformVersion's min_input_amount) gated \
         the sweep correctly."
    );

    if let Err(err) = ctx.manager().remove_wallet(&test_wallet_id).await {
        tracing::debug!(
            target: "platform_wallet::e2e::cases::pa_009",
            error = %err,
            "post-teardown unregister of re-derived wallet failed (best-effort)"
        );
    }
}
