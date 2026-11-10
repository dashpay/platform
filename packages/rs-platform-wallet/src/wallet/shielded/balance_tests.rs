use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::task::Poll;

use super::balance::{
    ShieldedBalanceSource, ShieldedLocalAccountBalance, ShieldedLocalBalanceSnapshot,
    ShieldedLocalBalanceState, ShieldedOrdinaryBalance,
};
use super::{
    shielded_identity_account_index, FileBackedShieldedStore, NetworkShieldedCoordinator,
    OrchardKeySet, ShieldedNote, ShieldedStore, SubwalletId,
};
use crate::changeset::{ShieldedSubwalletStartState, ShieldedSyncStartState};
use crate::error::PlatformWalletError;
use crate::wallet::persister::{NoPlatformPersistence, WalletPersister};
use drive_proof_verifier::types::{ShieldedEncryptedNotes, ShieldedEncryptedNotesQuery};
use tokio_util::sync::CancellationToken;

fn tree_path() -> PathBuf {
    // Clock resolution alone does not distinguish parallel test invocations.
    static NEXT_PATH: AtomicU64 = AtomicU64::new(0);
    let sequence = NEXT_PATH.fetch_add(1, Ordering::Relaxed);
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "shielded_local_balance_{}_{nonce}_{sequence}.sqlite",
        std::process::id()
    ))
}

