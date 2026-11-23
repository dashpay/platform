//! The shielded lifecycle boundary: a manager-owned coordinator, one
//! admission protocol, and a Clear that runs the host wipe itself.
//!
//! Every durable shielded mutation holds the coordinator's admission
//! shared; Clear holds it exclusively across quiesce, native reset, host
//! wipe and epoch bump. These tests park one side at a host callback with
//! a gate and check the other side neither overtakes it nor leaves state
//! behind.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

use dashcore::Network;
use key_wallet::wallet::initialization::WalletAccountCreationOptions;

use crate::changeset::{
    ClientStartState, PersistenceCapabilities, PersistenceError, PlatformWalletChangeSet,
    PlatformWalletPersistence, ShieldedSubwalletStartState,
};
use crate::error::PlatformWalletError;
use crate::events::{EventHandler, PlatformEventHandler};
use crate::wallet::platform_wallet::{PlatformWallet, WalletId};
use crate::wallet::shielded::store::ShieldedStore;
use crate::wallet::shielded::{NetworkShieldedCoordinator, ShieldedNote, SubwalletId};
use crate::PlatformWalletManager;

const SEED: [u8; 64] = [0x42; 64];

struct NoopEventHandler;
impl EventHandler for NoopEventHandler {}
impl PlatformEventHandler for NoopEventHandler {}

type WipeHook = Box<dyn FnMut() + Send>;

/// A host that keeps shielded viewing keys and per-subwallet rows the way a
/// Swift or Kotlin store does, deletes them on `wipe_shielded_state`, and
/// records the order of its shielded callbacks.
struct BoundaryHost {
    capabilities: PersistenceCapabilities,
    viewing_keys: Mutex<BTreeMap<SubwalletId, Vec<u8>>>,
    subwallets: Mutex<BTreeMap<SubwalletId, ShieldedSubwalletStartState>>,
    events: Mutex<Vec<&'static str>>,
    /// Shielded activity rows stored.
    activity_rows: AtomicUsize,
    wipes: AtomicUsize,
    wiped_exclusively: AtomicBool,
    fail_wipe: AtomicBool,
    /// Parks the next viewing-key store until released.
    store_gate: Mutex<Option<mpsc::Receiver<()>>>,
    /// Parks the next load until released.
    load_gate: Mutex<Option<mpsc::Receiver<()>>>,
    /// Set by the test once the manager is configured, so a wipe can tell
    /// whether Clear holds the admission exclusively.
    coordinator: Mutex<Option<Arc<NetworkShieldedCoordinator>>>,
    on_wipe: Mutex<Option<WipeHook>>,
}

impl BoundaryHost {
    fn new() -> Arc<Self> {
        Self::with_capabilities(
            PersistenceCapabilities::SHIELDED_FVK_RESTART
                .union(PersistenceCapabilities::SHIELDED_STATE_WIPE),
        )
    }

    fn with_capabilities(capabilities: PersistenceCapabilities) -> Arc<Self> {
        Arc::new(Self {
            capabilities,
            viewing_keys: Mutex::default(),
            subwallets: Mutex::default(),
            events: Mutex::default(),
            activity_rows: AtomicUsize::new(0),
            wipes: AtomicUsize::new(0),
            wiped_exclusively: AtomicBool::new(true),
            fail_wipe: AtomicBool::new(false),
            store_gate: Mutex::default(),
            load_gate: Mutex::default(),
            coordinator: Mutex::default(),
            on_wipe: Mutex::default(),
        })
    }

    fn gate(slot: &Mutex<Option<mpsc::Receiver<()>>>) -> mpsc::Sender<()> {
        let (tx, rx) = mpsc::channel();
        *slot.lock().unwrap() = Some(rx);
        tx
    }

    fn event(&self, event: &'static str) {
        self.events.lock().unwrap().push(event);
    }

    fn clear_events(&self) {
        self.events.lock().unwrap().clear();
    }

    fn events(&self) -> Vec<&'static str> {
        self.events.lock().unwrap().clone()
    }

    fn wipe_count(&self) -> usize {
        self.wipes.load(Ordering::SeqCst)
    }

    fn viewing_key_rows(&self) -> usize {
        self.viewing_keys.lock().unwrap().len()
    }

    /// Wait until the host logged `event`, failing after a generous bound.
    async fn reached(&self, event: &'static str) {
        for _ in 0..1_000 {
            if self.events().contains(&event) {
                return;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        panic!("the host never logged {event:?}: {:?}", self.events());
    }
}

impl PlatformWalletPersistence for BoundaryHost {
    fn persistence_capabilities(&self) -> PersistenceCapabilities {
        self.capabilities
    }

    fn store(
        &self,
        _wallet_id: WalletId,
        changeset: PlatformWalletChangeSet,
    ) -> Result<(), PersistenceError> {
        let Some(shielded) = changeset.shielded else {
            return Ok(());
        };
        self.activity_rows.fetch_add(
            shielded.activity_entries.values().map(Vec::len).sum(),
            Ordering::SeqCst,
        );
        if shielded.viewing_keys.is_empty() {
            return Ok(());
        }
        self.event("store viewing keys");
        let gate = self.store_gate.lock().unwrap().take();
        if let Some(gate) = gate {
            let _ = gate.recv();
        }
        self.viewing_keys
            .lock()
            .unwrap()
            .extend(shielded.viewing_keys);
        self.event("stored viewing keys");
        Ok(())
    }

    fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
        Ok(())
    }

    fn load(&self) -> Result<ClientStartState, PersistenceError> {
        self.event("load");
        let gate = self.load_gate.lock().unwrap().take();
        if let Some(gate) = gate {
            let _ = gate.recv();
        }
        let mut start = ClientStartState::default();
        start.shielded.viewing_keys = self.viewing_keys.lock().unwrap().clone();
        start.shielded.per_subwallet = self.subwallets.lock().unwrap().clone();
        Ok(start)
    }

    fn wipe_shielded_state(&self, _network: Network) -> Result<(), PersistenceError> {
        self.event("wipe");
        self.wipes.fetch_add(1, Ordering::SeqCst);
        if let Some(coordinator) = self.coordinator.lock().unwrap().as_ref() {
            if !coordinator.admission_is_closed() {
                self.wiped_exclusively.store(false, Ordering::SeqCst);
            }
        }
        if let Some(hook) = self.on_wipe.lock().unwrap().as_mut() {
            hook();
        }
        if self.fail_wipe.load(Ordering::SeqCst) {
            return Err(PersistenceError::backend("injected host wipe failure"));
        }
        self.viewing_keys.lock().unwrap().clear();
        self.subwallets.lock().unwrap().clear();
        self.event("wiped");
        Ok(())
    }
}

