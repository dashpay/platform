use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::task::Poll;

use super::balance::{
    ShieldedBalanceSource, ShieldedLocalBalanceSnapshot, ShieldedLocalBalanceState,
};
use super::{
    FileBackedShieldedStore, NetworkShieldedCoordinator, OrchardKeySet, ShieldedNote,
    ShieldedStore, SubwalletId,
};
use crate::changeset::{ShieldedSubwalletStartState, ShieldedSyncStartState};
use crate::wallet::persister::{NoPlatformPersistence, WalletPersister};
use drive_proof_verifier::types::{ShieldedEncryptedNotes, ShieldedEncryptedNotesQuery};

fn tree_path() -> PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!("shielded_local_balance_{nonce}.sqlite"))
}

fn coordinator(path: &std::path::Path) -> NetworkShieldedCoordinator {
    NetworkShieldedCoordinator::new(
        Arc::new(dash_sdk::Sdk::new_mock()),
        dashcore::Network::Testnet,
        path.to_path_buf(),
        FileBackedShieldedStore::open_path(path, 100).expect("open test store"),
    )
}

async fn register(coordinator: &NetworkShieldedCoordinator, wallet_id: [u8; 32], accounts: &[u32]) {
    let views = accounts
        .iter()
        .map(|account| {
            (
                *account,
                OrchardKeySet::from_seed(&[42; 64], dashcore::Network::Testnet, *account)
                    .expect("test viewing key")
                    .viewing_keys(),
            )
        })
        .collect();
    coordinator
        .register_wallet(
            wallet_id,
            views,
            WalletPersister::new(wallet_id, Arc::new(NoPlatformPersistence)),
        )
        .await;
}

fn note(n: u8, value: u64, is_spent: bool) -> ShieldedNote {
    ShieldedNote {
        position: n as u64,
        cmx: [n; 32],
        nullifier: [n; 32],
        block_height: 1,
        is_spent,
        value,
        note_data: vec![n; 115],
    }
}

async fn ready(
    coordinator: &NetworkShieldedCoordinator,
    wallet: [u8; 32],
) -> ShieldedLocalBalanceSnapshot {
    match coordinator
        .local_balance_snapshot(wallet)
        .await
        .expect("snapshot")
    {
        ShieldedLocalBalanceState::Ready(snapshot) => snapshot,
        other => panic!("expected ready ledger, got {other:?}"),
    }
}

#[tokio::test]
async fn local_balance_distinguishes_availability_and_explicit_empty_scan() {
    let path = tree_path();
    let coordinator = coordinator(&path);
    let wallet = [1; 32];
    assert_eq!(
        coordinator.local_balance_snapshot(wallet).await.unwrap(),
        ShieldedLocalBalanceState::Unbound
    );
    register(&coordinator, wallet, &[0, 1, 2]).await;
    assert_eq!(
        coordinator.local_balance_snapshot(wallet).await.unwrap(),
        ShieldedLocalBalanceState::RestoreIncomplete
    );
    coordinator.mark_hydrated(wallet, true).await;
    let fresh = ready(&coordinator, wallet).await;
    assert_eq!(fresh.accounts.len(), 3);
    assert!(fresh
        .accounts
        .values()
        .all(|balance| balance.spendable_credits == 0
            && balance.last_scanned_index.is_none()
            && balance.source == ShieldedBalanceSource::NoHistory));

    let snapshot = ShieldedSyncStartState {
        per_subwallet: BTreeMap::from([
            (
                SubwalletId::new(wallet, 0),
                ShieldedSubwalletStartState {
                    has_sync_state: true,
                    last_synced_index: 0,
                    ..Default::default()
                },
            ),
            (
                SubwalletId::new(wallet, 1),
                ShieldedSubwalletStartState {
                    notes: vec![note(1, 70, true)],
                    ..Default::default()
                },
            ),
        ]),
        ..Default::default()
    };
    coordinator
        .restore_for_wallet(wallet, &snapshot)
        .await
        .unwrap();
    let restored = ready(&coordinator, wallet).await;
    assert_eq!(restored.accounts[&0].spendable_credits, 0);
    assert_eq!(restored.accounts[&0].last_scanned_index, Some(0));
    assert_eq!(
        restored.accounts[&0].source,
        ShieldedBalanceSource::Restored
    );
    assert_eq!(restored.accounts[&1].spendable_credits, 0);
    assert_eq!(
        restored.accounts[&1].last_scanned_index, None,
        "legacy notes do not invent scan coverage"
    );
    assert_eq!(
        restored.accounts[&1].source,
        ShieldedBalanceSource::Restored
    );
    assert_eq!(
        restored.accounts[&2].source,
        ShieldedBalanceSource::NoHistory
    );

    coordinator
        .store()
        .write()
        .await
        .set_last_synced_note_index(SubwalletId::new(wallet, 2), 0)
        .unwrap();
    let scanned = ready(&coordinator, wallet).await;
    assert_eq!(scanned.accounts[&2].last_scanned_index, Some(0));
    assert_eq!(
        scanned.accounts[&2].source,
        ShieldedBalanceSource::ScannedThisSession
    );
    coordinator.clear().await.unwrap();
    assert_eq!(
        coordinator.local_balance_snapshot(wallet).await.unwrap(),
        ShieldedLocalBalanceState::Unbound
    );
}

