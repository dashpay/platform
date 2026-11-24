//! A sync pass against a wallet that is removed, re-added, re-keyed or
//! cleared around it.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use drive_proof_verifier::types::{
    ShieldedEncryptedNote, ShieldedEncryptedNotes, ShieldedEncryptedNotesQuery,
};
use tokio_util::sync::CancellationToken;

use super::balance_tests::tree_path;
use super::sync::pass_pause::{self, Point};
use super::{
    FileBackedShieldedStore, NetworkShieldedCoordinator, OrchardKeySet, ShieldedNote,
    ShieldedStore, SubwalletId,
};
use crate::changeset::{
    ClientStartState, PersistenceError, PlatformWalletChangeSet, PlatformWalletPersistence,
    ShieldedChangeSet, ShieldedSubwalletStartState, ShieldedSyncStartState,
};
use crate::manager::shielded_sync::ShieldedSyncPassSummary;
use crate::wallet::persister::WalletPersister;

/// The chain's notes in this test, by position. None decrypts for any
/// wallet; each carries the nullifier its action spent.
const POOL_SIZE: u64 = 2100;
/// Where the restored note's spend landed.
const SPENT_AT: u64 = 1500;
/// The older snapshot's watermark: before the spend.
const SNAPSHOT_WATERMARK: u64 = 1000;

const WALLET: [u8; 32] = [0x5A; 32];
/// A note whose spend landed at [`SPENT_AT`].
const SPENT_NULLIFIER: [u8; 32] = [0x4E; 32];
/// A note no action spends.
const UNSPENT_NULLIFIER: [u8; 32] = [0x4F; 32];

fn pool_nullifier(position: u64) -> [u8; 32] {
    if position == SPENT_AT {
        return SPENT_NULLIFIER;
    }
    let mut nullifier = [0xC0; 32];
    nullifier[..8].copy_from_slice(&position.to_le_bytes());
    nullifier
}

fn pool_note(position: u64) -> ShieldedEncryptedNote {
    let mut cmx = [0u8; 32];
    cmx[..8].copy_from_slice(&(position + 1).to_le_bytes());
    ShieldedEncryptedNote {
        cmx: cmx.to_vec(),
        nullifier: pool_nullifier(position).to_vec(),
        cv_net: vec![0; 32],
        encrypted_note: Vec::new(),
    }
}

/// A mock network serving the pool to a pass starting at any chunk.
async fn pool_sdk() -> dash_sdk::Sdk {
    let mut sdk = dash_sdk::Sdk::new_mock();
    let count = 2048
        * u32::from(
            sdk.version()
                .drive_abci
                .query
                .shielded_queries
                .max_query_chunks,
        );
    let notes: Vec<_> = (0..POOL_SIZE).map(pool_note).collect();
    let mut starts = BTreeSet::new();
    for first in (0..POOL_SIZE).step_by(2048) {
        for window in 0..20 {
            starts.insert(first + u64::from(count) * window);
        }
    }
    for start in starts {
        let from = start.min(POOL_SIZE) as usize;
        let to = (start + u64::from(count)).min(POOL_SIZE) as usize;
        sdk.mock()
            .expect_fetch(
                ShieldedEncryptedNotesQuery {
                    start_index: start,
                    count,
                },
                Some(ShieldedEncryptedNotes {
                    notes: notes[from..to].to_vec(),
                    total_count: POOL_SIZE,
                }),
            )
            .await
            .expect("mock pool response");
    }
    sdk
}

fn note(nullifier: [u8; 32]) -> ShieldedNote {
    ShieldedNote {
        position: u64::from(nullifier[0]),
        cmx: nullifier,
        nullifier,
        block_height: 1,
        is_spent: false,
        value: 1_000,
        note_data: vec![0u8; 115],
    }
}