/// A manager over `host` with one wallet, its shielded store configured
/// under `dir`.
async fn configured(
    host: &Arc<BoundaryHost>,
    dir: &tempfile::TempDir,
) -> (
    Arc<PlatformWalletManager<BoundaryHost>>,
    Arc<PlatformWallet>,
    Arc<NetworkShieldedCoordinator>,
) {
    let (manager, wallet) = unconfigured(host).await;
    manager
        .configure_shielded(dir.path().join("tree.sqlite"))
        .await
        .expect("the shielded store is configured");
    let coordinator = manager
        .shielded_coordinator()
        .await
        .expect("the manager's coordinator");
    *host.coordinator.lock().unwrap() = Some(Arc::clone(&coordinator));
    host.clear_events();
    (manager, wallet, coordinator)
}

async fn unconfigured(
    host: &Arc<BoundaryHost>,
) -> (
    Arc<PlatformWalletManager<BoundaryHost>>,
    Arc<PlatformWallet>,
) {
    let manager = Arc::new(PlatformWalletManager::new(
        Arc::new(dash_sdk::Sdk::new_mock()),
        Arc::clone(host),
        Arc::new(NoopEventHandler),
    ));
    let wallet = manager
        .create_wallet_from_seed_bytes(
            Network::Testnet,
            &[0x17; 64],
            WalletAccountCreationOptions::Default,
            Some(0),
        )
        .await
        .expect("wallet");
    (manager, wallet)
}

/// Host callbacks of a bind that finished before a Clear.
const BIND_THEN_CLEAR: [&str; 5] = [
    "load",
    "store viewing keys",
    "stored viewing keys",
    "wipe",
    "wiped",
];
/// Host callbacks of a Clear followed by a bind (or account add).
const CLEAR_THEN_BIND: [&str; 5] = [
    "wipe",
    "wiped",
    "load",
    "store viewing keys",
    "stored viewing keys",
];

fn spawn_clear(
    manager: &Arc<PlatformWalletManager<BoundaryHost>>,
) -> tokio::task::JoinHandle<Result<(), PlatformWalletError>> {
    let manager = Arc::clone(manager);
    tokio::spawn(async move { manager.clear_shielded().await })
}

/// Wait until a Clear holds or is queued on `coordinator`'s admission, so
/// what the test checks next happens after Clear reached it.
async fn until_admission_closed(coordinator: &NetworkShieldedCoordinator) {
    for _ in 0..1_000 {
        if coordinator.admission_is_closed() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    panic!("Clear never reached the admission");
}

/// Host rows a pre-Clear session left: one note and a caught-up watermark.
fn pre_clear_rows(wallet_id: WalletId) -> BTreeMap<SubwalletId, ShieldedSubwalletStartState> {
    BTreeMap::from([(
        SubwalletId::new(wallet_id, 0),
        ShieldedSubwalletStartState {
            notes: vec![ShieldedNote {
                position: 7,
                cmx: [0x2A; 32],
                nullifier: [0xE0; 32],
                block_height: 100,
                is_spent: false,
                value: 1_000,
                note_data: vec![0u8; 115],
            }],
            has_sync_state: true,
            last_synced_index: 900_000,
            ..Default::default()
        },
    )])
}

/// Neither the pre-Clear note nor its watermark survives in the native
/// store, and no account is registered.
async fn assert_native_state_cleared(coordinator: &NetworkShieldedCoordinator, id: SubwalletId) {
    let store = coordinator.store().read().await;
    assert!(store.get_all_notes(id).unwrap().is_empty());
    assert_eq!(store.last_synced_note_index(id).unwrap(), 0);
    drop(store);
    assert!(coordinator.registered_subwallets().await.is_empty());
}

/// A seed bind parked in its viewing-key write holds Clear off until it is
/// done; Clear then resets it and wipes the row it wrote, so nothing of the
/// bind outlives the Clear.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn should_wipe_the_viewing_keys_of_a_bind_whose_host_write_was_delayed_across_clear() {
    let dir = tempfile::tempdir().unwrap();
    let host = BoundaryHost::new();
    let (manager, wallet, coordinator) = configured(&host, &dir).await;
    let id = SubwalletId::new(wallet.wallet_id(), 0);

    let release = BoundaryHost::gate(&host.store_gate);
    let bind = {
        let wallet = Arc::clone(&wallet);
        tokio::spawn(async move { wallet.bind_shielded(&SEED, &[0]).await })
    };
    host.reached("store viewing keys").await;
    let clear = spawn_clear(&manager);
    until_admission_closed(&coordinator).await;
    assert_eq!(host.wipe_count(), 0, "Clear waits for the admitted bind");
    assert!(!clear.is_finished());

    release.send(()).unwrap();
    bind.await.unwrap().expect("the bind finishes first");
    clear.await.unwrap().expect("then Clear runs");

    assert_eq!(host.wipe_count(), 1);
    assert_eq!(
        host.viewing_key_rows(),
        0,
        "no viewing-key row outlives Clear"
    );
    assert_eq!(host.events(), BIND_THEN_CLEAR);
    assert_native_state_cleared(&coordinator, id).await;
}

