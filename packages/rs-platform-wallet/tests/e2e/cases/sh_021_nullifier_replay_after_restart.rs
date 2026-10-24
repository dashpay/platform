//! SH-021 — ADVERSARIAL: nullifier replay after a confirmed spend —
//! backend MUST reject [INJECT].
//! Spec: `tests/e2e/TEST_SPEC.md` §3 → SH-021. Priority: P0
//! (consensus-critical). CRITICAL-if-it-fails.
//!
//! Attack: capture a note, spend it (confirmed), then rebuild a fresh
//! transition spending the SAME now-spent note (via the build-against-note
//! seam, which skips the local spent-state guard) and re-broadcast. The
//! nullifier is permanently in Drive's spent set, so the replay MUST fail
//! (`NullifierAlreadySpentError`, code 40901) regardless of client state.
//!
//! RED if the replay is accepted (double-spend via replay).

#![cfg(feature = "shielded")]

// TODO(SH-021): add a Type-16 replay arm once the harness has a
// build-transfer-against-notes seam (only Type 17 has one today).

use std::sync::Arc;
use std::time::Duration;

use dash_sdk::platform::Fetch;
use dash_sdk::query_types::AddressInfo;
use platform_wallet::wallet::shielded::{
    FileBackedShieldedStore, NetworkShieldedCoordinator, ShieldedStore, SubwalletId,
};

use dpp::shielded::compute_minimum_shielded_fee;
use dpp::version::PlatformVersion;

use crate::framework::prelude::*;
use crate::framework::shielded::{
    adversarial_enabled, assert_adv_rejected, broadcast_raw, build_unshield_st_against_notes,
    new_file_backed_coordinator, shielded_prover, teardown_sweep_shielded, unspent_notes,
    wait_for_shielded_balance, ShieldedHandle,
};
use crate::framework::wait::{
    wait_for_address_balance_chain_confirmed_n, CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
};