#[tokio::test]
async fn local_balance_successful_empty_scan_marks_known_zero() {
    let mut sdk = dash_sdk::Sdk::new_mock();
    let count = 2048
        * u32::from(
            sdk.version()
                .drive_abci
                .query
                .shielded_queries
                .max_query_chunks,
        );
    // The stream seeds a 16-request window before the empty first chunk
    // establishes the pool size. All speculative chunks see the same empty pool.
    for chunk in 0..16 {
        sdk.mock()
            .expect_fetch(
                ShieldedEncryptedNotesQuery {
                    start_index: u64::from(count) * chunk,
                    count,
                },
                Some(ShieldedEncryptedNotes {
                    notes: Vec::new(),
                    total_count: 0,
                }),
            )
            .await
            .expect("empty mock network response");
    }
    let path = tree_path();
    let coordinator = NetworkShieldedCoordinator::new(
        Arc::new(sdk),
        dashcore::Network::Testnet,
        path.clone(),
        FileBackedShieldedStore::open_path(path, 100).unwrap(),
    );
    let wallet = [6; 32];
    register(&coordinator, wallet, &[0]).await;
    coordinator.mark_hydrated(wallet, true).await;
    assert_eq!(
        ready(&coordinator, wallet).await.accounts[&0].source,
        ShieldedBalanceSource::NoHistory
    );
    let outcome = coordinator.sync(true).await;
    assert_eq!(
        outcome.success_count(),
        1,
        "empty scan should succeed: {outcome:?}"
    );
    let account = ready(&coordinator, wallet).await.accounts[&0];
    assert_eq!(account.spendable_credits, 0);
    assert_eq!(account.last_scanned_index, Some(0));
    assert_eq!(account.source, ShieldedBalanceSource::ScannedThisSession);
}