/// A seedless rebind parked in its host load holds Clear off; Clear then
/// purges what it restored, so the pre-Clear snapshot it read restores
/// nothing that survives.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn should_restore_nothing_that_survives_from_a_snapshot_read_before_clear() {
    let dir = tempfile::tempdir().unwrap();
    let host = BoundaryHost::new();
    let (manager, wallet, coordinator) = configured(&host, &dir).await;
    let id = SubwalletId::new(wallet.wallet_id(), 0);
    wallet.bind_shielded(&SEED, &[0]).await.unwrap();
    *host.subwallets.lock().unwrap() = pre_clear_rows(wallet.wallet_id());
    host.clear_events();

    let release = BoundaryHost::gate(&host.load_gate);
    let rebind = {
        let wallet = Arc::clone(&wallet);
        tokio::spawn(async move { wallet.bind_shielded_from_persisted(&[0]).await })
    };
    host.reached("load").await;
    let clear = spawn_clear(&manager);
    until_admission_closed(&coordinator).await;
    assert_eq!(host.wipe_count(), 0, "Clear waits for the admitted rebind");

    release.send(()).unwrap();
    assert!(rebind.await.unwrap().expect("the rebind finishes first"));
    clear.await.unwrap().expect("then Clear runs");

    assert_eq!(host.events(), vec!["load", "wipe", "wiped"]);
    assert_eq!(host.viewing_key_rows(), 0);
    assert!(host.subwallets.lock().unwrap().is_empty());
    assert_native_state_cleared(&coordinator, id).await;
}

/// A bind that starts while the host wipe runs waits for the whole Clear
/// and then reads the wiped host: the pre-Clear rows it would have read a
/// moment earlier are not restored.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn should_hold_a_bind_started_during_the_wipe_until_clear_finishes() {
    let dir = tempfile::tempdir().unwrap();
    let host = BoundaryHost::new();
    let (manager, wallet, coordinator) = configured(&host, &dir).await;
    let id = SubwalletId::new(wallet.wallet_id(), 0);
    wallet.bind_shielded(&SEED, &[0]).await.unwrap();
    *host.subwallets.lock().unwrap() = pre_clear_rows(wallet.wallet_id());
    host.clear_events();

    type Bind = tokio::task::JoinHandle<Result<(), PlatformWalletError>>;
    let started: Arc<Mutex<Option<Bind>>> = Arc::default();
    let held_during_wipe = Arc::new(AtomicBool::new(false));
    {
        let wallet = Arc::clone(&wallet);
        let started = Arc::clone(&started);
        let held_during_wipe = Arc::clone(&held_during_wipe);
        let runtime = tokio::runtime::Handle::current();
        *host.on_wipe.lock().unwrap() = Some(Box::new(move || {
            let wallet = Arc::clone(&wallet);
            let bind = runtime.spawn(async move { wallet.bind_shielded(&SEED, &[0]).await });
            std::thread::sleep(Duration::from_millis(100));
            held_during_wipe.store(!bind.is_finished(), Ordering::SeqCst);
            *started.lock().unwrap() = Some(bind);
        }));
    }

    manager.clear_shielded().await.expect("Clear succeeds");
    assert!(held_during_wipe.load(Ordering::SeqCst));
    let bind = started
        .lock()
        .unwrap()
        .take()
        .expect("the hook started a bind");
    bind.await.unwrap().expect("the bind runs after Clear");

    assert_eq!(
        host.events(),
        CLEAR_THEN_BIND,
        "the bind reads the host only after the wipe"
    );
    let store = coordinator.store().read().await;
    assert!(store.get_all_notes(id).unwrap().is_empty());
    assert_eq!(store.last_synced_note_index(id).unwrap(), 0);
    drop(store);
    assert_eq!(coordinator.registered_subwallets().await, vec![id]);
}

/// Adding an account is admitted like a bind: Clear waits for one parked in
/// its host load, then removes the account it added.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn should_order_an_account_add_and_a_clear() {
    let dir = tempfile::tempdir().unwrap();
    let host = BoundaryHost::new();
    let (manager, wallet, coordinator) = configured(&host, &dir).await;
    wallet.bind_shielded(&SEED, &[0]).await.unwrap();
    host.clear_events();

    let release = BoundaryHost::gate(&host.load_gate);
    let add = {
        let wallet = Arc::clone(&wallet);
        tokio::spawn(async move { wallet.shielded_add_account(&SEED, 1).await })
    };
    host.reached("load").await;
    let clear = spawn_clear(&manager);
    until_admission_closed(&coordinator).await;
    assert_eq!(
        host.wipe_count(),
        0,
        "Clear waits for the admitted account add"
    );

    release.send(()).unwrap();
    add.await.unwrap().expect("the add finishes first");
    clear.await.unwrap().expect("then Clear runs");

    assert_eq!(host.events(), BIND_THEN_CLEAR);
    assert_eq!(host.viewing_key_rows(), 0);
    assert!(coordinator.registered_subwallets().await.is_empty());

    // An add started while the wipe runs waits for the whole Clear, then
    // reads the wiped host.
    type Add = tokio::task::JoinHandle<Result<(), PlatformWalletError>>;
    let started: Arc<Mutex<Option<Add>>> = Arc::default();
    {
        let wallet = Arc::clone(&wallet);
        let started = Arc::clone(&started);
        let runtime = tokio::runtime::Handle::current();
        *host.on_wipe.lock().unwrap() = Some(Box::new(move || {
            let wallet = Arc::clone(&wallet);
            *started.lock().unwrap() =
                Some(runtime.spawn(async move { wallet.shielded_add_account(&SEED, 2).await }));
            std::thread::sleep(Duration::from_millis(100));
        }));
    }
    host.clear_events();
    manager.clear_shielded().await.expect("the second Clear");
    let add = started
        .lock()
        .unwrap()
        .take()
        .expect("the hook started an add");
    add.await.unwrap().expect("the add runs after Clear");
    assert_eq!(
        host.events(),
        CLEAR_THEN_BIND,
        "the add reads the host only after the wipe"
    );
    assert_eq!(
        host.viewing_key_rows(),
        1,
        "only the post-Clear account row"
    );
}