/// Run a Clear on `coordinator`, whose store is the file at `path`, that
/// resets the commitment tree and then fails in the purge: a real SQLite
/// schema error (the pending-spends table renamed away) for the length of
/// the Clear. Requires that the purge is the step that failed.
pub(super) async fn clear_failing_at_the_purge(
    coordinator: &NetworkShieldedCoordinator,
    path: &std::path::Path,
) {
    let conn = rusqlite::Connection::open(path).expect("open db");
    conn.execute_batch("ALTER TABLE shielded_pending_spends RENAME TO unavailable_pending_spends;")
        .expect("break schema");
    let cleared = coordinator.clear().await;
    conn.execute_batch("ALTER TABLE unavailable_pending_spends RENAME TO shielded_pending_spends;")
        .expect("restore schema");
    assert!(
        matches!(
            cleared,
            Err(PlatformWalletError::ShieldedStoreError(ref message))
                if message.contains("purge_all_subwallets failed")
        ),
        "the Clear must fail in the purge: {cleared:?}"
    );
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
        .await
        .expect("register test wallet");
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

async fn empty_pool_sdk() -> dash_sdk::Sdk {
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
    sdk
}

#[tokio::test]
async fn local_balance_successful_empty_scan_marks_known_zero() {
    let sdk = empty_pool_sdk().await;
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
    let outcome = coordinator.sync(true, &CancellationToken::new()).await;
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
async fn local_balance_successful_scan_covers_unhydrated_registration() {
    let path = tree_path();
    let coordinator = NetworkShieldedCoordinator::new(
        Arc::new(empty_pool_sdk().await),
        dashcore::Network::Testnet,
        path.clone(),
        FileBackedShieldedStore::open_path(path, 100).unwrap(),
    );
    let wallet = [7; 32];
    register(&coordinator, wallet, &[0]).await;
    assert_eq!(
        coordinator.local_balance_snapshot(wallet).await.unwrap(),
        ShieldedLocalBalanceState::RestoreIncomplete
    );
    let outcome = coordinator.sync(true, &CancellationToken::new()).await;
    assert_eq!(outcome.success_count(), 1, "{outcome:?}");
    let account = ready(&coordinator, wallet).await.accounts[&0];
    assert_eq!(account.spendable_credits, 0);
    assert_eq!(account.source, ShieldedBalanceSource::ScannedThisSession);
    assert_eq!(account.last_scanned_index, Some(0));
    assert!(
        !coordinator.is_hydrated(wallet).await,
        "scan coverage must not fast-path a later host snapshot restore"
    );
}

#[tokio::test]
async fn local_balance_recovers_after_failed_clear_and_completed_scan() {
    let path = tree_path();
    let coordinator = NetworkShieldedCoordinator::new(
        Arc::new(empty_pool_sdk().await),
        dashcore::Network::Testnet,
        path.clone(),
        FileBackedShieldedStore::open_path(&path, 100).unwrap(),
    );
    let wallet = [8; 32];
    register(&coordinator, wallet, &[0]).await;
    coordinator.mark_hydrated(wallet, true).await;
    let id = SubwalletId::new(wallet, 0);
    coordinator
        .store()
        .write()
        .await
        .save_note(id, &note(1, 900, false))
        .unwrap();
    assert_eq!(
        ready(&coordinator, wallet).await.accounts[&0].spendable_credits,
        900
    );

    // FileBacked's SQL-first ordering keeps the live subwallet ledger intact
    // when its purge fails; the host keeps its rows.
    clear_failing_at_the_purge(&coordinator, &path).await;
    assert!(!coordinator.is_hydrated(wallet).await);
    assert_eq!(coordinator.registered_subwallets().await, vec![id]);
    assert_eq!(
        coordinator.local_balance_snapshot(wallet).await.unwrap(),
        ShieldedLocalBalanceState::RestoreIncomplete
    );
    let outcome = coordinator.sync(true, &CancellationToken::new()).await;
    assert_eq!(outcome.success_count(), 1, "{outcome:?}");
    let account = ready(&coordinator, wallet).await.accounts[&0];
    assert_eq!(account.spendable_credits, 900);
    assert_eq!(account.source, ShieldedBalanceSource::ScannedThisSession);
    assert!(!coordinator.is_hydrated(wallet).await);
}

/// A Clear resets the commitment tree before it purges the ledger. When the
/// reset lands and the purge fails, the notes stay, and so did the evidence
/// of a scan completed in this session: with it, an account read as known
/// with its pre-Clear amount, against a tree that Clear had emptied. A failed
/// Clear forgets that evidence, so the wallet reads as incomplete until a
/// scan completes again. The notes are kept.
#[tokio::test]
async fn should_forget_scan_evidence_when_a_clear_resets_the_tree_but_fails_to_purge() {
    let path = tree_path();
    let coordinator = coordinator(&path);
    let wallet = [12; 32];
    register(&coordinator, wallet, &[0]).await;
    let id = SubwalletId::new(wallet, 0);
    {
        let mut store = coordinator.store().write().await;
        store.save_note(id, &note(1, 900, false)).unwrap();
        store.append_commitment(&[7; 32], true).unwrap();
        // A scan completed in this session: the account's ledger is
        // established with no host hydration behind it.
        store.set_last_synced_note_index(id, 1).unwrap();
    }
    assert!(!coordinator.is_hydrated(wallet).await);
    let scanned = ready(&coordinator, wallet).await.accounts[&0];
    assert_eq!(scanned.spendable_credits, 900);
    assert_eq!(scanned.source, ShieldedBalanceSource::ScannedThisSession);

    clear_failing_at_the_purge(&coordinator, &path).await;

    assert_eq!(
        coordinator.local_balance_snapshot(wallet).await.unwrap(),
        ShieldedLocalBalanceState::RestoreIncomplete,
        "evidence of a scan of the tree as it was must not outlive the reset"
    );
    {
        let store = coordinator.store().read().await;
        assert_eq!(store.tree_size().unwrap(), 0, "the tree reset landed");
        assert_eq!(
            store.get_all_notes(id).unwrap().len(),
            1,
            "the failed purge kept the notes"
        );
        let account = store.local_account_balance(id).unwrap();
        assert_eq!(account.source, ShieldedBalanceSource::NoHistory);
        assert_eq!(account.last_scanned_index, None);
        assert!(!account.is_established());
        assert_eq!(account.spendable_credits, 900);
    }
    assert_eq!(coordinator.registered_subwallets().await, vec![id]);

    // A scan that completes afterwards establishes the ledger again.
    coordinator
        .store()
        .write()
        .await
        .set_last_synced_note_index(id, 1)
        .unwrap();
    let rescanned = ready(&coordinator, wallet).await.accounts[&0];
    assert_eq!(rescanned.spendable_credits, 900);
    assert_eq!(rescanned.source, ShieldedBalanceSource::ScannedThisSession);
}

#[tokio::test]
async fn local_balance_unhydrated_requires_scan_coverage_for_every_account() {
    let path = tree_path();
    let coordinator = coordinator(&path);
    let wallet = [9; 32];
    register(&coordinator, wallet, &[0]).await;
    coordinator
        .store()
        .write()
        .await
        .set_last_synced_note_index(SubwalletId::new(wallet, 0), 0)
        .unwrap();
    assert_eq!(ready(&coordinator, wallet).await.accounts.len(), 1);
    register(&coordinator, wallet, &[0, 1]).await;
    assert_eq!(
        coordinator.local_balance_snapshot(wallet).await.unwrap(),
        ShieldedLocalBalanceState::RestoreIncomplete,
        "an added account without scan coverage keeps the wallet incomplete"
    );
    coordinator
        .store()
        .write()
        .await
        .set_last_synced_note_index(SubwalletId::new(wallet, 1), 0)
        .unwrap();
    assert_eq!(ready(&coordinator, wallet).await.accounts.len(), 2);
    coordinator.clear().await.unwrap();
    register(&coordinator, wallet, &[0, 1]).await;
    assert_eq!(
        coordinator.local_balance_snapshot(wallet).await.unwrap(),
        ShieldedLocalBalanceState::RestoreIncomplete,
        "successful Clear must not retain the previous scan evidence"
    );
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
                    identity_nonce_finalized: false,
                    identity_user_abandoned: false,
                    anchor_height: None,
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
async fn should_release_lifecycle_while_snapshot_waits_for_store_and_revalidate_registration() {
    let path = tree_path();
    let coordinator = coordinator(&path);
    let wallet = [14; 32];
    let other_wallet = [15; 32];
    register(&coordinator, wallet, &[0]).await;
    register(&coordinator, other_wallet, &[0]).await;
    coordinator.mark_hydrated(wallet, true).await;

    let mut store = coordinator.store().write().await;
    let mut read = Box::pin(coordinator.local_balance_snapshot(wallet));
    assert!(matches!(futures::poll!(&mut read), Poll::Pending));
    // The actual snapshot is now parked behind a scan's store writer. An
    // unrelated wallet's idempotent registration must keep its store-free path.
    let mut rebind = Box::pin(register(&coordinator, other_wallet, &[0]));
    assert!(matches!(futures::poll!(&mut rebind), Poll::Ready(())));

    // Registration may also change while the snapshot waits. Its eventual
    // read must re-enumerate accounts and re-check the invalidated hydration.
    let mut add_account = Box::pin(register(&coordinator, wallet, &[0, 1]));
    assert!(matches!(futures::poll!(&mut add_account), Poll::Ready(())));
    store
        .save_note(SubwalletId::new(wallet, 0), &note(1, 100, false))
        .unwrap();
    store
        .save_note(SubwalletId::new(wallet, 1), &note(2, 200, false))
        .unwrap();
    store
        .set_last_synced_note_index(SubwalletId::new(wallet, 0), 1)
        .unwrap();
    drop(store);
    assert_eq!(
        read.await.unwrap(),
        ShieldedLocalBalanceState::RestoreIncomplete
    );
    coordinator.mark_hydrated(wallet, true).await;
    let snapshot = ready(&coordinator, wallet).await;
    assert_eq!(snapshot.accounts.len(), 2);
    assert_eq!(snapshot.accounts[&0].spendable_credits, 100);
    assert_eq!(snapshot.accounts[&1].spendable_credits, 200);
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
    assert!(
        coordinator.store().try_write().is_ok(),
        "a snapshot waiting for lifecycle must not hold the store lock"
    );
    install.mark_hydrated(false).await;
    drop(install);
    assert_eq!(
        read.await.unwrap(),
        ShieldedLocalBalanceState::Ready(snapshot),
        "completed scan evidence survives invalidated host hydration"
    );
}

#[tokio::test]
async fn should_reject_restored_local_balance_overflow() {
    let path = tree_path();
    let coordinator = coordinator(&path);
    let wallet = [7; 32];
    let id = SubwalletId::new(wallet, 0);
    register(&coordinator, wallet, &[0]).await;
    let start = ShieldedSyncStartState {
        per_subwallet: BTreeMap::from([(
            id,
            ShieldedSubwalletStartState {
                notes: vec![note(1, u64::MAX, false), note(2, 1, false)],
                ..Default::default()
            },
        )]),
        ..Default::default()
    };
    coordinator
        .restore_for_wallet(wallet, &start)
        .await
        .unwrap();
    coordinator.mark_hydrated(wallet, true).await;

    let error = coordinator
        .local_balance_snapshot(wallet)
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        PlatformWalletError::ShieldedStoreError(message)
            if message == "spendable shielded balance exceeds u64"
    ));

    // Reservations and spent notes are excluded before checked aggregation.
    // The largest representable spendable balance remains valid.
    coordinator
        .store()
        .write()
        .await
        .mark_pending(id, &[2; 32])
        .unwrap();
    assert_eq!(
        ready(&coordinator, wallet).await.accounts[&0].spendable_credits,
        u64::MAX
    );
    {
        let mut store = coordinator.store().write().await;
        store.clear_pending(id, &[2; 32]).unwrap();
        store.mark_spent(id, &[2; 32]).unwrap();
    }
    assert_eq!(
        ready(&coordinator, wallet).await.accounts[&0].spendable_credits,
        u64::MAX
    );
}

fn snapshot_of(spendable: &[(u32, u64)]) -> ShieldedLocalBalanceSnapshot {
    snapshot_with_sources(
        &spendable
            .iter()
            .map(|(account, credits)| (*account, *credits, ShieldedBalanceSource::Restored))
            .collect::<Vec<_>>(),
    )
}

fn snapshot_with_sources(
    accounts: &[(u32, u64, ShieldedBalanceSource)],
) -> ShieldedLocalBalanceSnapshot {
    ShieldedLocalBalanceSnapshot {
        accounts: accounts
            .iter()
            .map(|(account, spendable_credits, source)| {
                (
                    *account,
                    ShieldedLocalAccountBalance {
                        spendable_credits: *spendable_credits,
                        source: *source,
                        ..ShieldedLocalAccountBalance::default()
                    },
                )
            })
            .collect(),
    }
}

#[test]
fn should_leave_identity_accounts_out_of_the_ordinary_balance() {
    let identity_account = shielded_identity_account_index(0).unwrap();
    let snapshot = snapshot_of(&[(0, 100), (1, 50), (identity_account, 900)]);
    assert_eq!(
        snapshot.ordinary_balance().unwrap(),
        ShieldedOrdinaryBalance {
            spendable_credits: 150,
            established: true,
        }
    );
}

#[test]
fn should_report_an_established_zero_ordinary_balance_without_an_ordinary_account() {
    let zero = ShieldedOrdinaryBalance {
        spendable_credits: 0,
        established: true,
    };
    assert_eq!(snapshot_of(&[]).ordinary_balance().unwrap(), zero);
    let identity_account = shielded_identity_account_index(7).unwrap();
    assert_eq!(
        snapshot_with_sources(&[(identity_account, 900, ShieldedBalanceSource::NoHistory)])
            .ordinary_balance()
            .unwrap(),
        zero,
        "an identity account without history does not unsettle the ordinary balance"
    );
}

#[test]
fn should_refuse_an_ordinary_balance_past_u64() {
    let identity_account = shielded_identity_account_index(0).unwrap();
    // The identity account's balance does not count toward the overflow.
    let fits = snapshot_of(&[(0, u64::MAX - 1), (identity_account, u64::MAX)]);
    assert_eq!(
        fits.ordinary_balance().unwrap().spendable_credits,
        u64::MAX - 1
    );
    let overflows = snapshot_of(&[(0, u64::MAX - 1), (1, 2), (identity_account, u64::MAX)]);
    assert!(matches!(
        overflows.ordinary_balance(),
        Err(PlatformWalletError::ShieldedStoreError(message))
            if message == "wallet-wide shielded balance exceeds u64"
    ));
}

#[test]
fn should_not_establish_the_ordinary_balance_while_an_ordinary_account_has_no_history() {
    let identity_account = shielded_identity_account_index(0).unwrap();
    let snapshot = snapshot_with_sources(&[
        (0, 100, ShieldedBalanceSource::ScannedThisSession),
        (1, 0, ShieldedBalanceSource::NoHistory),
        (
            identity_account,
            900,
            ShieldedBalanceSource::ScannedThisSession,
        ),
    ]);
    assert_eq!(
        snapshot.ordinary_balance().unwrap(),
        ShieldedOrdinaryBalance {
            spendable_credits: 100,
            established: false,
        }
    );
}

#[tokio::test]
async fn should_not_establish_a_hydrated_empty_ledger_until_an_empty_scan_completes() {
    let path = tree_path();
    let coordinator = NetworkShieldedCoordinator::new(
        Arc::new(empty_pool_sdk().await),
        dashcore::Network::Testnet,
        path.clone(),
        FileBackedShieldedStore::open_path(path, 100).unwrap(),
    );
    let wallet = [16; 32];
    register(&coordinator, wallet, &[0]).await;
    coordinator.mark_hydrated(wallet, true).await;
    assert_eq!(
        ready(&coordinator, wallet)
            .await
            .ordinary_balance()
            .unwrap(),
        ShieldedOrdinaryBalance {
            spendable_credits: 0,
            established: false,
        },
        "a hydrated ledger with no notes and no scan is not a known zero"
    );
    let outcome = coordinator.sync(true, &CancellationToken::new()).await;
    assert_eq!(outcome.success_count(), 1, "{outcome:?}");
    assert_eq!(
        ready(&coordinator, wallet)
            .await
            .ordinary_balance()
            .unwrap(),
        ShieldedOrdinaryBalance {
            spendable_credits: 0,
            established: true,
        },
        "a completed empty scan makes the zero known"
    );
}

// ── Sync passes superseded by a concurrent lifecycle change ─────────────
//
// A pass no longer holds the store lock across its download, so a
// lifecycle change can land mid-pass. These drive `coordinator.sync` over
// the mock SDK and inject the change from the tree-progress hook, which
// fires inside the pass between store-lock holds.

/// Install a tree-progress hook that runs `on_batch(call_index)` inside the
/// pass (once per streamed batch), blocking on the lifecycle future.
/// Nothing in the pass holds a coordinator or store lock while the hook
/// runs, so the future completes without contention. It is driven on the
/// runtime via `block_in_place` (the tests use the multi-thread flavor),
/// `unconstrained` (the pass's task may have spent its coop budget) and
/// under a timeout, so a regression that fires the hook while holding a
/// lock the future needs fails the test instead of hanging it. Returns the
/// call counter.
fn hook_tree_progress<F, Fut>(
    coordinator: &Arc<NetworkShieldedCoordinator>,
    on_batch: F,
) -> Arc<std::sync::atomic::AtomicUsize>
where
    F: Fn(Arc<NetworkShieldedCoordinator>, usize) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = ()> + Send,
{
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let weak = Arc::downgrade(coordinator);
    let counter = Arc::clone(&calls);
    coordinator.install_tree_progress_handler(Some(Arc::new(move |_, _| {
        let call = counter.fetch_add(1, Ordering::SeqCst);
        if let Some(coordinator) = weak.upgrade() {
            let lifecycle_change = tokio::task::unconstrained(tokio::time::timeout(
                std::time::Duration::from_secs(10),
                on_batch(coordinator, call),
            ));
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current()
                    .block_on(lifecycle_change)
                    .expect("lifecycle change blocked inside the sync pass")
            });
        }
    })));
    calls
}