/// The host's snapshot of the spent and the unspent note, scanned up to
/// [`SNAPSHOT_WATERMARK`], before the spend.
fn older_snapshot(id: SubwalletId) -> ShieldedSyncStartState {
    ShieldedSyncStartState {
        per_subwallet: BTreeMap::from([(
            id,
            ShieldedSubwalletStartState {
                notes: vec![note(SPENT_NULLIFIER), note(UNSPENT_NULLIFIER)],
                has_sync_state: true,
                last_synced_index: SNAPSHOT_WATERMARK,
                ..Default::default()
            },
        )]),
        ..Default::default()
    }
}

fn coordinator_on(sdk: dash_sdk::Sdk, path: &std::path::Path) -> Arc<NetworkShieldedCoordinator> {
    Arc::new(NetworkShieldedCoordinator::new(
        Arc::new(sdk),
        dashcore::Network::Testnet,
        path.to_path_buf(),
        FileBackedShieldedStore::open_path(path, 100).unwrap(),
    ))
}

fn remove_store_files(path: &std::path::Path) {
    for suffix in ["", "-wal", "-shm", ".lock"] {
        let mut file = path.as_os_str().to_owned();
        file.push(suffix);
        let _ = std::fs::remove_file(file);
    }
}

async fn sync(coordinator: &NetworkShieldedCoordinator) -> ShieldedSyncPassSummary {
    coordinator.sync(true, &CancellationToken::new()).await
}

/// The host's persister for the wallet: commits each `store` inline.
#[derive(Default)]
struct Host {
    saved: Mutex<Vec<ShieldedChangeSet>>,
}

impl Host {
    fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Whether any changeset the host took touches `id`.
    fn took_any_for(&self, id: SubwalletId) -> bool {
        self.saved.lock().unwrap().iter().any(|changeset| {
            changeset.notes_saved.contains_key(&id)
                || changeset.nullifiers_spent.contains_key(&id)
                || changeset.synced_indices.contains_key(&id)
        })
    }

    /// Whether the host took `nullifier` as spent for `id`.
    fn took_spent(&self, id: SubwalletId, nullifier: [u8; 32]) -> bool {
        self.saved.lock().unwrap().iter().any(|changeset| {
            changeset
                .nullifiers_spent
                .get(&id)
                .is_some_and(|spent| spent.contains(&nullifier))
        })
    }
}

impl PlatformWalletPersistence for Host {
    fn store_commits_inline(&self) -> bool {
        true
    }

    fn store(
        &self,
        _wallet_id: [u8; 32],
        changeset: PlatformWalletChangeSet,
    ) -> Result<(), PersistenceError> {
        if let Some(shielded) = changeset.shielded {
            self.saved.lock().unwrap().push(shielded);
        }
        Ok(())
    }

    fn flush(&self, _wallet_id: [u8; 32]) -> Result<(), PersistenceError> {
        Ok(())
    }

    fn load(&self) -> Result<ClientStartState, PersistenceError> {
        Ok(ClientStartState::default())
    }
}

/// Register [`WALLET`]'s account 0 with `host` as its persister, under the
/// keys of `seed`.
async fn register_keys(
    coordinator: &NetworkShieldedCoordinator,
    host: &Arc<Host>,
    seed: &[u8; 64],
) {
    let views = BTreeMap::from([(
        0,
        OrchardKeySet::from_seed(seed, dashcore::Network::Testnet, 0)
            .expect("test viewing key")
            .viewing_keys(),
    )]);
    coordinator
        .register_wallet(WALLET, views, WalletPersister::new(WALLET, host.clone()))
        .await
        .expect("register test wallet");
}

async fn register_with(coordinator: &NetworkShieldedCoordinator, host: &Arc<Host>) {
    register_keys(coordinator, host, &[42; 64]).await;
}

/// Add the wallet again with `host` and restore the older snapshot.
async fn re_add(coordinator: &NetworkShieldedCoordinator, host: &Arc<Host>, id: SubwalletId) {
    register_with(coordinator, host).await;
    coordinator
        .restore_for_wallet(WALLET, &older_snapshot(id))
        .await
        .unwrap();
    coordinator.mark_hydrated(WALLET, true).await;
}

