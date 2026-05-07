//! CR-001 — SPV mn-list sync readiness.
//!
//! Spec: `tests/e2e/TEST_SPEC.md` (### Core (CR) → CR-001).
//!
//! Pins the SPV-readiness contract: the mn-list manager reaches
//! `SyncState::Synced` within 180 s, the synced height is > 0, and the
//! SPV runtime is in a started (running) state on return.
//!
//! The harness already calls `wait_for_mn_list_synced` during
//! `E2eContext::build`; this test re-asserts the same contract from the
//! test-body perspective to keep the pin explicit and independently
//! verifiable. The call returns immediately when the harness already
//! cleared the gate.
//!
//! Mirrors DET's `test_spv_sync_and_create_wallet` at
//! `dash-evo-tool/tests/backend-e2e/spv_wallet.rs:14`.

use std::time::Duration;

use crate::framework::prelude::*;
use crate::framework::spv::wait_for_mn_list_synced;

/// Maximum time this test body will wait for mn-list sync. The
/// harness gate already ran at init — this is an independent ceiling
/// that fires only if the sync regresses between init and the test body
/// (extremely unlikely, but the spec pins <= 180 s explicitly).
const MN_LIST_SYNC_TIMEOUT: Duration = Duration::from_secs(180);

#[ignore = "CR-001 — needs testnet + SPV runtime. \
            Set PLATFORM_WALLET_E2E_DISABLE_SPV=0 (or unset) and supply \
            DAPI endpoints via PLATFORM_WALLET_E2E_DAPI_ADDRESSES. \
            Mirrors DET's test_spv_sync_and_create_wallet."]
#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn cr_001_spv_mn_list_sync_readiness() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    // Respect the operator escape hatch — when SPV is disabled the mn-list
    // will never sync; skip with an informative message rather than burn
    // the full timeout.
    let ctx = E2eContext::init().await.expect("E2eContext::init failed");

    if ctx.config.disable_spv {
        tracing::info!(
            target: "platform_wallet::e2e::cases::cr_001",
            "PLATFORM_WALLET_E2E_DISABLE_SPV is set — skipping CR-001 \
             (mn-list will never sync without a live SPV runtime)"
        );
        return;
    }

    let s = crate::framework::setup().await.expect("setup failed");

    // Step 1: assert the SPV runtime is live. The harness only populates
    // `ctx.spv()` when `disable_spv` is false, so `None` here is a
    // harness bug worth surfacing with a clear message.
    let spv = s.ctx.spv().expect(
        "PRE-pin violated: ctx.spv() is None but PLATFORM_WALLET_E2E_DISABLE_SPV \
             is not set — SPV runtime was not started by the harness",
    );

    // Step 2: wait <= 180s for mn-list sync. The harness already ran this
    // during init; this call returns immediately if already synced.
    wait_for_mn_list_synced(spv, MN_LIST_SYNC_TIMEOUT)
        .await
        .expect("wait_for_mn_list_synced failed within 180 s");

    // Step 3: read the mn-list height from the live sync progress.
    let progress = spv.sync_progress().await.expect(
        "PRE-pin violated: sync_progress() returned None after \
             wait_for_mn_list_synced succeeded — SPV client must be running",
    );
    let mn = progress
        .masternodes()
        .expect("SyncProgress::masternodes() failed after successful mn-list sync");
    let mn_height = mn.current_height();

    tracing::info!(
        target: "platform_wallet::e2e::cases::cr_001",
        mn_height,
        state = ?mn.state(),
        "CR-001: mn-list synced"
    );

    // Assertion 1: mn-list height > 0 (proves the client synced real data,
    // not just initialised with a zero-height placeholder).
    assert!(
        mn_height > 0,
        "POST-pin violated: mn-list height is 0 after sync — \
         the mn-list manager must advance at least one block to report Synced. \
         Check SPV peer connectivity and mn-list initial-sync logic."
    );

    // Assertion 2: SPV runtime is started (running). `is_started()` returns
    // `true` when the internal DashSpvClient is initialised and the sync
    // loop is live. This is the available proxy for the spec's "Ready state"
    // contract (SpvHealth is not yet a public type — see TEST_SPEC.md CR-001
    // harness-extensions note).
    assert!(
        spv.is_started(),
        "POST-pin violated: SpvRuntime::is_started() is false after \
         wait_for_mn_list_synced returned Ok — the runtime must remain \
         started (running) throughout the test session."
    );

    s.teardown().await.expect("teardown");
}
