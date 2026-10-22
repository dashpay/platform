//! Identity keys across a restart: a restore whose key write the disk
//! refuses, and keys a refresh finds.
//!
//! Every test drives the real `PlatformWalletManager` + `IdentityWallet`
//! over a mock SDK and the real SQLite persister (Immediate mode, the
//! default and what dash-evo-tool runs). A disk-full failure is the genuine
//! rusqlite `SQLITE_FULL` error primed into the persister's own flush seam
//! (`force_next_flush_to_fail`), so classification, buffer restore and the
//! `FlushRetryable` surface are the production code paths; only the moment
//! the disk fills is chosen by the test. A "restart" closes the SQLite file,
//! reopens it and rebuilds a fresh manager through `load_from_persistor`.
//!

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use dash_sdk::platform::types::identity::PublicKeyHash;
use dpp::identity::accessors::{IdentityGettersV0, IdentitySettersV0};
use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::v0::IdentityV0;
use dpp::identity::{Identity, IdentityPublicKey, KeyID, KeyType, Purpose, SecurityLevel};
use dpp::platform_value::BinaryData;
use dpp::prelude::Identifier;
use key_wallet::bip32::ExtendedPrivKey;
use key_wallet::mnemonic::Mnemonic;
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::Network;
use platform_wallet::changeset::{
    ClientStartState, IdentityKeyEntry, IdentityScanStateEntry, PersistenceCapabilities,
    PersistenceError, PersistenceErrorKind, PlatformWalletChangeSet, PlatformWalletPersistence,
};
use platform_wallet::wallet::identity::network::{
    derive_ecdsa_identity_auth_keypair_from_master, derive_identity_auth_key_hash_from_master,
    IdentityDiscoveryOptions,
};
use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet::{PlatformEventHandler, PlatformWalletManager};
use platform_wallet_storage::sqlite::schema::{identity_keys, identity_scan_states};
use platform_wallet_storage::{
    LoadCtx, SqlitePersister, SqlitePersisterConfig, WalletStorageError,
};

const MNEMONIC_A: &str = "abandon abandon abandon abandon abandon abandon \
     abandon abandon abandon abandon abandon about";
const NET: Network = Network::Testnet;

// ---------------------------------------------------------------------------
// Persister: real SQLite, with a disk that can fill
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum DiskFull {
    /// The disk never fills.
    Never,
    /// The disk fills on the first write that carries a key breadcrumb and
    /// stays full until the process ends.
    FromBreadcrumbWriteOn,
}

struct Disk {
    inner: Mutex<Option<SqlitePersister>>,
    mode: DiskFull,
    tripped: AtomicBool,
    full: AtomicBool,
    /// Every error `store`/`flush` returned, with its kind.
    failures: Mutex<Vec<Option<PersistenceErrorKind>>>,
}

fn sqlite_full() -> WalletStorageError {
    WalletStorageError::Sqlite(rusqlite::Error::SqliteFailure(
        rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_FULL),
        Some("database or disk is full".into()),
    ))
}

impl Disk {
    fn open(path: &std::path::Path, mode: DiskFull) -> Arc<Self> {
        let p = SqlitePersister::open(SqlitePersisterConfig::new(path)).expect("open persister");
        Arc::new(Self {
            inner: Mutex::new(Some(p)),
            mode,
            tripped: AtomicBool::new(false),
            full: AtomicBool::new(false),
            failures: Mutex::new(Vec::new()),
        })
    }

    /// Process death: the SQLite handle closes; anything only buffered is gone.
    fn close(&self) {
        self.inner.lock().unwrap().take();
    }

    fn with<R>(&self, f: impl FnOnce(&SqlitePersister) -> R) -> R {
        let guard = self.inner.lock().unwrap();
        f(guard.as_ref().expect("persister closed"))
    }

    fn before_write(&self, p: &SqlitePersister) {
        if self.full.load(Ordering::SeqCst) {
            p.force_next_flush_to_fail(sqlite_full());
        }
    }

    fn record<T>(&self, r: Result<T, PersistenceError>) -> Result<T, PersistenceError> {
        if let Err(e) = &r {
            self.failures.lock().unwrap().push(e.kind());
        }
        r
    }