// Clears `SHIELD_AMOUNT + 1e9 reserve + ~1.63e8 shield fee` (see `sh_010`),
// with 1e8 headroom.
const FUNDING_CREDITS: u64 = 2_382_851_200;
const SHIELD_AMOUNT: u64 = 1_120_000_000;
const UNSHIELD_AMOUNT: u64 = 20_000_000;
const STEP_TIMEOUT: Duration = Duration::from_secs(60);
/// Consensus commit needs block production + proof — longer than a per-step gate.
const COMMIT_TIMEOUT: Duration = Duration::from_secs(120);

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn sh_021_nullifier_replay_after_restart() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    if !adversarial_enabled() {
        tracing::info!(
            target: "platform_wallet::e2e::cases::sh_021",
            "PLATFORM_WALLET_E2E_SHIELDED_ADVERSARIAL set to a falsy value — abuse case opted out (no-op pass)"
        );
        return;
    }

    let s = setup().await.expect("e2e setup failed");
    let prover = shielded_prover();
    let coordinator = new_file_backed_coordinator(&s.test_wallet, &s.ctx.workdir)
        .await
        .expect("private persisted coordinator");
    s.test_wallet
        .platform_wallet()
        .bind_shielded(&s.test_wallet.seed_bytes(), &[0], &coordinator)
        .await
        .expect("bind shielded");
    let handle = ShieldedHandle {
        coordinator,
        accounts: vec![0],
    };

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
    wait_for_address_balance_chain_confirmed_n(
        s.ctx.sdk(),
        &addr_1,
        FUNDING_CREDITS,
        CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
        STEP_TIMEOUT,
    )
    .await
    .expect("addr_1 funding never observed");
    s.test_wallet
        .sync_balances()
        .await
        .expect("pre-shield sync");
    s.test_wallet
        .platform_wallet()
        .shielded_shield_from_account(
            &handle.coordinator,
            0,
            0,
            SHIELD_AMOUNT,
            s.test_wallet.address_signer(),
            prover,
        )
        .await
        .expect("shield_from_account");
    wait_for_shielded_balance(&s.test_wallet, &handle, 0, SHIELD_AMOUNT, STEP_TIMEOUT)
        .await
        .expect("shielded balance never reached SHIELD_AMOUNT");

    // Capture the note BEFORE spending so the replay can rebuild against
    // it after it's confirmed-spent.
    let notes = unspent_notes(&s.test_wallet, &handle, 0)
        .await
        .expect("capture unspent notes");
    assert!(!notes.is_empty(), "expected one synced note");
    let captured = vec![notes[0].clone()];

    // First spend through the real wallet path (confirmed).
    let dst = s.test_wallet.next_unused_address().await.expect("dst");
    let dst_b32 = dst.to_bech32m_string(s.ctx.bank().network());
    s.test_wallet
        .platform_wallet()
        .shielded_unshield_to(
            &handle.coordinator,
            &s.test_wallet.seed_bytes(),
            0,
            &dst_b32,
            UNSHIELD_AMOUNT,
            prover,
        )
        .await
        .expect("first unshield must succeed");
    wait_for_address_balance_chain_confirmed_n(
        s.ctx.sdk(),
        &dst,
        UNSHIELD_AMOUNT,
        CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
        STEP_TIMEOUT,
    )
    .await
    .expect("first unshield destination never observed");

    // Restart the coordinator against its persisted SQLite tree and reload the
    // wallet's shielded snapshot through the normal bind path. Never reset it.
    handle.sync().await;
    s.test_wallet
        .platform_wallet()
        .flush_persist()
        .expect("flush shielded snapshot before restart");
    let path = handle.coordinator.db_path().to_path_buf();
    drop(handle);
    let store = FileBackedShieldedStore::open_path(&path, 100).expect("reopen persisted tree");
    let coordinator = Arc::new(NetworkShieldedCoordinator::new(
        s.test_wallet.platform_wallet().sdk_arc(),
        s.ctx.bank().network(),
        path,
        store,
    ));
    assert!(
        s.test_wallet
            .platform_wallet()
            .bind_shielded_from_persisted(&[0], &coordinator)
            .await
            .expect("reload persisted shielded binding"),
        "persisted binding must exist"
    );
    let handle = ShieldedHandle {
        coordinator,
        accounts: vec![0],
    };
    handle.sync().await;
    let id = SubwalletId::new(s.test_wallet.id(), 0);
    let restored = handle
        .coordinator
        .store()
        .read()
        .await
        .get_all_notes(id)
        .expect("restored notes");
    assert!(
        restored
            .iter()
            .any(|note| note.nullifier == captured[0].nullifier && note.is_spent),
        "restart/resync must preserve the spent nullifier"
    );

    // Replay: rebuild a fresh transition against the now-spent captured
    // note and broadcast. The witness still resolves (the commitment is
    // in the tree), but the nullifier is already spent on-chain.
    let exact_fee = compute_minimum_shielded_fee(1, s.ctx.sdk().version())
        .expect("compute_minimum_shielded_fee");
    let dst2 = s.test_wallet.next_unused_address().await.expect("dst2");
    let replay_st = build_unshield_st_against_notes(
        &s.test_wallet,
        &handle,
        0,
        &dst2,
        UNSHIELD_AMOUNT,
        exact_fee,
        &captured,
    )
    .await
    .expect("rebuild replay against spent note");
    let replay = broadcast_raw(s.ctx.sdk(), &replay_st).await;
    assert_adv_rejected(
        s.ctx.sdk(),
        "SH-021",
        &replay,
        &replay_st,
        COMMIT_TIMEOUT,
        &[40901],
    )
    .await;
    assert_eq!(
        AddressInfo::fetch(s.ctx.sdk(), dst2)
            .await
            .expect("replay destination balance")
            .map_or(0, |info| info.balance),
        0,
        "replay must not credit dst2"
    );
    handle.sync().await;
    let notes = handle
        .coordinator
        .store()
        .read()
        .await
        .get_all_notes(id)
        .expect("post-replay notes");
    assert!(notes
        .iter()
        .any(|note| note.nullifier == captured[0].nullifier && note.is_spent));

    let bank_addr = s
        .ctx
        .bank()
        .primary_receive_address()
        .to_bech32m_string(s.ctx.bank().network());
    teardown_sweep_shielded(&s.test_wallet, &handle, &bank_addr).await;
    s.teardown().await.expect("teardown");
}
