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
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use drive_proof_verifier::types::{ShieldedEncryptedNotes, ShieldedEncryptedNotesQuery};

use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use super::balance::{ShieldedBalanceUpdate, ShieldedLocalBalanceState};
use super::balance_emitter::spawn_balance_emitter;
use super::cancelled_scan_tests::{chunked_pool, coordinator_over, note_to, opaque_note, WALLET};
use super::operations::mark_notes_pending;
use super::{
    FileBackedShieldedStore, NetworkShieldedCoordinator, OrchardKeySet, ShieldedNote,
    ShieldedStore, SubwalletId,
};
use crate::events::{EventHandler, PlatformEventHandler, PlatformEventManager};
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
        Self::start_with(coordinator, |_| {})
    }

    /// [`Self::start`] whose sink runs `before_delivery` on each update before
    /// passing it on, on the emitter's task.
    fn start_with(
        coordinator: &Arc<NetworkShieldedCoordinator>,
        before_delivery: impl Fn(&ShieldedBalanceUpdate) + Send + Sync + 'static,
    ) -> Self {
        let (sender, updates) = mpsc::unbounded_channel();
        let cancel = CancellationToken::new();
        let join = spawn_balance_emitter(
            Arc::clone(coordinator),
            Arc::new(move |update: &ShieldedBalanceUpdate| {
                before_delivery(update);
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

/// Holds the thread that reaches it, once armed, until released: stands in
/// for a scan paused inside its download or a host callback that does not
/// return.
#[derive(Default)]
struct Pause {
    state: Mutex<PauseState>,
    wake: Condvar,
}

#[derive(Default)]
struct PauseState {
    armed: bool,
    entered: bool,
    released: bool,
}

impl Pause {
    fn armed() -> Arc<Self> {
        let pause = Arc::new(Self::default());
        pause.state.lock().expect("pause").armed = true;
        pause
    }

    fn arm(&self) {
        self.state.lock().expect("pause").armed = true;
    }

    /// Block here until released when armed; a later call passes.
    fn hold_if_armed(&self) {
        let mut state = self.state.lock().expect("pause");
        if !state.armed {
            return;
        }
        state.armed = false;
        state.entered = true;
        self.wake.notify_all();
        while !state.released {
            state = self.wake.wait(state).expect("pause");
        }
    }

    async fn wait_entered(&self) {
        let deadline = tokio::time::Instant::now() + DELIVERY;
        while !self.state.lock().expect("pause").entered {
            assert!(tokio::time::Instant::now() < deadline, "pause reached");
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    }

    fn release(&self) {
        // Also reached from `PauseRelease::drop` while a failed assertion
        // unwinds, so a poisoned lock is taken over rather than unwrapped.
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.released = true;
        self.wake.notify_all();
    }

    /// Release the pause when the returned guard drops, on every exit of the
    /// test, a failed assertion included: a worker parked here would
    /// otherwise keep the runtime, and the suite, from finishing.
    fn release_on_drop(self: &Arc<Self>) -> PauseRelease {
        PauseRelease(Arc::clone(self))
    }
}

/// Releases its [`Pause`] when dropped.
struct PauseRelease(Arc<Pause>);

impl Drop for PauseRelease {
    fn drop(&mut self) {
        self.0.release();
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

/// A pool whose first chunk is full and served, and whose later chunks the
/// mock has no answer for: a scan takes the store, commits the first chunk,
/// then fails on the next fetch.
async fn pool_failing_after_one_chunk() -> dash_sdk::Sdk {
    let mut sdk = dash_sdk::Sdk::new_mock();
    let count = 2048
        * u64::from(
            sdk.version()
                .drive_abci
                .query
                .shielded_queries
                .max_query_chunks,
        );
    sdk.mock()
        .expect_fetch(
            ShieldedEncryptedNotesQuery {
                start_index: 0,
                count: count as u32,
            },
            Some(ShieldedEncryptedNotes {
                notes: (0..count).map(opaque_note).collect(),
                total_count: count * 2,
            }),
        )
        .await
        .expect("mock first chunk");
    sdk
}

/// A scan holds the store write lock for its whole download. A change that
/// lands meanwhile is read by the emitter, which waits for the store instead
/// of giving up, and the update comes out once the scan releases it, here
/// after the scan failed. The scan is paused inside its download, after it
/// took the store and committed its first chunk.
#[tokio::test(flavor = "multi_thread", worker_threads = 3)]
async fn should_emit_once_a_failed_scan_releases_the_store() {
    let coordinator = coordinator_with(pool_failing_after_one_chunk().await, &tree_path());
    let wallet = [2; 32];
    register(&coordinator, wallet, &[0]).await;
    let mut emitter = Emitter::start(&coordinator);
    let first = emitter.next().await;
    assert_eq!(first.state, ShieldedLocalBalanceState::RestoreIncomplete);

    let pause = Pause::armed();
    let _release = pause.release_on_drop();
    {
        let pause = Arc::clone(&pause);
        coordinator.install_tree_progress_handler(Some(Arc::new(move |_, _| {
            pause.hold_if_armed();
        })));
    }
    let scan = {
        let coordinator = Arc::clone(&coordinator);
        tokio::spawn(async move { coordinator.sync(true, &CancellationToken::new()).await })
    };
    pause.wait_entered().await;
    assert!(
        coordinator.store().try_write().is_err(),
        "the paused scan holds the store"
    );

    // Needs no store: the emitter wakes and waits for the scan's store.
    coordinator.mark_hydrated(wallet, true).await;
    emitter.assert_quiet().await;

    pause.release();
    let update = emitter.next().await;
    assert_eq!(update.revision, 2);
    assert_eq!(ordinary_credits(&update), 0);
    let outcome = tokio::time::timeout(DELIVERY, scan)
        .await
        .expect("scan ends")
        .expect("scan task");
    assert_eq!(outcome.success_count(), 0, "the scan fails: {outcome:?}");
    assert_eq!(outcome.error_count(), 1, "the scan fails: {outcome:?}");
    emitter.assert_quiet().await;

    // The failed scan left the emitter watching.
    coordinator
        .store()
        .write()
        .await
        .save_note(SubwalletId::new(wallet, 0), &note(1, 50))
        .expect("save");
    let after = emitter.next().await;
    assert_eq!(after.revision, 3);
    assert_eq!(ordinary_credits(&after), 50);
    emitter.stop().await;
}

/// A wallet that was unregistered is read first in a pass: its `Unbound`
/// needs no store, so it is not held up behind a bound wallet's read while
/// a scan holds the store.
#[tokio::test(flavor = "multi_thread", worker_threads = 3)]
async fn should_emit_an_unregistered_wallet_before_bound_ones_wait_for_the_store() {
    let coordinator = coordinator(&tree_path());
    let (first, unregistered, last) = ([0x11; 32], [0x12; 32], [0x13; 32]);
    for wallet in [first, unregistered, last] {
        register(&coordinator, wallet, &[0]).await;
        coordinator.mark_hydrated(wallet, true).await;
    }
    let pause = Arc::new(Pause::default());
    let _release = pause.release_on_drop();
    let mut emitter = {
        let pause = Arc::clone(&pause);
        Emitter::start_with(&coordinator, move |update| {
            if update.wallet_id == last {
                pause.hold_if_armed();
            }
        })
    };
    let mut ready = std::collections::BTreeSet::new();
    while ready.len() < 3 {
        let update = emitter.next().await;
        if matches!(update.state, ShieldedLocalBalanceState::Ready(_)) {
            ready.insert(update.wallet_id);
        }
    }

    // Park the emitter in a delivery for the last wallet, unregister the
    // middle one, then hold the store as a scan would.
    pause.arm();
    coordinator
        .store()
        .write()
        .await
        .save_note(SubwalletId::new(last, 0), &note(1, 10))
        .expect("save");
    pause.wait_entered().await;
    coordinator.unregister_wallet(unregistered).await;
    let held = coordinator.store().write().await;
    pause.release();

    let delivered = emitter.next().await;
    assert_eq!(delivered.wallet_id, last);
    assert_eq!(ordinary_credits(&delivered), 10);
    let update = emitter.next().await;
    assert_eq!(update.wallet_id, unregistered);
    assert_eq!(update.state, ShieldedLocalBalanceState::Unbound);
    assert_eq!(update.revision, 2);
    emitter.assert_quiet().await;
    drop(held);
    emitter.stop().await;
}

/// Forwards balance updates to a channel, as a host handler would.
struct ChannelHandler(mpsc::UnboundedSender<ShieldedBalanceUpdate>);

impl EventHandler for ChannelHandler {}

impl PlatformEventHandler for ChannelHandler {
    fn on_shielded_balance_changed(&self, update: &ShieldedBalanceUpdate) {
        let _ = self.0.send(update.clone());
    }
}

struct PanickingHandler;

impl EventHandler for PanickingHandler {}

impl PlatformEventHandler for PanickingHandler {
    fn on_shielded_balance_changed(&self, _update: &ShieldedBalanceUpdate) {
        panic!("host handler failed");
    }
}

/// A handler that panics neither stops the emitter nor keeps the update
/// from the other handlers, now or later.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_keep_emitting_to_other_handlers_after_one_panics() {
    let coordinator = coordinator(&tree_path());
    let (sender, updates) = mpsc::unbounded_channel();
    let events = Arc::new(PlatformEventManager::new(vec![
        Arc::new(PanickingHandler) as Arc<dyn PlatformEventHandler>,
        Arc::new(ChannelHandler(sender)) as Arc<dyn PlatformEventHandler>,
    ]));
    let cancel = CancellationToken::new();
    let join = spawn_balance_emitter(
        Arc::clone(&coordinator),
        Arc::new(move |update: &ShieldedBalanceUpdate| events.on_shielded_balance_changed(update)),
        cancel.clone(),
    );
    let mut emitter = Emitter {
        updates,
        cancel,
        join,
    };
    let wallet = [0x21; 32];
    register(&coordinator, wallet, &[0]).await;
    assert_eq!(emitter.next().await.revision, 1);
    coordinator.mark_hydrated(wallet, true).await;
    let later = emitter.next().await;
    assert_eq!(later.revision, 2);
    assert_eq!(ordinary_credits(&later), 0);
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

/// The same failed Clear for a wallet whose ledger rests on a scan completed
/// in this session, with no hydration behind it. Its state used to stay
/// `Ready` with the pre-Clear amount, so nothing was emitted and the host
/// kept showing that amount as known.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_emit_restore_incomplete_after_a_failed_clear_of_a_scanned_wallet() {
    let path = tree_path();
    let coordinator = coordinator(&path);
    let wallet = [13; 32];
    let id = SubwalletId::new(wallet, 0);
    register(&coordinator, wallet, &[0]).await;
    {
        let mut store = coordinator.store().write().await;
        store.save_note(id, &note(1, 900)).expect("save");
        store.set_last_synced_note_index(id, 1).expect("scanned");
    }
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

/// A ledger the read cannot sum is delivered as not established, so a host
/// stops showing its earlier balance, and the next successful read brings
/// `Ready` back.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_report_an_unreadable_ledger_as_restore_incomplete_until_it_reads_again() {
    let coordinator = coordinator(&tree_path());
    let wallet = [0x41; 32];
    let id = SubwalletId::new(wallet, 0);
    register(&coordinator, wallet, &[0]).await;
    coordinator.mark_hydrated(wallet, true).await;
    coordinator
        .store()
        .write()
        .await
        .save_note(id, &note(1, 50))
        .expect("save");
    let mut emitter = Emitter::start(&coordinator);
    assert_eq!(ordinary_credits(&emitter.next().await), 50);

    // One account's notes now sum past `u64`: the read fails.
    coordinator
        .store()
        .write()
        .await
        .save_note(id, &note(2, u64::MAX))
        .expect("save");
    assert!(coordinator.local_balance_snapshot(wallet).await.is_err());
    let unreadable = emitter.next().await;
    assert_eq!(unreadable.revision, 2);
    assert_eq!(
        unreadable.state,
        ShieldedLocalBalanceState::RestoreIncomplete
    );
    emitter.assert_quiet().await;

    // Reserving the oversized note makes the ledger readable again.
    coordinator
        .store()
        .write()
        .await
        .mark_pending(id, &[2; 32])
        .expect("reserve");
    let readable = emitter.next().await;
    assert_eq!(readable.revision, 3);
    assert_eq!(ordinary_credits(&readable), 50);
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

/// Holds the first balance update it receives until released, as a host
/// callback that has not returned yet.
struct BlockingHandler {
    pause: Arc<Pause>,
}

impl EventHandler for BlockingHandler {}

impl PlatformEventHandler for BlockingHandler {
    fn on_shielded_balance_changed(&self, _update: &ShieldedBalanceUpdate) {
        self.pause.hold_if_armed();
    }
}

/// A `shutdown` dropped while it waits for the emitter, whose host callback
/// is still running, leaves the emitter owned: the next `shutdown` joins it
/// and reports it, instead of finding nothing to join.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn should_join_the_emitter_on_a_later_shutdown_after_one_was_dropped_mid_join() {
    let pause = Pause::armed();
    let _release = pause.release_on_drop();
    let manager = PlatformWalletManager::new(
        Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk")),
        Arc::new(NoPlatformPersistence),
        Arc::new(BlockingHandler {
            pause: Arc::clone(&pause),
        }) as Arc<dyn PlatformEventHandler>,
    );
    manager
        .configure_shielded(tree_path())
        .await
        .expect("configure shielded");
    let coordinator = manager.shielded_coordinator().await.expect("coordinator");
    register(&coordinator, [0x31; 32], &[0]).await;
    pause.wait_entered().await;

    // Run the first shutdown until it waits for the emitter's join, then
    // drop it there, with the host callback still in progress.
    {
        let shutdown = manager.shutdown();
        tokio::pin!(shutdown);
        tokio::select! {
            _ = &mut shutdown => panic!("shutdown returned while the callback was in progress"),
            _ = manager.shielded_balance_emitter_join_reached.notified() => {}
        }
        tokio::select! {
            _ = &mut shutdown => panic!("shutdown returned while the callback was in progress"),
            _ = tokio::time::sleep(QUIET) => {}
        }
    }

    pause.release();
    let report = manager.shutdown().await;
    assert_eq!(
        report.per_worker.get(&WalletWorker::ShieldedBalanceEmitter),
        Some(&WorkerStatus::Ok),
        "{report:?}"
    );
}