    fn key_rows(&self, w: &WalletId, id: &Identifier) -> BTreeMap<KeyID, IdentityKeyEntry> {
        self.with(|p| {
            let conn = p.lock_conn_for_test();
            identity_keys::load_state(&conn, w, &LoadCtx::strict())
                .expect("read identity_keys")
                .upserts
                .into_iter()
                .filter(|((owner, _), _)| owner == id)
                .map(|((_, key_id), e)| (key_id, e))
                .collect()
        })
    }

    fn scan_verdict(&self, w: &WalletId) -> Option<IdentityScanStateEntry> {
        self.with(|p| {
            let conn = p.lock_conn_for_test();
            identity_scan_states::load_for_wallet(&conn, w, &LoadCtx::strict())
                .expect("read identity_scan_states")
        })
    }
}

fn closed() -> PersistenceError {
    PersistenceError::backend("persister closed (process ended)")
}

impl PlatformWalletPersistence for Disk {
    fn store_commits_inline(&self) -> bool {
        self.with(|p| p.store_commits_inline())
    }
    fn persistence_capabilities(&self) -> PersistenceCapabilities {
        self.with(|p| p.persistence_capabilities())
    }
    fn store_transient_is_reissuable(&self) -> bool {
        self.with(|p| p.store_transient_is_reissuable())
    }
    fn stores_dashpay_backfill(&self) -> bool {
        self.with(|p| p.stores_dashpay_backfill())
    }
    fn store(&self, w: WalletId, cs: PlatformWalletChangeSet) -> Result<(), PersistenceError> {
        let carries_breadcrumb = cs
            .identity_keys
            .as_ref()
            .is_some_and(|k| k.upserts.values().any(|e| e.derivation_indices.is_some()));
        if self.mode != DiskFull::Never
            && carries_breadcrumb
            && !self.tripped.swap(true, Ordering::SeqCst)
        {
            self.full.store(true, Ordering::SeqCst);
        }
        let guard = self.inner.lock().unwrap();
        let Some(p) = guard.as_ref() else {
            return Err(closed());
        };
        self.before_write(p);
        let r = p.store(w, cs);
        drop(guard);
        self.record(r)
    }
    fn flush(&self, w: WalletId) -> Result<(), PersistenceError> {
        let guard = self.inner.lock().unwrap();
        let Some(p) = guard.as_ref() else {
            return Err(closed());
        };
        self.before_write(p);
        let r = p.flush(w);
        drop(guard);
        self.record(r)
    }
    fn load(&self) -> Result<ClientStartState, PersistenceError> {
        self.with(|p| p.load())
    }
    fn persist_tracked_masternodes(
        &self,
        network: dashcore::Network,
        records: &[platform_wallet::masternode::TrackedMasternode],
    ) -> Result<(), PersistenceError> {
        self.with(|p| p.persist_tracked_masternodes(network, records))
    }
    fn load_tracked_masternodes(
        &self,
        network: dashcore::Network,
    ) -> Result<Vec<platform_wallet::masternode::TrackedMasternode>, PersistenceError> {
        self.with(|p| p.load_tracked_masternodes(network))
    }
}

// ---------------------------------------------------------------------------
// Wallet, identity and Platform fixtures
// ---------------------------------------------------------------------------

struct Noop;
impl platform_wallet::events::EventHandler for Noop {}
impl PlatformEventHandler for Noop {}

fn seed(phrase: &str) -> [u8; 64] {
    Mnemonic::from_phrase(phrase).unwrap().to_seed("")
}

fn master(phrase: &str) -> ExtendedPrivKey {
    ExtendedPrivKey::new_master(NET, &seed(phrase)).unwrap()
}