async fn watermark(coordinator: &NetworkShieldedCoordinator, id: SubwalletId) -> u64 {
    coordinator
        .store()
        .read()
        .await
        .last_synced_note_index(id)
        .unwrap()
}

async fn unspent(coordinator: &NetworkShieldedCoordinator, id: SubwalletId) -> BTreeSet<[u8; 32]> {
    coordinator
        .store()
        .read()
        .await
        .get_unspent_notes(id)
        .unwrap()
        .into_iter()
        .map(|note| note.nullifier)
        .collect()
}

async fn wait_for(pause: &pass_pause::Pause) {
    tokio::time::timeout(Duration::from_secs(60), pause.reached.acquire())
        .await
        .expect("the pass reaches its pause point")
        .unwrap()
        .forget();
}

/// The wallet's next pass, from the restored snapshot, scans the spend that
/// landed above the snapshot's watermark and hands it to `host`.
async fn assert_next_pass_scans_the_spend(
    coordinator: &NetworkShieldedCoordinator,
    host: &Arc<Host>,
    id: SubwalletId,
) {
    let outcome = sync(coordinator).await;
    assert_eq!(outcome.success_count(), 1, "{outcome:?}");
    assert_eq!(
        unspent(coordinator, id).await,
        BTreeSet::from([UNSPENT_NULLIFIER]),
        "the spend above the snapshot's watermark is scanned"
    );
    assert!(host.took_spent(id, SPENT_NULLIFIER));
    assert_eq!(watermark(coordinator, id).await, POOL_SIZE);
}

/// A wallet removed after a pass took its subwallet list but before the pass
/// took the store guard, and optionally added again under the same id and
/// keys before the pass resumes: the pass writes no watermark and saves no
/// note for it, and hands its host nothing. Restored from an older snapshot,
/// the wallet's next pass scans from that snapshot's watermark.
async fn removal_before_the_pass_guard(re_add_during_the_pause: bool) {
    let path = tree_path();
    let coordinator = coordinator_on(pool_sdk().await, &path);
    let id = SubwalletId::new(WALLET, 0);
    register_with(&coordinator, &Host::new()).await;
    coordinator.mark_hydrated(WALLET, true).await;

    let pause = pass_pause::arm_at(
        pass_pause::key(coordinator.store()),
        Point::AfterListSnapshot,
    );
    let pass = tokio::spawn({
        let coordinator = coordinator.clone();
        async move { sync(&coordinator).await }
    });
    wait_for(&pause).await;
    tokio::time::timeout(
        Duration::from_secs(60),
        coordinator.unregister_wallet(WALLET),
    )
    .await
    .expect("a removal does not wait for a pass that holds no lock");
    let host = Host::new();
    if re_add_during_the_pause {
        register_with(&coordinator, &host).await;
    }
    pause.resume.add_permits(1);
    pass.await.unwrap();

    assert_eq!(
        watermark(&coordinator, id).await,
        0,
        "the pass writes no watermark for the removed wallet"
    );
    assert!(
        coordinator
            .store()
            .read()
            .await
            .get_all_notes(id)
            .unwrap()
            .is_empty(),
        "the pass saves no note for the removed wallet"
    );
    assert!(
        !host.took_any_for(id),
        "the pass hands the re-added wallet's host nothing"
    );

    if !re_add_during_the_pause {
        register_with(&coordinator, &host).await;
    }
    coordinator
        .restore_for_wallet(WALLET, &older_snapshot(id))
        .await
        .unwrap();
    coordinator.mark_hydrated(WALLET, true).await;
    assert_next_pass_scans_the_spend(&coordinator, &host, id).await;
    drop(coordinator);
    remove_store_files(&path);
}

#[tokio::test]
async fn a_removal_before_the_pass_guard_leaves_the_wallet_unscanned() {
    removal_before_the_pass_guard(false).await;
}

#[tokio::test]
async fn a_same_key_re_add_before_the_pass_guard_leaves_the_wallet_unscanned() {
    removal_before_the_pass_guard(true).await;
}