/// A bind queued behind a long shield-class operation on its wallet (a
/// funding waiting on its ChainLock holds `shield_guard`) holds no
/// admission, so it does not stall a Clear.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn should_clear_while_a_bind_waits_for_the_wallets_shield_guard() {
    let dir = tempfile::tempdir().unwrap();
    let host = BoundaryHost::new();
    let (manager, wallet, coordinator) = configured(&host, &dir).await;
    wallet.bind_shielded(&SEED, &[0]).await.unwrap();
    host.clear_events();

    let long_operation = wallet.shield_guard.lock().await;
    let bind = {
        let wallet = Arc::clone(&wallet);
        tokio::spawn(async move { wallet.bind_shielded(&SEED, &[0]).await })
    };
    tokio::time::sleep(Duration::from_millis(200)).await;
    tokio::time::timeout(Duration::from_secs(5), manager.clear_shielded())
        .await
        .expect("Clear does not wait for the queued bind")
        .expect("Clear succeeds");
    assert!(!bind.is_finished(), "the bind still waits for the guard");

    drop(long_operation);
    bind.await.unwrap().expect("the bind runs after Clear");
    assert_eq!(host.events(), CLEAR_THEN_BIND);
    assert_eq!(
        coordinator.registered_subwallets().await,
        vec![SubwalletId::new(wallet.wallet_id(), 0)]
    );
}

/// A shielded operation that keeps its admission past the budget fails the
/// Clear with nothing reset and the host never called.
#[tokio::test(start_paused = true)]
async fn should_abort_clear_with_nothing_reset_when_admission_is_held_past_the_budget() {
    let dir = tempfile::tempdir().unwrap();
    let host = BoundaryHost::new();
    let (_manager, wallet, coordinator) = configured(&host, &dir).await;
    wallet.bind_shielded(&SEED, &[0]).await.unwrap();
    let epoch = coordinator.clear_epoch().await;

    let in_flight = coordinator.admit().await.unwrap();
    let error = coordinator
        .clear(&*host)
        .await
        .expect_err("the admission is never released");
    drop(in_flight);

    assert!(
        matches!(error, PlatformWalletError::ShutdownIncomplete(_)),
        "{error:?}"
    );
    assert_eq!(host.wipe_count(), 0);
    assert_eq!(host.viewing_key_rows(), 1);
    assert_eq!(coordinator.clear_epoch().await, epoch);
    assert_eq!(
        coordinator.registered_subwallets().await,
        vec![SubwalletId::new(wallet.wallet_id(), 0)]
    );
}

/// Clear calls the host wipe exactly once, while it holds the admission
/// exclusively, and returns only after it.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn should_wipe_once_under_exclusive_admission() {
    let dir = tempfile::tempdir().unwrap();
    let host = BoundaryHost::new();
    let (manager, wallet, coordinator) = configured(&host, &dir).await;
    wallet.bind_shielded(&SEED, &[0]).await.unwrap();
    let epoch = coordinator.clear_epoch().await;

    manager.clear_shielded().await.unwrap();

    assert_eq!(host.wipe_count(), 1);
    assert!(host.wiped_exclusively.load(Ordering::SeqCst));
    assert!(!coordinator.admission_is_closed(), "released on return");
    assert_eq!(coordinator.clear_epoch().await, epoch + 1);
}

/// A host that cannot wipe makes Clear refuse before anything is reset.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn should_refuse_clear_without_the_wipe_capability() {
    let dir = tempfile::tempdir().unwrap();
    let host = BoundaryHost::with_capabilities(PersistenceCapabilities::SHIELDED_FVK_RESTART);
    let (manager, wallet, coordinator) = configured(&host, &dir).await;
    wallet.bind_shielded(&SEED, &[0]).await.unwrap();
    let epoch = coordinator.clear_epoch().await;

    let error = manager
        .clear_shielded()
        .await
        .expect_err("Clear fails closed");

    assert!(
        matches!(&error, PlatformWalletError::Persistence(message) if message.contains("nothing was reset")),
        "{error:?}"
    );
    assert_eq!(host.wipe_count(), 0);
    assert_eq!(host.viewing_key_rows(), 1);
    assert_eq!(coordinator.clear_epoch().await, epoch);
    assert_eq!(
        coordinator.registered_subwallets().await,
        vec![SubwalletId::new(wallet.wallet_id(), 0)]
    );
}