/// The DIP-9 ECDSA auth key a wallet derives at `(identity_index, key_index)`.
fn derived_key(
    phrase: &str,
    identity_index: u32,
    key_index: u32,
    id: KeyID,
    level: SecurityLevel,
) -> IdentityPublicKey {
    let kp = derive_ecdsa_identity_auth_keypair_from_master(
        &master(phrase),
        NET,
        identity_index,
        key_index,
    )
    .unwrap();
    IdentityPublicKey::V0(IdentityPublicKeyV0 {
        id,
        purpose: Purpose::AUTHENTICATION,
        security_level: level,
        contract_bounds: None,
        key_type: KeyType::ECDSA_SECP256K1,
        read_only: false,
        data: BinaryData::new(kp.public_key.to_vec()),
        disabled_at: None,
    })
}

/// Identity registered by wallet A at index 0 the way registration does it:
/// MASTER, HIGH and CRITICAL auth keys at key ids 0..=2, all from the seed.
fn registered_identity() -> Identity {
    let levels = [
        SecurityLevel::MASTER,
        SecurityLevel::HIGH,
        SecurityLevel::CRITICAL,
    ];
    let public_keys = levels
        .iter()
        .enumerate()
        .map(|(i, level)| {
            let i = i as u32;
            (i, derived_key(MNEMONIC_A, 0, i, i, *level))
        })
        .collect();
    Identity::V0(IdentityV0 {
        id: Identifier::from([0x5A; 32]),
        public_keys,
        balance: 1_000_000,
        revision: 1,
    })
}

fn probe(phrase: &str, index: u32) -> PublicKeyHash {
    PublicKeyHash(
        derive_identity_auth_key_hash_from_master(&master(phrase), NET, index, 0).unwrap(),
    )
}

/// Platform: `phrase`'s index 0 answers `at_0`, index 1 holds nothing, and a
/// fetch by id answers `by_id`.
async fn platform(
    phrase: &str,
    at_0: Option<Identity>,
    by_id: Option<Identity>,
) -> Arc<dash_sdk::Sdk> {
    let mut sdk = dash_sdk::SdkBuilder::new_mock()
        .with_network(NET)
        .build()
        .unwrap();
    sdk.mock()
        .expect_fetch(probe(phrase, 0), at_0)
        .await
        .unwrap();
    sdk.mock()
        .expect_fetch(probe(phrase, 1), None::<Identity>)
        .await
        .unwrap();
    if let Some(identity) = by_id {
        sdk.mock()
            .expect_fetch(identity.id(), Some(identity))
            .await
            .unwrap();
    }
    Arc::new(sdk)
}

type Manager = Arc<PlatformWalletManager<Disk>>;

/// First launch: a fresh install restores the wallet from its mnemonic.
async fn first_launch(
    sdk: Arc<dash_sdk::Sdk>,
    disk: Arc<Disk>,
    phrase: &str,
) -> (Manager, WalletId) {
    let manager = Arc::new(PlatformWalletManager::new(sdk, disk, Arc::new(Noop)));
    let wallet = manager
        .create_wallet_from_seed_bytes(
            NET,
            &seed(phrase),
            WalletAccountCreationOptions::Default,
            Some(0),
        )
        .await
        .expect("create wallet");
    let id = wallet.wallet_id();
    (manager, id)
}

/// A later launch: rebuild from what SQLite holds (watch-only wallet, the
/// iOS shape — signing keys come from the Keychain seed via the breadcrumb).
async fn relaunch(sdk: Arc<dash_sdk::Sdk>, disk: Arc<Disk>) -> Manager {
    let manager = Arc::new(PlatformWalletManager::new(sdk, disk, Arc::new(Noop)));
    manager
        .load_from_persistor()
        .await
        .expect("load from SQLite");
    manager
}

async fn end_process(manager: Manager, disk: &Disk) {
    // No `shutdown()`: it needs a multi-thread runtime. Dropping the manager
    // and closing the SQLite handle under it is the process dying.
    drop(manager);
    disk.close();
}

/// Startup's identity step (`start_wallet_subsystems`, startup.rs ~543): a
/// local identity on file and no recorded incomplete scan takes the warm
/// shortcut — no discovery runs this launch.
async fn startup_takes_warm_shortcut(manager: &Manager, w: &WalletId) -> bool {
    let wm = manager.wallet_manager_arc();
    let wm = wm.read().await;
    let info = wm.get_wallet_info(w).expect("wallet info");
    let known = !info.identity_manager.wallet_identity_ids(w).is_empty();
    known && !info.identity_manager.identity_scan_is_incomplete(w)
}