/// A wallet removed and added again under the same keys after a pass scanned
/// it, but before the pass hands its host the changeset: the re-added
/// wallet's host gets nothing from that pass, whose spend and watermark
/// belong to the removed registration, and the re-added wallet's own next
/// pass scans from its restored snapshot.
#[tokio::test]
async fn a_removal_before_the_host_save_hands_the_new_host_nothing() {
    let path = tree_path();
    let coordinator = coordinator_on(pool_sdk().await, &path);
    let id = SubwalletId::new(WALLET, 0);
    re_add(&coordinator, &Host::new(), id).await;

    let pause = pass_pause::arm_at(pass_pause::key(coordinator.store()), Point::BeforeHostSave);
    let pass = tokio::spawn({
        let coordinator = coordinator.clone();
        async move { sync(&coordinator).await }
    });
    wait_for(&pause).await;
    coordinator.unregister_wallet(WALLET).await;
    let host = Host::new();
    re_add(&coordinator, &host, id).await;
    pause.resume.add_permits(1);
    pass.await.unwrap();

    assert!(
        !host.took_any_for(id),
        "the pass hands the re-added wallet's host nothing"
    );
    assert_eq!(watermark(&coordinator, id).await, SNAPSHOT_WATERMARK);
    assert_next_pass_scans_the_spend(&coordinator, &host, id).await;
    drop(coordinator);
    remove_store_files(&path);
}

/// The wallet re-bound under new keys after a pass took its subwallet list
/// but before the pass took the store guard: the pass writes nothing under
/// the old keys, and hands the new registration's host nothing.
#[tokio::test]
async fn a_re_key_before_the_pass_guard_writes_nothing_for_the_old_key() {
    let path = tree_path();
    let coordinator = coordinator_on(pool_sdk().await, &path);
    let id = SubwalletId::new(WALLET, 0);
    register_with(&coordinator, &Host::new()).await;
    coordinator.mark_hydrated(WALLET, true).await;

    let pause = pass_pause::arm_at(
        pass_pause::key(coordinator.store()),
        Point::AfterListSnapshot,
    );
    let pass = tokio::spawn({
        let coordinator = coordinator.clone();
        async move { sync(&coordinator).await }
    });
    wait_for(&pause).await;
    let host = Host::new();
    register_keys(&coordinator, &host, &[43; 64]).await;
    pause.resume.add_permits(1);
    pass.await.unwrap();

    assert_eq!(
        watermark(&coordinator, id).await,
        0,
        "the pass writes no watermark under the old key"
    );
    assert!(
        !host.took_any_for(id),
        "the pass hands the new registration's host nothing"
    );
    drop(coordinator);
    remove_store_files(&path);
}

/// A Clear after a pass took its subwallet list but before the pass took the
/// store guard: the pass writes nothing into the reset store, so the wallet
/// bound again after it still starts from index 0. A restore never rewinds a
/// watermark, and the pass's changeset found no persister, so a watermark it
/// wrote would leave the host without the notes below it.
#[tokio::test]
async fn a_clear_before_the_pass_guard_leaves_the_store_reset() {
    let path = tree_path();
    let coordinator = coordinator_on(pool_sdk().await, &path);
    let id = SubwalletId::new(WALLET, 0);
    register_with(&coordinator, &Host::new()).await;
    coordinator.mark_hydrated(WALLET, true).await;

    let pause = pass_pause::arm_at(
        pass_pause::key(coordinator.store()),
        Point::AfterListSnapshot,
    );
    let pass = tokio::spawn({
        let coordinator = coordinator.clone();
        async move { sync(&coordinator).await }
    });
    wait_for(&pause).await;
    tokio::time::timeout(Duration::from_secs(60), coordinator.clear())
        .await
        .expect("a Clear does not wait for a pass that holds no lock")
        .unwrap();
    pause.resume.add_permits(1);
    pass.await.unwrap();

    register_with(&coordinator, &Host::new()).await;
    coordinator
        .restore_for_wallet(WALLET, &ShieldedSyncStartState::default())
        .await
        .unwrap();
    assert_eq!(
        watermark(&coordinator, id).await,
        0,
        "the wallet bound again after the Clear starts from index 0"
    );
    assert_eq!(
        coordinator.store().read().await.tree_size().unwrap(),
        0,
        "the tree stays reset"
    );
    drop(coordinator);
    remove_store_files(&path);
}