/// A wipe that fails after the native reset leaves binds refused until a
/// retried Clear wipes the host.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn should_refuse_binds_until_a_failed_host_wipe_is_retried() {
    let dir = tempfile::tempdir().unwrap();
    let host = BoundaryHost::new();
    let (manager, wallet, coordinator) = configured(&host, &dir).await;
    wallet.bind_shielded(&SEED, &[0]).await.unwrap();

    host.fail_wipe.store(true, Ordering::SeqCst);
    manager
        .clear_shielded()
        .await
        .expect_err("the host wipe failed");
    assert_eq!(host.viewing_key_rows(), 1, "the host kept its rows");
    let refused = wallet
        .bind_shielded_from_persisted(&[0])
        .await
        .expect_err("a bind would restore the unwiped rows");
    assert!(
        matches!(refused, PlatformWalletError::ShieldedStoreError(_)),
        "{refused:?}"
    );
    let refused = wallet
        .shielded_add_account(&SEED, 1)
        .await
        .expect_err("an account add is refused too");
    assert!(
        matches!(refused, PlatformWalletError::ShieldedStoreError(_)),
        "{refused:?}"
    );
    let refused = coordinator
        .restore_for_wallet(wallet.wallet_id(), &Default::default())
        .await
        .expect_err("so is a direct coordinator restore");
    assert!(
        matches!(refused, PlatformWalletError::ShieldedStoreError(_)),
        "{refused:?}"
    );

    host.fail_wipe.store(false, Ordering::SeqCst);
    manager.clear_shielded().await.expect("the retry wipes");
    assert_eq!(host.viewing_key_rows(), 0);
    wallet
        .bind_shielded(&SEED, &[0])
        .await
        .expect("binds again");
}

/// Whether `error` is the refusal of an admission while a Clear's host wipe
/// is pending.
fn refused_for_pending_wipe(error: &PlatformWalletError) -> bool {
    matches!(error, PlatformWalletError::ShieldedStoreError(message)
        if message.contains("did not wipe host storage"))
}

/// While a failed Clear's host wipe is pending, no shielded operation runs:
/// a mutation, the balance read and a restoration from the journal are
/// refused, not served from the reset store.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn should_refuse_mutations_and_balance_reads_while_the_host_wipe_is_pending() {
    let dir = tempfile::tempdir().unwrap();
    let host = BoundaryHost::new();
    let (manager, wallet, coordinator) = configured(&host, &dir).await;
    wallet.bind_shielded(&SEED, &[0]).await.unwrap();

    host.fail_wipe.store(true, Ordering::SeqCst);
    manager
        .clear_shielded()
        .await
        .expect_err("the host wipe failed");

    let refused = wallet
        .abandon_shielded_identity_debit(0, [0x33; 32], true)
        .await
        .expect_err("a mutation is refused");
    assert!(refused_for_pending_wipe(&refused), "{refused:?}");
    let refused = wallet
        .shielded_balances()
        .await
        .expect_err("the balance read is refused, not reported empty");
    assert!(refused_for_pending_wipe(&refused), "{refused:?}");
    let refused = coordinator
        .local_balance_snapshot(wallet.wallet_id())
        .await
        .expect_err("so is the local balance snapshot");
    assert!(refused_for_pending_wipe(&refused), "{refused:?}");
    let crypto = crate::wallet::identity::network::SeedCryptoProvider::from_seed(
        [0x17; 64],
        Network::Testnet,
    );
    let refused = wallet
        .restore_pending_private_contact_requests(&crypto)
        .await
        .expect_err("a restoration from the shielded store is refused");
    assert!(refused_for_pending_wipe(&refused), "{refused:?}");

    host.fail_wipe.store(false, Ordering::SeqCst);
    manager.clear_shielded().await.expect("the retry wipes");
    let balances = wallet.shielded_balances().await.expect("readable again");
    assert!(
        balances.values().all(|&balance| balance == 0),
        "{balances:?}"
    );
    wallet
        .restore_pending_private_contact_requests(&crypto)
        .await
        .expect("restorations run again");
}

/// A host wipe that failed is still pending after a restart: the store keeps
/// the mark, so a reopened coordinator refuses binds and mutations, and the
/// host's surviving rows are never restored, until a later Clear wipes them.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn should_keep_a_failed_host_wipe_pending_across_a_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let host = BoundaryHost::new();
    {
        let (manager, wallet, _coordinator) = configured(&host, &dir).await;
        wallet.bind_shielded(&SEED, &[0]).await.unwrap();
        host.fail_wipe.store(true, Ordering::SeqCst);
        manager
            .clear_shielded()
            .await
            .expect_err("the host wipe failed");
        let _ = manager.shutdown().await;
    }
    *host.coordinator.lock().unwrap() = None;
    host.fail_wipe.store(false, Ordering::SeqCst);
    assert_eq!(host.viewing_key_rows(), 1, "the host kept its rows");

    let (manager, wallet, coordinator) = configured(&host, &dir).await;
    let refused = wallet
        .bind_shielded_from_persisted(&[0])
        .await
        .expect_err("a reopened coordinator still refuses the bind");
    assert!(refused_for_pending_wipe(&refused), "{refused:?}");
    let refused = wallet
        .bind_shielded(&SEED, &[0])
        .await
        .expect_err("a seed bind too");
    assert!(refused_for_pending_wipe(&refused), "{refused:?}");
    let refused = wallet
        .abandon_shielded_identity_debit(0, [0x33; 32], true)
        .await
        .expect_err("and a mutation");
    assert!(refused_for_pending_wipe(&refused), "{refused:?}");
    assert!(coordinator.registered_subwallets().await.is_empty());

    manager.clear_shielded().await.expect("the retry wipes");
    assert_eq!(host.viewing_key_rows(), 0);
    wallet
        .bind_shielded(&SEED, &[0])
        .await
        .expect("binds again");
}

