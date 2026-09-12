//! CR-001 — SPV mn-list sync readiness.
//!
//! Spec: `tests/e2e/TEST_SPEC.md` (### Core (CR) → CR-001).
//!
//! Pins the SPV-readiness contract: the mn-list manager reaches
//! `SyncState::Synced`, the synced height is > 0, and the SPV runtime
//! is in a started (running) state on return. The spec's informational
//! warm-cache target is 180 s; the helper internally raises every
//! request to its `COLD_CACHE_TIMEOUT_FLOOR` (600 s) so the actual
//! wait can be longer on a cold testnet cache.
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

use crate::framework::config::{spv_disabled_from_env, vars};
use crate::framework::prelude::*;
use crate::framework::spv::wait_for_mn_list_synced;

/// Spec's informational warm-cache target for mn-list sync. NOT a hard
/// ceiling: `wait_for_mn_list_synced` raises every request to its
/// internal `COLD_CACHE_TIMEOUT_FLOOR` (600 s — see
/// `framework::spv::wait_for_mn_list_synced`) and emits an `info!`
/// when it does. The real wait is bounded by the floor, not by this
/// constant.
const MN_LIST_SYNC_TIMEOUT: Duration = Duration::from_secs(180);

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
    if spv_disabled_from_env() {
        tracing::info!(
            target: "platform_wallet::e2e::cases::cr_001",
            var = vars::DISABLE_SPV,
            "SPV disabled via env — skipping CR-001 \
             (mn-list will never sync without a live SPV runtime)"
        );
        return;
    }

    let s = crate::framework::setup().await.expect("setup failed");

    // Step 1: bind the SPV runtime. `E2eContext::build` always starts
    // SPV unconditionally, so `ctx.spv()` is `Some` in every supported
    // configuration; the `expect` is defence-in-depth in case a future
    // harness change makes the runtime optional.
    let spv = s
        .ctx
        .spv()
        .expect("PRE-pin violated: ctx.spv() is None — harness must always start SPV");

    // Step 2: re-pin mn-list sync from the test body. The harness
    // already ran this at init, so the call returns immediately when
    // already synced; on a cold cache the helper waits up to its
    // `COLD_CACHE_TIMEOUT_FLOOR` (600 s).
    wait_for_mn_list_synced(spv, s.ctx.mn_list_observer(), MN_LIST_SYNC_TIMEOUT)
        .await
        .expect("wait_for_mn_list_synced failed before the cold-cache floor elapsed");

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
