//! The balance emitter pushes a wallet's local balance state whenever it
//! changes, and only then, whatever changed it.
//!
//! Drives a real coordinator over a SQLite store with no network: binds,
//! hydration, scans against a mock pool, a failing scan, note reservations
//! (including the release a dropped spend leaves to a spawned task), Clear
//! and unregister. The last test runs the emitter the manager installs and
//! checks that nothing is delivered once `shutdown` returned.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use super::balance::{ShieldedBalanceUpdate, ShieldedLocalBalanceState};
use super::balance_emitter::spawn_balance_emitter;
use super::cancelled_scan_tests::{chunked_pool, coordinator_over, note_to, WALLET};
use super::operations::mark_notes_pending;
use super::{
    FileBackedShieldedStore, NetworkShieldedCoordinator, OrchardKeySet, ShieldedNote,
    ShieldedStore, SubwalletId,
};
use crate::events::{EventHandler, PlatformEventHandler};
use crate::manager::{PlatformWalletManager, WalletWorker};
use crate::wallet::persister::{NoPlatformPersistence, WalletPersister};
use crate::wallet::platform_wallet::WalletId;
use dash_async::WorkerStatus;

/// How long a test waits for an update it expects.
const DELIVERY: Duration = Duration::from_secs(10);
/// How long a test waits to be satisfied that no update comes.
const QUIET: Duration = Duration::from_millis(300);

fn tree_path() -> PathBuf {
    static NEXT_PATH: AtomicU64 = AtomicU64::new(0);
    let sequence = NEXT_PATH.fetch_add(1, Ordering::Relaxed);
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "shielded_balance_emitter_{}_{nonce}_{sequence}.sqlite",
        std::process::id()
    ))
}

fn coordinator_with(sdk: dash_sdk::Sdk, path: &std::path::Path) -> Arc<NetworkShieldedCoordinator> {
    Arc::new(NetworkShieldedCoordinator::new(
        Arc::new(sdk),
        dashcore::Network::Testnet,
        path.to_path_buf(),
        FileBackedShieldedStore::open_path(path, 100).expect("open test store"),
    ))
}

fn coordinator(path: &std::path::Path) -> Arc<NetworkShieldedCoordinator> {
    coordinator_with(dash_sdk::Sdk::new_mock(), path)
}