/// A Clear whose reset fails while a pass that already scanned waits to
/// hand its host the changeset: nothing was cleared, so the pass goes on as
/// if there had been no Clear. Its host takes the changeset, and the wallet
/// stays registered.
#[tokio::test]
async fn a_failed_clear_leaves_a_scanned_pass_to_save() {
    let path = tree_path();
    let coordinator = coordinator_on(pool_sdk().await, &path);
    let id = SubwalletId::new(WALLET, 0);
    let host = Host::new();
    re_add(&coordinator, &host, id).await;

    let pause = pass_pause::arm_at(pass_pause::key(coordinator.store()), Point::BeforeHostSave);
    let pass = tokio::spawn({
        let coordinator = coordinator.clone();
        async move { sync(&coordinator).await }
    });
    wait_for(&pause).await;
    // The subwallet purge fails on a missing table, so the Clear fails.
    let conn = rusqlite::Connection::open(&path).unwrap();
    conn.execute_batch("ALTER TABLE shielded_pending_spends RENAME TO unavailable_pending_spends;")
        .unwrap();
    assert!(coordinator.clear().await.is_err());
    conn.execute_batch("ALTER TABLE unavailable_pending_spends RENAME TO shielded_pending_spends;")
        .unwrap();
    drop(conn);
    pause.resume.add_permits(1);
    let outcome = pass.await.unwrap();
    assert_eq!(outcome.success_count(), 1, "{outcome:?}");

    assert!(
        host.took_spent(id, SPENT_NULLIFIER),
        "the host takes the pass's changeset"
    );
    assert_eq!(
        coordinator.registered_subwallets().await,
        vec![id],
        "a failed Clear keeps the registration"
    );
    drop(coordinator);
    remove_store_files(&path);
}

/// A pass that starts after a Clear reset the store but before it cleared
/// the registries: it lists the wallets only after the Clear, so it leaves
/// no watermark or commitment in the reset store.
#[tokio::test]
async fn a_pass_listing_during_a_clear_writes_nothing_into_the_reset_store() {
    let path = tree_path();
    let coordinator = coordinator_on(pool_sdk().await, &path);
    let id = SubwalletId::new(WALLET, 0);
    register_with(&coordinator, &Host::new()).await;
    coordinator.mark_hydrated(WALLET, true).await;

    let key = pass_pause::key(coordinator.store());
    let reset = pass_pause::arm_at(key, Point::AfterClearReset);
    let clear = tokio::spawn({
        let coordinator = coordinator.clone();
        async move { coordinator.clear().await }
    });
    wait_for(&reset).await;
    let listed = pass_pause::arm_at(key, Point::AfterListSnapshot);
    let cancel = CancellationToken::new();
    let mut pass = Box::pin(coordinator.sync(true, &cancel));
    // One poll takes the pass as far as it can go now: to its list, unless
    // the Clear still holds `accounts`.
    let polled = std::future::Future::poll(
        pass.as_mut(),
        &mut std::task::Context::from_waker(std::task::Waker::noop()),
    );
    assert!(polled.is_pending());
    let listed_during_the_clear = listed.reached.try_acquire().is_ok();
    reset.resume.add_permits(1);
    clear.await.unwrap().unwrap();
    listed.resume.add_permits(1);
    pass.await;

    assert!(
        !listed_during_the_clear,
        "no pass lists the wallets while the Clear holds them"
    );
    assert_eq!(
        watermark(&coordinator, id).await,
        0,
        "no pass leaves a watermark in the reset store"
    );
    assert_eq!(
        coordinator.store().read().await.tree_size().unwrap(),
        0,
        "the tree stays reset"
    );
    drop(coordinator);
    remove_store_files(&path);
}