async fn held_key_ids(manager: &Manager, w: &WalletId, id: &Identifier) -> Vec<KeyID> {
    let wm = manager.wallet_manager_arc();
    let wm = wm.read().await;
    wm.get_wallet_info(w)
        .expect("wallet info")
        .identity_manager
        .identity(id)
        .expect("identity in memory")
        .identity
        .public_keys()
        .keys()
        .copied()
        .collect()
}

fn breadcrumbed(rows: &BTreeMap<KeyID, IdentityKeyEntry>) -> Vec<KeyID> {
    rows.iter()
        .filter(|(_, e)| e.derivation_indices.is_some())
        .map(|(k, _)| *k)
        .collect()
}

fn scan_from_zero() -> IdentityDiscoveryOptions {
    // The options startup's discovery uses (startup.rs ~835).
    IdentityDiscoveryOptions {
        start_index: Some(0),
        gap_limit: 1,
    }
}

// ---------------------------------------------------------------------------
// A key-row store fails during a restore
// ---------------------------------------------------------------------------

/// A restore runs discovery, and the disk fills on the write that carries
/// the key breadcrumbs and stays full for the rest of the session (the scan
/// verdict cannot land either). The new identity and its keys go to the store
/// as one changeset, so nothing of it lands: after a restart no key row is
/// left without its breadcrumb, startup does not take the warm shortcut (no
/// identity is on file), and its discovery restores every breadcrumb.
#[tokio::test]
async fn should_not_leave_watch_only_key_rows_when_the_disk_fills_during_a_restore() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("wallet.db");
    let identity = registered_identity();
    let id = identity.id();

    // Launch 1.
    let disk = Disk::open(&path, DiskFull::FromBreadcrumbWriteOn);
    let (manager, w) = first_launch(
        platform(MNEMONIC_A, Some(identity.clone()), None).await,
        disk.clone(),
        MNEMONIC_A,
    )
    .await;
    let err = manager
        .get_wallet(&w)
        .await
        .unwrap()
        .identity()
        .discover_from_master(scan_from_zero(), &master(MNEMONIC_A))
        .await
        .expect_err("the key write fails");
    assert!(
        err.to_string()
            .contains("identity keys not persisted during discovery"),
        "{err}"
    );
    assert!(
        disk.failures
            .lock()
            .unwrap()
            .iter()
            .all(|k| *k == Some(PersistenceErrorKind::Transient)),
        "disk full reaches the wallet as Transient"
    );
    // In memory the session is fine: all three keys are held.
    assert_eq!(held_key_ids(&manager, &w, &id).await, vec![0, 1, 2]);
    end_process(manager, &disk).await;

    // Launch 2, disk has space again.
    let disk = Disk::open(&path, DiskFull::Never);
    let rows = disk.key_rows(&w, &id);
    assert!(
        rows.values().all(|row| row.derivation_indices.is_some()),
        "no key row is left watch-only: {rows:?}"
    );
    assert!(disk.scan_verdict(&w).is_none(), "the verdict never landed");

    let manager = relaunch(
        platform(MNEMONIC_A, Some(identity.clone()), None).await,
        disk.clone(),
    )
    .await;
    assert!(
        !startup_takes_warm_shortcut(&manager, &w).await,
        "no identity is on file, so startup runs discovery"
    );
    manager
        .get_wallet(&w)
        .await
        .unwrap()
        .identity()
        .discover_from_master(scan_from_zero(), &master(MNEMONIC_A))
        .await
        .expect("startup's discovery");
    assert_eq!(
        breadcrumbed(&disk.key_rows(&w, &id)),
        vec![0, 1, 2],
        "the startup discovery restores every breadcrumb"
    );
    end_process(manager, &disk).await;
}

// ---------------------------------------------------------------------------
// Keys a refresh finds (added from another device)
// ---------------------------------------------------------------------------