/// The manager owns the coordinator: its wallets resolve exactly the
/// manager's, and none before the manager configured one.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn should_resolve_the_managers_coordinator_and_none_before_it_is_configured() {
    let dir = tempfile::tempdir().unwrap();
    let host = BoundaryHost::new();
    let (manager, wallet) = unconfigured(&host).await;

    let error = wallet
        .bind_shielded(&SEED, &[0])
        .await
        .expect_err("no coordinator yet");
    assert!(
        matches!(&error, PlatformWalletError::ShieldedStoreError(message) if message.contains("not configured")),
        "{error:?}"
    );
    assert_eq!(host.viewing_key_rows(), 0, "refused before any host write");

    manager
        .configure_shielded(dir.path().join("tree.sqlite"))
        .await
        .unwrap();
    let owned = manager.shielded_coordinator().await.unwrap();
    assert!(Arc::ptr_eq(
        &wallet.shielded_coordinator().await.unwrap(),
        &owned
    ));
    let later = manager
        .create_wallet_from_seed_bytes(
            Network::Testnet,
            &[0x18; 64],
            WalletAccountCreationOptions::Default,
            Some(0),
        )
        .await
        .unwrap();
    assert!(Arc::ptr_eq(
        &later.shielded_coordinator().await.unwrap(),
        &owned
    ));
    wallet.bind_shielded(&SEED, &[0]).await.unwrap();
    assert_eq!(
        owned.registered_subwallets().await,
        vec![SubwalletId::new(wallet.wallet_id(), 0)]
    );
}

/// An unconfigured manager's wallet reports its shielded balance as an
/// error, not as an empty (zero) balance; once configured, an unbound wallet
/// reads empty.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn should_refuse_a_balance_read_before_the_manager_is_configured() {
    let dir = tempfile::tempdir().unwrap();
    let host = BoundaryHost::new();
    let (manager, wallet) = unconfigured(&host).await;

    let error = wallet
        .shielded_balances()
        .await
        .expect_err("no coordinator yet");
    assert!(
        matches!(&error, PlatformWalletError::ShieldedStoreError(message) if message.contains("not configured")),
        "{error:?}"
    );

    manager
        .configure_shielded(dir.path().join("tree.sqlite"))
        .await
        .unwrap();
    assert!(wallet.shielded_balances().await.unwrap().is_empty());
}

/// A private payment restoration parked after its journal read, waiting for
/// the wallet, holds Clear off: Clear neither resets nor wipes until the
/// restoration's last write is done.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn should_hold_clear_until_a_private_payment_restoration_in_flight_finishes() {
    use std::future::Future;
    use std::task::{Context, Waker};

    let dir = tempfile::tempdir().unwrap();
    let host = BoundaryHost::new();
    let (manager, wallet, coordinator) = configured(&host, &dir).await;
    let mut cx = Context::from_waker(Waker::noop());

    let held_store = coordinator.store().write().await;
    let mut restore = Box::pin(wallet.restore_private_contact_payments());
    assert!(
        restore.as_mut().poll(&mut cx).is_pending(),
        "waits for the store"
    );
    let held_wallet = wallet.wallet_manager().write().await;
    drop(held_store);
    assert!(
        restore.as_mut().poll(&mut cx).is_pending(),
        "read its journal, waits for the wallet"
    );

    let clear = spawn_clear(&manager);
    until_admission_closed(&coordinator).await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(host.wipe_count(), 0, "Clear waits for the restoration");
    assert!(!clear.is_finished());
    assert_eq!(coordinator.store().read().await.tree_size().unwrap(), 0);

    drop(held_wallet);
    restore.await.expect("the restoration finishes first");
    clear.await.unwrap().expect("then Clear runs");
    assert_eq!(host.wipe_count(), 1);
    assert!(host.wiped_exclusively.load(Ordering::SeqCst));
}

/// A restoration that starts while the host wipe runs waits for the whole
/// Clear, as a bind does, and then succeeds against the cleared store.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn should_hold_a_restoration_started_during_the_wipe_until_clear_finishes() {
    let dir = tempfile::tempdir().unwrap();
    let host = BoundaryHost::new();
    let (manager, wallet, coordinator) = configured(&host, &dir).await;

    type Restore = tokio::task::JoinHandle<
        Result<Vec<(dpp::prelude::Identifier, dpp::prelude::Identifier)>, PlatformWalletError>,
    >;
    let started: Arc<Mutex<Option<Restore>>> = Arc::default();
    let held_during_wipe = Arc::new(AtomicBool::new(false));
    {
        let wallet = Arc::clone(&wallet);
        let started = Arc::clone(&started);
        let held_during_wipe = Arc::clone(&held_during_wipe);
        let runtime = tokio::runtime::Handle::current();
        *host.on_wipe.lock().unwrap() = Some(Box::new(move || {
            let wallet = Arc::clone(&wallet);
            let restore =
                runtime.spawn(async move { wallet.restore_private_contact_payments().await });
            std::thread::sleep(Duration::from_millis(100));
            held_during_wipe.store(!restore.is_finished(), Ordering::SeqCst);
            *started.lock().unwrap() = Some(restore);
        }));
    }

    manager.clear_shielded().await.expect("Clear succeeds");
    assert!(held_during_wipe.load(Ordering::SeqCst));
    let restore = started
        .lock()
        .unwrap()
        .take()
        .expect("the hook started a restoration");
    restore
        .await
        .unwrap()
        .expect("the restoration runs after Clear");
    assert_eq!(coordinator.clear_epoch().await, 1);
    assert!(!wallet.private_payments_held().await);
}