#[tokio::test]
async fn local_balance_restores_funds_and_durable_pending_reservations_offline() {
    let path = tree_path();
    let wallet = [2; 32];
    let id = SubwalletId::new(wallet, 0);
    let other_wallet = [3; 32];
    // Only the redrive reservation belongs to the tree DB. Decrypted notes
    // below are the host persister's snapshot, as on a real cold bind.
    {
        let mut store = FileBackedShieldedStore::open_path(&path, 100).unwrap();
        store
            .arm_redrive(
                id,
                super::store::PendingRedrive {
                    activity_id: [9; 32],
                    anchor: [8; 32],
                    nullifiers: vec![[2; 32]],
                    st_bytes: vec![7; 32],
                    attempts: 0,
                },
            )
            .unwrap();
    }
    let coordinator = coordinator(&path);
    register(&coordinator, wallet, &[0, 1]).await;
    register(&coordinator, other_wallet, &[0]).await;
    let start = ShieldedSyncStartState {
        per_subwallet: BTreeMap::from([
            (
                id,
                ShieldedSubwalletStartState {
                    notes: vec![note(1, 100, false), note(2, 200, false), note(3, 300, true)],
                    has_sync_state: true,
                    last_synced_index: 50,
                    ..Default::default()
                },
            ),
            (
                SubwalletId::new(wallet, 1),
                ShieldedSubwalletStartState {
                    notes: vec![note(4, 40, false)],
                    ..Default::default()
                },
            ),
            (
                SubwalletId::new(other_wallet, 0),
                ShieldedSubwalletStartState {
                    notes: vec![note(5, 500, false)],
                    ..Default::default()
                },
            ),
        ]),
        ..Default::default()
    };
    coordinator
        .restore_for_wallet(wallet, &start)
        .await
        .unwrap();
    coordinator.mark_hydrated(wallet, true).await;
    let restored = ready(&coordinator, wallet).await;
    assert_eq!(
        restored.accounts[&0].spendable_credits, 100,
        "reserved and spent notes are excluded after reopen"
    );
    assert_eq!(restored.accounts[&0].last_scanned_index, Some(50));
    assert_eq!(
        restored.accounts[&0].source,
        ShieldedBalanceSource::Restored
    );
    assert_eq!(restored.accounts[&1].spendable_credits, 40);
    assert_eq!(restored.accounts[&1].last_scanned_index, None);
    let store = coordinator.store().read().await;
    assert_eq!(
        restored.accounts[&0].spendable_credits,
        store.spendable_balance(id).unwrap()
    );
    assert!(
        store
            .get_all_notes(SubwalletId::new(other_wallet, 0))
            .unwrap()
            .is_empty(),
        "another wallet is not restored by this bind"
    );
    drop(store);
    coordinator.unregister_wallet(wallet).await;
    assert_eq!(
        coordinator.local_balance_snapshot(wallet).await.unwrap(),
        ShieldedLocalBalanceState::Unbound
    );
}

#[tokio::test]
async fn local_balance_waits_for_atomic_store_and_lifecycle_updates() {
    let path = tree_path();
    let coordinator = coordinator(&path);
    let wallet = [4; 32];
    let id = SubwalletId::new(wallet, 0);
    register(&coordinator, wallet, &[0]).await;
    coordinator.mark_hydrated(wallet, true).await;
    let mut store = coordinator.store().write().await;
    store.save_note(id, &note(1, 100, false)).unwrap();
    let mut read = Box::pin(coordinator.local_balance_snapshot(wallet));
    assert!(matches!(futures::poll!(&mut read), Poll::Pending));
    store.mark_pending(id, &[1; 32]).unwrap();
    store.set_last_synced_note_index(id, 42).unwrap();
    drop(store);
    let ShieldedLocalBalanceState::Ready(snapshot) = read.await.unwrap() else {
        panic!("ready");
    };
    assert_eq!(snapshot.accounts[&0].spendable_credits, 0);
    assert_eq!(snapshot.accounts[&0].last_scanned_index, Some(42));
    assert_eq!(
        snapshot.accounts[&0].source,
        ShieldedBalanceSource::ScannedThisSession
    );

    let install = coordinator.begin_install(wallet).await;
    let mut read = Box::pin(coordinator.local_balance_snapshot(wallet));
    assert!(matches!(futures::poll!(&mut read), Poll::Pending));
    install.mark_hydrated(false).await;
    drop(install);
    assert_eq!(
        read.await.unwrap(),
        ShieldedLocalBalanceState::RestoreIncomplete
    );
}