/// The identity after the user's other device (same seed) added a MEDIUM key
/// at key id 3 — derived, as that device's add-key flow does, at (0, 3).
fn with_key_from_other_device(identity: &Identity) -> Identity {
    let mut updated = identity.clone();
    updated.add_public_key(derived_key(MNEMONIC_A, 0, 3, 3, SecurityLevel::MEDIUM));
    updated.set_revision(2);
    updated
}

/// A refresh that finds a key the user's other device added stores its row
/// (without a breadcrumb, which leaves the stored ones as they are), so after
/// a restart the identity holds the key set its stored revision claims.
#[tokio::test]
async fn should_store_a_key_a_refresh_finds_so_it_survives_a_restart() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("wallet.db");
    let identity = registered_identity();
    let id = identity.id();
    let updated = with_key_from_other_device(&identity);

    let disk = Disk::open(&path, DiskFull::Never);
    let (manager, w) = first_launch(
        platform(MNEMONIC_A, Some(identity.clone()), Some(updated)).await,
        disk.clone(),
        MNEMONIC_A,
    )
    .await;
    let iw = manager.get_wallet(&w).await.unwrap();
    iw.identity()
        .discover_from_master(scan_from_zero(), &master(MNEMONIC_A))
        .await
        .expect("restore");
    iw.identity().refresh_identity(&id).await.expect("refresh");
    assert_eq!(held_key_ids(&manager, &w, &id).await, vec![0, 1, 2, 3]);
    let rows = disk.key_rows(&w, &id);
    assert!(rows.contains_key(&3), "the refreshed key has a row");
    assert_eq!(
        breadcrumbed(&rows),
        vec![0, 1, 2],
        "the restore's breadcrumbs stay"
    );
    drop(iw);
    end_process(manager, &disk).await;

    let disk = Disk::open(&path, DiskFull::Never);
    let manager = relaunch(
        platform(MNEMONIC_A, Some(identity), None).await,
        disk.clone(),
    )
    .await;
    assert_eq!(
        held_key_ids(&manager, &w, &id).await,
        vec![0, 1, 2, 3],
        "key 3 is there after restart"
    );
    let wm = manager.wallet_manager_arc();
    let revision = wm
        .read()
        .await
        .get_wallet_info(&w)
        .unwrap()
        .identity_manager
        .identity(&id)
        .unwrap()
        .identity
        .revision();
    assert_eq!(revision, 2);
    end_process(manager, &disk).await;
}

/// A load at the identity's index (the host's "load identity at index")
/// re-fetches the identity and stores every on-chain key with a breadcrumb
/// when it derives from the seed, a key another device added included.
#[tokio::test]
async fn should_store_a_key_a_load_at_index_finds_with_its_breadcrumb() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("wallet.db");
    let identity = registered_identity();
    let id = identity.id();
    let updated = with_key_from_other_device(&identity);

    let disk = Disk::open(&path, DiskFull::Never);
    let (manager, w) = first_launch(
        platform(MNEMONIC_A, Some(identity.clone()), None).await,
        disk.clone(),
        MNEMONIC_A,
    )
    .await;
    manager
        .get_wallet(&w)
        .await
        .unwrap()
        .identity()
        .discover_from_master(scan_from_zero(), &master(MNEMONIC_A))
        .await
        .expect("restore");
    end_process(manager, &disk).await;

    // Next launch: Platform now holds the other device's key 3.
    let disk = Disk::open(&path, DiskFull::Never);
    let manager = relaunch(
        platform(MNEMONIC_A, Some(updated.clone()), None).await,
        disk.clone(),
    )
    .await;
    manager
        .get_wallet(&w)
        .await
        .unwrap()
        .identity()
        .load_identity_by_index_from_master(0, &master(MNEMONIC_A))
        .await
        .expect("load at index 0")
        .expect("found");
    assert_eq!(breadcrumbed(&disk.key_rows(&w, &id)), vec![0, 1, 2, 3]);
    end_process(manager, &disk).await;

    let disk = Disk::open(&path, DiskFull::Never);
    let manager = relaunch(
        platform(MNEMONIC_A, Some(updated), None).await,
        disk.clone(),
    )
    .await;
    assert_eq!(held_key_ids(&manager, &w, &id).await, vec![0, 1, 2, 3]);
    end_process(manager, &disk).await;
}