/// A Clear that reset native state but failed to wipe the host refuses
/// restorations, holding the wallet, until a Clear completes both halves.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn should_refuse_restorations_until_a_failed_host_wipe_is_retried() {
    let dir = tempfile::tempdir().unwrap();
    let host = BoundaryHost::new();
    let (manager, wallet, _coordinator) = configured(&host, &dir).await;

    host.fail_wipe.store(true, Ordering::SeqCst);
    manager
        .clear_shielded()
        .await
        .expect_err("the host wipe fails");
    let error = wallet
        .restore_private_contact_payments()
        .await
        .expect_err("refused while the host may hold cleared rows");
    assert!(
        matches!(&error, PlatformWalletError::ShieldedStoreError(message) if message.contains("did not wipe host storage")),
        "{error:?}"
    );
    assert!(
        wallet.private_payments_held().await,
        "the wallet stays held"
    );

    host.fail_wipe.store(false, Ordering::SeqCst);
    manager
        .clear_shielded()
        .await
        .expect("the retry wipes the host");
    wallet
        .restore_private_contact_payments()
        .await
        .expect("restored once the host is wiped");
    assert!(!wallet.private_payments_held().await);
}

/// A bind parked inside its admission while a Clear queues for the lock runs
/// its private payment restoration under that same admission: it neither
/// deadlocks on a second admission behind the queued Clear nor defers the
/// restoration, and Clear runs once the bind is done.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn should_restore_a_binds_private_payments_under_its_admission_while_clear_is_queued() {
    let dir = tempfile::tempdir().unwrap();
    let host = BoundaryHost::new();
    let (manager, wallet, coordinator) = configured(&host, &dir).await;

    let release = BoundaryHost::gate(&host.store_gate);
    let bind = {
        let wallet = Arc::clone(&wallet);
        tokio::spawn(async move { wallet.bind_shielded(&SEED, &[0]).await })
    };
    host.reached("store viewing keys").await;
    let clear = spawn_clear(&manager);
    until_admission_closed(&coordinator).await;

    release.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(10), bind)
        .await
        .expect("the bind does not deadlock behind the queued Clear")
        .unwrap()
        .expect("the bind finishes first");
    assert_eq!(
        wallet.deferred_private_payment_restores(),
        0,
        "restored inline, not deferred"
    );
    assert!(!wallet.private_payments_held().await);
    clear.await.unwrap().expect("then Clear runs");
    assert_eq!(host.events(), BIND_THEN_CLEAR);
}

/// A funding's confirmed activity row, as the recorder builds it.
fn funding_activity() -> crate::wallet::shielded::activity::ShieldedActivityEntry {
    use crate::wallet::shielded::activity::{
        ShieldedActivityEntry, ShieldedActivityKind, ShieldedActivityStatus, ShieldedDirection,
    };
    ShieldedActivityEntry {
        id: [0x5A; 32],
        kind: ShieldedActivityKind::ShieldFromAssetLock,
        direction: ShieldedDirection::In,
        amount: 1_000,
        fee: None,
        counterparty: None,
        memo: None,
        block_height: None,
        status: ShieldedActivityStatus::Pending,
        created_at_ms: 1,
        min_note_position: None,
        note_cmxs: vec![[0x2A; 32]],
        spent_nullifiers: Vec::new(),
    }
}

/// A funding drops its admission across the broadcast and the ChainLock
/// wait, and is admitted again for its activity write. A Clear completing
/// in between has reset the state that row belongs to, so the write is
/// skipped: no activity row reaches the store or the host. Without a Clear
/// the same write lands.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn should_record_no_funding_activity_after_a_clear_that_followed_its_broadcast() {
    let dir = tempfile::tempdir().unwrap();
    let host = BoundaryHost::new();
    let (manager, wallet, coordinator) = configured(&host, &dir).await;
    wallet.bind_shielded(&SEED, &[0]).await.unwrap();
    let id = SubwalletId::new(wallet.wallet_id(), 0);
    let entry = funding_activity();

    // Admitted before the broadcast, then released, as a funding does.
    let funding_epoch = coordinator.admit().await.unwrap().epoch();
    manager.clear_shielded().await.expect("Clear succeeds");
    wallet
        .record_funding_activity_since(funding_epoch, 0, entry.clone())
        .await;
    wallet.persister().flush().unwrap();

    assert!(
        coordinator
            .store()
            .read()
            .await
            .get_activity_by_entry_id(id, &entry.id)
            .unwrap()
            .is_none(),
        "a pre-Clear funding's activity must not land in the post-Clear store"
    );
    assert_eq!(host.activity_rows.load(Ordering::SeqCst), 0);

    // A funding admitted after the Clear records as usual.
    let funding_epoch = coordinator.admit().await.unwrap().epoch();
    wallet
        .record_funding_activity_since(funding_epoch, 0, entry.clone())
        .await;
    wallet.persister().flush().unwrap();
    assert!(coordinator
        .store()
        .read()
        .await
        .get_activity_by_entry_id(id, &entry.id)
        .unwrap()
        .is_some());
    assert_eq!(host.activity_rows.load(Ordering::SeqCst), 1);
}