async fn register(coordinator: &NetworkShieldedCoordinator, wallet_id: WalletId, accounts: &[u32]) {
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

fn note(n: u8, value: u64) -> ShieldedNote {
    ShieldedNote {
        position: u64::from(n),
        cmx: [n; 32],
        nullifier: [n; 32],
        block_height: 1,
        is_spent: false,
        value,
        note_data: vec![n; 115],
    }
}

/// A running emitter whose updates arrive on a channel.
struct Emitter {
    updates: mpsc::UnboundedReceiver<ShieldedBalanceUpdate>,
    cancel: CancellationToken,
    join: JoinHandle<()>,
}

impl Emitter {
    fn start(coordinator: &Arc<NetworkShieldedCoordinator>) -> Self {
        let (sender, updates) = mpsc::unbounded_channel();
        let cancel = CancellationToken::new();
        let join = spawn_balance_emitter(
            Arc::clone(coordinator),
            Arc::new(move |update: &ShieldedBalanceUpdate| {
                let _ = sender.send(update.clone());
            }),
            cancel.clone(),
        );
        Self {
            updates,
            cancel,
            join,
        }
    }

    async fn next(&mut self) -> ShieldedBalanceUpdate {
        tokio::time::timeout(DELIVERY, self.updates.recv())
            .await
            .expect("an update in time")
            .expect("emitter alive")
    }

    async fn assert_quiet(&mut self) {
        if let Ok(update) = tokio::time::timeout(QUIET, self.updates.recv()).await {
            panic!("no update expected, got {update:?}");
        }
    }

    async fn stop(self) {
        self.cancel.cancel();
        tokio::time::timeout(DELIVERY, self.join)
            .await
            .expect("emitter joins")
            .expect("emitter did not panic");
    }
}

fn ordinary_credits(update: &ShieldedBalanceUpdate) -> u64 {
    match &update.state {
        ShieldedLocalBalanceState::Ready(snapshot) => {
            snapshot
                .ordinary_balance()
                .expect("ordinary balance")
                .spendable_credits
        }
        other => panic!("expected a ready ledger, got {other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_emit_an_update_when_a_wallet_is_first_registered_and_hydrated() {
    let coordinator = coordinator(&tree_path());
    let mut emitter = Emitter::start(&coordinator);
    emitter.assert_quiet().await;

    let wallet = [1; 32];
    register(&coordinator, wallet, &[0]).await;
    let first = emitter.next().await;
    assert_eq!(first.wallet_id, wallet);
    assert_eq!(first.revision, 1);
    assert_eq!(first.state, ShieldedLocalBalanceState::RestoreIncomplete);

    coordinator.mark_hydrated(wallet, true).await;
    let hydrated = emitter.next().await;
    assert_eq!(hydrated.revision, 2);
    assert_eq!(ordinary_credits(&hydrated), 0);
    emitter.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_emit_the_balance_a_completed_scan_found() {
    let keys = OrchardKeySet::from_seed(&[42; 64], dashcore::Network::Testnet, 0)
        .expect("test viewing key")
        .viewing_keys();
    let (sdk, _) = chunked_pool(&[vec![note_to(keys.default_address, 500)]]).await;
    let (coordinator, _host) = coordinator_over(sdk, &keys).await;
    let coordinator = Arc::new(coordinator);
    let mut emitter = Emitter::start(&coordinator);
    let first = emitter.next().await;
    assert_eq!(first.wallet_id, WALLET);
    assert_eq!(first.state, ShieldedLocalBalanceState::RestoreIncomplete);

    let outcome = coordinator.sync(true, &CancellationToken::new()).await;
    assert_eq!(outcome.success_count(), 1, "{outcome:?}");
    let scanned = loop {
        let update = emitter.next().await;
        if matches!(update.state, ShieldedLocalBalanceState::Ready(_)) {
            break update;
        }
    };
    assert_eq!(ordinary_credits(&scanned), 500);
    assert!(
        scanned.revision >= 2,
        "the scan's update follows the bind's: {scanned:?}"
    );
    emitter.stop().await;
}

/// A scan holds the store write lock for its whole download. The emitter
/// waits for it instead of giving up, and the update comes out when it is
/// released, here by a failing scan that ran behind the held store.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_emit_once_a_failed_scan_releases_the_store() {
    let coordinator = coordinator(&tree_path());
    let wallet = [2; 32];
    let id = SubwalletId::new(wallet, 0);
    register(&coordinator, wallet, &[0]).await;
    coordinator.mark_hydrated(wallet, true).await;
    let mut emitter = Emitter::start(&coordinator);
    assert_eq!(ordinary_credits(&emitter.next().await), 0);

    let mut store = coordinator.store().write().await;
    store.save_note(id, &note(1, 100)).expect("save");
    // A scan queued behind the held store; the mock pool has no answer for
    // it, so it fails once it gets the store.
    let scan = {
        let coordinator = Arc::clone(&coordinator);
        tokio::spawn(async move { coordinator.sync(true, &CancellationToken::new()).await })
    };
    emitter.assert_quiet().await;
    drop(store);

    let update = emitter.next().await;
    assert_eq!(update.revision, 2);
    assert_eq!(ordinary_credits(&update), 100);
    let outcome = tokio::time::timeout(DELIVERY, scan)
        .await
        .expect("scan ends")
        .expect("scan task");
    assert_eq!(outcome.success_count(), 0, "the scan fails: {outcome:?}");
    emitter.assert_quiet().await;

    // The failed scan left the emitter watching.
    coordinator
        .store()
        .write()
        .await
        .save_note(id, &note(2, 50))
        .expect("save");
    let after = emitter.next().await;
    assert_eq!(after.revision, 3);
    assert_eq!(ordinary_credits(&after), 150);
    emitter.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_emit_when_notes_are_reserved_and_when_each_kind_of_release_lands() {
    let coordinator = coordinator(&tree_path());
    let wallet = [3; 32];
    let id = SubwalletId::new(wallet, 0);
    register(&coordinator, wallet, &[0]).await;
    coordinator.mark_hydrated(wallet, true).await;
    coordinator
        .store()
        .write()
        .await
        .save_note(id, &note(1, 100))
        .expect("save");
    let mut emitter = Emitter::start(&coordinator);
    assert_eq!(ordinary_credits(&emitter.next().await), 100);

    let store = coordinator.store();
    // Reserved, then released by `cancel`.
    let reservation = {
        let mut locked = store.write().await;
        mark_notes_pending(&mut *locked, store, id, &[note(1, 100)]).expect("reserve")
    };
    assert_eq!(ordinary_credits(&emitter.next().await), 0);
    reservation.cancel().await;
    assert_eq!(ordinary_credits(&emitter.next().await), 100);

    // Reserved, then released by a drop that finds the store free.
    let reservation = {
        let mut locked = store.write().await;
        mark_notes_pending(&mut *locked, store, id, &[note(1, 100)]).expect("reserve")
    };
    assert_eq!(ordinary_credits(&emitter.next().await), 0);
    drop(reservation);
    assert_eq!(ordinary_credits(&emitter.next().await), 100);

    // Reserved, then dropped while the store is held: the release runs on a
    // spawned task with only the store in hand, once the store frees.
    let reservation = {
        let mut locked = store.write().await;
        mark_notes_pending(&mut *locked, store, id, &[note(1, 100)]).expect("reserve")
    };
    let reserved = emitter.next().await;
    assert_eq!(ordinary_credits(&reserved), 0);
    let held = store.write().await;
    drop(reservation);
    emitter.assert_quiet().await;
    drop(held);
    let released = emitter.next().await;
    assert_eq!(ordinary_credits(&released), 100);
    assert_eq!(released.revision, reserved.revision + 1);
    emitter.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_emit_unbound_when_a_wallet_is_unregistered() {
    let coordinator = coordinator(&tree_path());
    let wallet = [4; 32];
    register(&coordinator, wallet, &[0]).await;
    coordinator.mark_hydrated(wallet, true).await;
    let mut emitter = Emitter::start(&coordinator);
    assert_eq!(ordinary_credits(&emitter.next().await), 0);

    coordinator.unregister_wallet(wallet).await;
    let update = emitter.next().await;
    assert_eq!(update.revision, 2);
    assert_eq!(update.state, ShieldedLocalBalanceState::Unbound);
    emitter.assert_quiet().await;

    // Bound again: the revisions continue.
    register(&coordinator, wallet, &[0]).await;
    let rebound = emitter.next().await;
    assert_eq!(rebound.revision, 3);
    assert_eq!(rebound.state, ShieldedLocalBalanceState::RestoreIncomplete);
    emitter.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_emit_unbound_after_a_successful_clear() {
    let coordinator = coordinator(&tree_path());
    let wallet = [5; 32];
    register(&coordinator, wallet, &[0]).await;
    coordinator.mark_hydrated(wallet, true).await;
    let mut emitter = Emitter::start(&coordinator);
    assert_eq!(ordinary_credits(&emitter.next().await), 0);

    coordinator.clear().await.expect("clear");
    let update = emitter.next().await;
    assert_eq!(update.state, ShieldedLocalBalanceState::Unbound);
    emitter.assert_quiet().await;
    emitter.stop().await;
}

/// A failed Clear keeps the registration but invalidates hydration, so the
/// wallet the host showed as ready is no longer known to be.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_emit_restore_incomplete_after_a_clear_that_fails() {
    let path = tree_path();
    let coordinator = coordinator(&path);
    let wallet = [6; 32];
    register(&coordinator, wallet, &[0]).await;
    coordinator.mark_hydrated(wallet, true).await;
    coordinator
        .store()
        .write()
        .await
        .save_note(SubwalletId::new(wallet, 0), &note(1, 900))
        .expect("save");
    let mut emitter = Emitter::start(&coordinator);
    assert_eq!(ordinary_credits(&emitter.next().await), 900);

    // A real SQLite schema error makes the purge fail after the tree reset.
    let conn = rusqlite::Connection::open(&path).expect("open db");
    conn.execute_batch("ALTER TABLE shielded_pending_spends RENAME TO unavailable_pending_spends;")
        .expect("break schema");
    assert!(coordinator.clear().await.is_err());
    conn.execute_batch("ALTER TABLE unavailable_pending_spends RENAME TO shielded_pending_spends;")
        .expect("restore schema");
    drop(conn);

    let update = emitter.next().await;
    assert_eq!(update.revision, 2);
    assert_eq!(update.state, ShieldedLocalBalanceState::RestoreIncomplete);
    emitter.assert_quiet().await;
    emitter.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_not_emit_again_when_the_state_did_not_change() {
    let coordinator = coordinator(&tree_path());
    let wallet = [7; 32];
    let id = SubwalletId::new(wallet, 0);
    register(&coordinator, wallet, &[0]).await;
    coordinator.mark_hydrated(wallet, true).await;
    coordinator
        .store()
        .write()
        .await
        .save_note(id, &note(1, 100))
        .expect("save");
    let mut emitter = Emitter::start(&coordinator);
    assert_eq!(ordinary_credits(&emitter.next().await), 100);

    // Each of these raises the signal and leaves the state as it was.
    coordinator
        .store()
        .write()
        .await
        .save_note(id, &note(1, 100))
        .expect("save the same note");
    coordinator.mark_hydrated(wallet, true).await;
    register(&coordinator, wallet, &[0]).await;
    emitter.assert_quiet().await;
    emitter.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_number_each_wallet_s_updates_on_its_own_strictly_increasing_revisions() {
    let coordinator = coordinator(&tree_path());
    let first = [8; 32];
    let second = [9; 32];
    let mut emitter = Emitter::start(&coordinator);

    register(&coordinator, first, &[0]).await;
    coordinator.mark_hydrated(first, true).await;
    register(&coordinator, second, &[0]).await;
    for value in [10, 20, 30] {
        coordinator
            .store()
            .write()
            .await
            .save_note(SubwalletId::new(first, 0), &note(value as u8, value))
            .expect("save");
    }
    coordinator.mark_hydrated(second, true).await;
    coordinator
        .store()
        .write()
        .await
        .save_note(SubwalletId::new(second, 0), &note(99, 7))
        .expect("save");

    let mut last: std::collections::BTreeMap<WalletId, (u64, u64)> = Default::default();
    // Wait until both wallets show their final balances; updates may
    // coalesce, but each wallet's revisions must rise by one each time.
    while last.get(&first).map(|(_, credits)| *credits) != Some(60)
        || last.get(&second).map(|(_, credits)| *credits) != Some(7)
    {
        let update = emitter.next().await;
        let expected = last
            .get(&update.wallet_id)
            .map_or(1, |(revision, _)| revision + 1);
        assert_eq!(update.revision, expected, "{update:?}");
        let credits = match &update.state {
            ShieldedLocalBalanceState::Ready(snapshot) => {
                snapshot
                    .ordinary_balance()
                    .expect("ordinary balance")
                    .spendable_credits
            }
            _ => 0,
        };
        last.insert(update.wallet_id, (update.revision, credits));
    }
    emitter.assert_quiet().await;
    emitter.stop().await;
}

/// Records every balance update the manager's event fan-out delivers.
#[derive(Default)]
struct BalanceRecorder {
    updates: Mutex<Vec<ShieldedBalanceUpdate>>,
}

impl EventHandler for BalanceRecorder {}

impl PlatformEventHandler for BalanceRecorder {
    fn on_shielded_balance_changed(&self, update: &ShieldedBalanceUpdate) {
        self.updates.lock().expect("recorder").push(update.clone());
    }
}

impl BalanceRecorder {
    fn count(&self) -> usize {
        self.updates.lock().expect("recorder").len()
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_deliver_nothing_after_shutdown_returns() {
    let recorder = Arc::new(BalanceRecorder::default());
    let manager = PlatformWalletManager::new(
        Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk")),
        Arc::new(NoPlatformPersistence),
        Arc::clone(&recorder) as Arc<dyn PlatformEventHandler>,
    );
    manager
        .configure_shielded(tree_path())
        .await
        .expect("configure shielded");
    let coordinator = manager.shielded_coordinator().await.expect("coordinator");
    let wallet = [10; 32];
    register(&coordinator, wallet, &[0]).await;
    let deadline = tokio::time::Instant::now() + DELIVERY;
    while recorder.count() == 0 {
        assert!(
            tokio::time::Instant::now() < deadline,
            "the manager's emitter delivers the bind"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert_eq!(
        recorder.updates.lock().expect("recorder")[0].state,
        ShieldedLocalBalanceState::RestoreIncomplete
    );

    let report = manager.shutdown().await;
    assert_eq!(
        report.per_worker.get(&WalletWorker::ShieldedBalanceEmitter),
        Some(&WorkerStatus::Ok),
        "{report:?}"
    );
    let delivered = recorder.count();

    // The store and the registry keep changing after shutdown.
    coordinator.mark_hydrated(wallet, true).await;
    coordinator
        .store()
        .write()
        .await
        .save_note(SubwalletId::new(wallet, 0), &note(1, 100))
        .expect("save");
    coordinator.unregister_wallet(wallet).await;
    tokio::time::sleep(QUIET).await;
    assert_eq!(recorder.count(), delivered, "nothing after shutdown");

    // A second shutdown finds the emitter gone.
    let again = manager.shutdown().await;
    assert_eq!(
        again.per_worker.get(&WalletWorker::ShieldedBalanceEmitter),
        Some(&WorkerStatus::NotRunning)
    );
}
