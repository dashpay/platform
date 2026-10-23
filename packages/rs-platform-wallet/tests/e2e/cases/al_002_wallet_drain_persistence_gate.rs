//! AL-002 — Core→Shielded wallet drain: persistence-gate refusal (coverage gap pin).
//! Spec: `tests/e2e/TEST_SPEC.md` §3 "### Asset Lock (AL)" → AL-002.
//! Priority: P1.
//!
//! The wallet drain (`shielded_wallet_drain_preview` /
//! `register_wallet_drain_operation` / `shielded_wallet_drain_operation_status`)
//! is the newest asset-lock flow, and the suite cannot drive it end to end:
//! every drain entry point first requires `PersistenceCapabilities::WALLET_DRAIN`
//! (which includes `WALLET_RESTORE` and `ASSET_LOCK_OPERATION_IDS`) plus a
//! backend that commits each store inline. The harness persister advertises
//! only `ATOMIC_CHANGESETS | SHIELDED_VIEWING_KEYS`, and `SqlitePersister`
//! attests the missing capabilities only further up the #5150 stack
//! (#5210 adds `WALLET_RESTORE`, #5528 adds `ASSET_LOCK_OPERATION_IDS`).
//!
//! Until then this case pins the gate: the read-only status query must
//! refuse with a typed `PlatformWalletError::Persistence` that names the
//! missing capabilities. It never touches the network beyond setup and
//! moves no funds. When it turns RED because the drain is no longer
//! refused, replace it with a real drain case (TEST_SPEC AL-002).
//!
//! Expected outcome: PASS (the refusal is the pinned behaviour).

#![cfg(feature = "shielded")]

use platform_wallet::error::PlatformWalletError;

use crate::framework::prelude::*;

/// Arbitrary operation id: the gate refuses before any lookup.
const OPERATION_ID: [u8; 16] = [0xA1; 16];

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn al_002_wallet_drain_persistence_gate() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    let s = setup().await.expect("e2e setup failed");

    let status = s
        .test_wallet
        .platform_wallet()
        .shielded_wallet_drain_operation_status(OPERATION_ID)
        .await;
    match &status {
        Err(PlatformWalletError::Persistence(msg)) => {
            assert!(
                msg.contains("wallet drain requires persistence capabilities")
                    && msg.contains("wallet_restore")
                    && msg.contains("asset_lock_operation_ids"),
                "the refusal must name the missing drain capabilities \
                 (wallet_restore, asset_lock_operation_ids); observed {msg:?}"
            );
        }
        other => panic!(
            "AL-002 gap pin: expected the typed Persistence refusal of the wallet drain; \
             observed {other:?}. If the harness persister now attests WALLET_DRAIN, \
             replace this pin with a real drain case (TEST_SPEC AL-002)."
        ),
    }

    s.teardown().await.expect("teardown");
}