/// Wallet removal runs inside the admission protocol. A shield-class
/// operation holds the wallet's `shield_guard` across a wait that holds no
/// admission (a funding waiting on its ChainLock); the removal waits for
/// it, so that operation's last write lands before the removal returns and
/// the host's deletion of the wallet's rows removes it. A retained handle's
/// write after the removal is refused, so no row outlives it.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn should_leave_no_funding_activity_row_after_a_removal_returns() {
    let dir = tempfile::tempdir().unwrap();
    let host = BoundaryHost::new();
    let (manager, wallet, coordinator) = configured(&host, &dir).await;
    wallet.bind_shielded(&SEED, &[0]).await.unwrap();
    let id = SubwalletId::new(wallet.wallet_id(), 0);
    let entry = funding_activity();

    // A funding admitted before its broadcast, now waiting on its ChainLock.
    let funding_epoch = coordinator.admit().await.unwrap().epoch();
    let funding = Arc::clone(&wallet.shield_guard).lock_owned().await;
    let removal = {
        let manager = Arc::clone(&manager);
        let wallet_id = wallet.wallet_id();
        tokio::spawn(async move { manager.remove_wallet(&wallet_id).await.map(drop) })
    };
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(
        !removal.is_finished(),
        "the removal waits for the shield-class operation in flight"
    );
    wallet
        .record_funding_activity_since(funding_epoch, 0, entry.clone())
        .await;
    wallet.persister().flush().unwrap();
    assert_eq!(host.activity_rows.load(Ordering::SeqCst), 1);
    drop(funding);
    removal.await.unwrap().expect("the removal completes");

    // The host deletes the wallet's rows now. A funding on a retained
    // handle that settles afterwards writes nothing, native or host.
    let rows_at_removal = host.activity_rows.load(Ordering::SeqCst);
    let late_epoch = coordinator.admit().await.unwrap().epoch();
    let late = Arc::clone(&wallet.shield_guard).lock_owned().await;
    let mut late_entry = funding_activity();
    late_entry.id = [0x5B; 32];
    wallet
        .record_funding_activity_since(late_epoch, 0, late_entry.clone())
        .await;
    drop(late);
    let _ = wallet.persister().flush();
    assert_eq!(
        host.activity_rows.load(Ordering::SeqCst),
        rows_at_removal,
        "no activity row reaches the host after the removal returned"
    );
    assert!(coordinator
        .store()
        .read()
        .await
        .get_activity_by_entry_id(id, &late_entry.id)
        .unwrap()
        .is_none());
}

/// A bind parked in its viewing-key write holds a removal of its wallet
/// off, through the generation gate, `shield_guard` and the admission the
/// two take in the same order; released, both complete, and a bind on the
/// retained handle afterwards writes nothing.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn should_order_a_bind_and_a_removal_of_its_wallet() {
    let dir = tempfile::tempdir().unwrap();
    let host = BoundaryHost::new();
    let (manager, wallet, coordinator) = configured(&host, &dir).await;

    let release = BoundaryHost::gate(&host.store_gate);
    let bind = {
        let wallet = Arc::clone(&wallet);
        tokio::spawn(async move { wallet.bind_shielded(&SEED, &[0]).await })
    };
    host.reached("store viewing keys").await;
    let removal = {
        let manager = Arc::clone(&manager);
        let wallet_id = wallet.wallet_id();
        tokio::spawn(async move { manager.remove_wallet(&wallet_id).await.map(drop) })
    };
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(!removal.is_finished(), "the removal waits for the bind");

    release.send(()).unwrap();
    bind.await.unwrap().expect("the bind finishes first");
    removal.await.unwrap().expect("then the removal");
    assert!(coordinator.registered_subwallets().await.is_empty());

    let rows = host.viewing_key_rows();
    let refused = wallet.bind_shielded(&SEED, &[0]).await;
    assert!(
        matches!(refused, Err(PlatformWalletError::WalletNotFound(_))),
        "{refused:?}"
    );
    let refused = wallet.shielded_add_account(&SEED, 1).await;
    assert!(
        matches!(refused, Err(PlatformWalletError::WalletNotFound(_))),
        "{refused:?}"
    );
    assert_eq!(
        host.viewing_key_rows(),
        rows,
        "nothing written after removal"
    );
}

/// A pending host wipe refuses shielded writes, not a wallet's removal,
/// which only deletes: a user can still delete a wallet while a failed
/// Clear waits for its retry.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn should_remove_a_wallet_while_a_host_wipe_is_pending() {
    let dir = tempfile::tempdir().unwrap();
    let host = BoundaryHost::new();
    let (manager, wallet, coordinator) = configured(&host, &dir).await;
    wallet.bind_shielded(&SEED, &[0]).await.unwrap();
    host.fail_wipe.store(true, Ordering::SeqCst);
    manager
        .clear_shielded()
        .await
        .expect_err("the host wipe fails");
    assert!(coordinator.admit().await.is_err(), "the wipe is pending");

    manager
        .remove_wallet(&wallet.wallet_id())
        .await
        .expect("the removal is not refused");
    assert!(manager.get_wallet(&wallet.wallet_id()).await.is_none());
}

/// A handle kept past its wallet's removal spends nothing: every shielded
/// spend checks the wallet is still attached under the generation gate the
/// removal takes exclusively, and is refused before it derives a key or
/// selects a note.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn should_refuse_every_shielded_spend_on_a_handle_kept_past_its_removal() {
    let dir = tempfile::tempdir().unwrap();
    let host = BoundaryHost::new();
    let (manager, wallet, _coordinator) = configured(&host, &dir).await;
    wallet.bind_shielded(&SEED, &[0]).await.unwrap();
    manager
        .remove_wallet(&wallet.wallet_id())
        .await
        .expect("the removal completes");

    let prover = super::prover::CachedOrchardProver::new();
    let removed = |result: Result<(), PlatformWalletError>| {
        assert!(
            matches!(result, Err(PlatformWalletError::WalletNotFound(_))),
            "{result:?}"
        );
    };
    removed(
        wallet
            .shielded_transfer_to(&SEED, 0, &[0; 43], 1, [0; 36], &prover)
            .await,
    );
    removed(
        wallet
            .shielded_unshield_to(&SEED, 0, "tdash1invalid", 1, &prover)
            .await,
    );
    removed(
        wallet
            .shielded_withdraw_to(&SEED, 0, "yinvalid", 1, 1, &prover)
            .await,
    );
    removed(
        wallet
            .shielded_identity_top_up_from_pool(&SEED, 0, &[7; 32].into(), 1, &prover)
            .await
            .map(drop),
    );
    removed(
        wallet
            .abandon_shielded_identity_debit(0, [0; 32], true)
            .await,
    );
    removed(wallet.bind_shielded_from_persisted(&[0]).await.map(drop));
}