/// A host snapshot carrying sync state for `wallet`'s account 0.
fn synced_start_state(wallet: [u8; 32]) -> ShieldedSyncStartState {
    ShieldedSyncStartState {
        per_subwallet: BTreeMap::from([(
            SubwalletId::new(wallet, 0),
            ShieldedSubwalletStartState {
                has_sync_state: true,
                ..Default::default()
            },
        )]),
        ..Default::default()
    }
}

/// Coordinator over the empty mock pool, plus how many batches (hook calls)
/// one uninterrupted pass over it streams — the unit the tests below count
/// scan attempts in.
async fn empty_pool_coordinator() -> (Arc<NetworkShieldedCoordinator>, usize) {
    let path = tree_path();
    let coordinator = Arc::new(NetworkShieldedCoordinator::new(
        Arc::new(empty_pool_sdk().await),
        dashcore::Network::Testnet,
        path.clone(),
        FileBackedShieldedStore::open_path(&path, 100).unwrap(),
    ));
    register(&coordinator, [0x30; 32], &[0]).await;
    let calls = hook_tree_progress(&coordinator, |_, _| async {});
    assert_eq!(
        coordinator
            .sync(true, &CancellationToken::new())
            .await
            .success_count(),
        1
    );
    let batches_per_pass = calls.load(Ordering::SeqCst);
    assert!(batches_per_pass > 0);
    coordinator.unregister_wallet([0x30; 32]).await;
    (coordinator, batches_per_pass)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_retry_sync_superseded_by_wallet_removal_without_resurrecting_it() {
    let (coordinator, batches_per_pass) = empty_pool_coordinator().await;
    let kept = [0x31; 32];
    let removed = [0x32; 32];
    register(&coordinator, kept, &[0]).await;
    register(&coordinator, removed, &[0]).await;
    let calls = hook_tree_progress(&coordinator, move |coordinator, call| async move {
        if call == 0 {
            coordinator.unregister_wallet(removed).await;
        }
    });

    let outcome = coordinator.sync(true, &CancellationToken::new()).await;
    assert_eq!(outcome.success_count(), 1, "{outcome:?}");
    assert!(outcome.wallet_results.contains_key(&kept));
    assert!(!outcome.wallet_results.contains_key(&removed));
    assert_eq!(
        calls.load(Ordering::SeqCst),
        2 * batches_per_pass,
        "the superseded scan was retried once"
    );
    let store = coordinator.store().read().await;
    assert_eq!(
        store
            .local_account_balance(SubwalletId::new(kept, 0))
            .unwrap()
            .source,
        ShieldedBalanceSource::ScannedThisSession
    );
    assert_eq!(
        store
            .local_account_balance(SubwalletId::new(removed, 0))
            .unwrap()
            .source,
        ShieldedBalanceSource::NoHistory,
        "the superseded pass must not write sync state for the removed wallet"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_fail_sync_after_repeated_supersession() {
    let (coordinator, batches_per_pass) = empty_pool_coordinator().await;
    let wallet = [0x33; 32];
    register(&coordinator, wallet, &[0]).await;
    // Every attempt is superseded by a restore of the wallet it scans.
    let calls = hook_tree_progress(&coordinator, move |coordinator, _| async move {
        let snapshot = synced_start_state(wallet);
        coordinator
            .restore_for_wallet(wallet, &snapshot)
            .await
            .unwrap();
    });

    let outcome = coordinator.sync(true, &CancellationToken::new()).await;
    assert_eq!(outcome.success_count(), 0, "{outcome:?}");
    assert!(
        matches!(&outcome.wallet_results[&wallet],
            crate::manager::shielded_sync::WalletShieldedOutcome::Err(message)
                if message.contains("superseded")),
        "{outcome:?}"
    );
    assert_eq!(
        calls.load(Ordering::SeqCst),
        3 * batches_per_pass,
        "one attempt plus two retries"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_not_supersede_sync_for_a_wallet_bound_mid_pass() {
    let (coordinator, batches_per_pass) = empty_pool_coordinator().await;
    let scanning = [0x34; 32];
    let late = [0x35; 32];
    register(&coordinator, scanning, &[0]).await;
    // A second wallet binds (register + restore) while the pass runs.
    let calls = hook_tree_progress(&coordinator, move |coordinator, call| async move {
        if call == 0 {
            register(&coordinator, late, &[0]).await;
            let snapshot = synced_start_state(late);
            coordinator
                .restore_for_wallet(late, &snapshot)
                .await
                .unwrap();
        }
    });

    let outcome = coordinator.sync(true, &CancellationToken::new()).await;
    assert_eq!(outcome.success_count(), 1, "{outcome:?}");
    assert!(outcome.wallet_results.contains_key(&scanning));
    assert_eq!(
        calls.load(Ordering::SeqCst),
        batches_per_pass,
        "binding another wallet must not discard the in-flight scan"
    );
}
