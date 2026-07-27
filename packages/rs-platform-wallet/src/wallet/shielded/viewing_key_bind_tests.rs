//! Round-trip: bind → persist → simulated restart → seedless rebind.
//!
//! Pins the launch-path contract behind the viewing-key persistence
//! seam: the first (seed-backed) `bind_shielded` queues each account's
//! raw 96-byte FVK through the persister, and a later launch rebinds
//! via `bind_shielded_from_persisted` from those rows alone — no seed
//! parameter exists on that path, so a mnemonic resolve is impossible
//! by construction. The rebind must reproduce the exact same
//! viewing-grade material (FVK / IVK / OVK / default address) or the
//! restarted wallet would trial-decrypt someone else's notes.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use dashcore::Network;
use key_wallet::account::StandardAccountType;

use crate::changeset::PersistenceError;
use crate::changeset::PlatformWalletPersistence;
use crate::changeset::{ClientStartState, PlatformWalletChangeSet, ShieldedSubwalletStartState};
use crate::test_support::funded_wallet_manager;
use crate::wallet::platform_wallet::{PlatformWallet, WalletId};
use crate::wallet::shielded::store::ShieldedStore;
use crate::wallet::shielded::{FileBackedShieldedStore, NetworkShieldedCoordinator, SubwalletId};

/// Persister double for both halves of the round trip:
/// - `store` captures every queued changeset (the persist half);
/// - `load` serves whatever viewing-key rows the test staged via
///   [`Self::serve_viewing_keys`] (the restart half). Load calls are
///   counted so the test can assert the seedless path actually read
///   persistence rather than short-circuiting.
#[derive(Default)]
struct CapturingPersistence {
    stored: Mutex<Vec<PlatformWalletChangeSet>>,
    serve: Mutex<BTreeMap<SubwalletId, Vec<u8>>>,
    serve_subwallets: Mutex<BTreeMap<SubwalletId, ShieldedSubwalletStartState>>,
    load_calls: Mutex<usize>,
    /// Blocks the next `load()` until the test releases it, so a test can
    /// hold a bind at the point where it has read the host snapshot but
    /// has not yet opened its install transaction. A real host callback
    /// blocks its calling thread the same way.
    load_gate: Mutex<Option<std::sync::mpsc::Receiver<()>>>,
    /// Incremented on ENTRY to `load()`, before the gate blocks — the
    /// handshake a test waits on to know a bind has actually reached the
    /// gate rather than merely having been spawned.
    load_entries: Mutex<usize>,
}

impl CapturingPersistence {
    /// Every viewing-key row captured across all stored changesets.
    fn captured_viewing_keys(&self) -> BTreeMap<SubwalletId, Vec<u8>> {
        let mut out = BTreeMap::new();
        for cs in self.stored.lock().expect("stored lock").iter() {
            if let Some(shielded) = &cs.shielded {
                for (id, fvk) in &shielded.viewing_keys {
                    out.insert(*id, fvk.clone());
                }
            }
        }
        out
    }

    /// Stage the rows `load()` hands back — the "durable storage" the
    /// restarted session finds.
    fn serve_viewing_keys(&self, rows: BTreeMap<SubwalletId, Vec<u8>>) {
        *self.serve.lock().expect("serve lock") = rows;
    }

    /// Stage the per-subwallet snapshot `load()` hands back, so a bind's
    /// restore has something to apply (and therefore reaches the store).
    fn serve_subwallets(&self, rows: BTreeMap<SubwalletId, ShieldedSubwalletStartState>) {
        *self.serve_subwallets.lock().expect("serve_subwallets lock") = rows;
    }

    fn load_calls(&self) -> usize {
        *self.load_calls.lock().expect("load_calls lock")
    }

    /// Number of changesets queued so far — lets a test assert that a
    /// refused bind wrote nothing to durable storage.
    fn stored_count(&self) -> usize {
        self.stored.lock().expect("stored lock").len()
    }

    /// Make the next `load()` block until the returned sender fires.
    fn gate_next_load(&self) -> std::sync::mpsc::Sender<()> {
        let (tx, rx) = std::sync::mpsc::channel();
        *self.load_gate.lock().expect("load_gate lock") = Some(rx);
        tx
    }

    /// How many `load()` calls have STARTED (gated ones included).
    fn load_entries(&self) -> usize {
        *self.load_entries.lock().expect("load_entries lock")
    }
}

impl PlatformWalletPersistence for CapturingPersistence {
    fn persistence_capabilities(&self) -> crate::changeset::PersistenceCapabilities {
        crate::changeset::PersistenceCapabilities::SHIELDED_FVK_RESTART
    }

    fn store(
        &self,
        _wallet_id: WalletId,
        changeset: PlatformWalletChangeSet,
    ) -> Result<(), PersistenceError> {
        self.stored.lock().expect("stored lock").push(changeset);
        Ok(())
    }

    fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
        Ok(())
    }

    fn load(&self) -> Result<ClientStartState, PersistenceError> {
        *self.load_entries.lock().expect("load_entries lock") += 1;
        // Take the gate before blocking on it, so a second load() isn't
        // stuck behind the mutex of the one being held.
        let gate = self.load_gate.lock().expect("load_gate lock").take();
        if let Some(rx) = gate {
            let _ = rx.recv();
        }
        *self.load_calls.lock().expect("load_calls lock") += 1;
        let mut start = ClientStartState::default();
        start.shielded.viewing_keys = self.serve.lock().expect("serve lock").clone();
        start.shielded.per_subwallet = self
            .serve_subwallets
            .lock()
            .expect("serve_subwallets lock")
            .clone();
        Ok(start)
    }
}

/// Unique temp directory for a test's SQLite tree (no `tempfile` dev-dep).
fn temp_dir(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("viewing_key_bind_test_{tag}_{nanos}"))
}

fn coordinator_at(dir: &std::path::Path) -> Arc<NetworkShieldedCoordinator> {
    std::fs::create_dir_all(dir).expect("create temp dir");
    let db_path = dir.join("tree.sqlite");
    let store = FileBackedShieldedStore::open_path(&db_path, 100).expect("open file store");
    Arc::new(NetworkShieldedCoordinator::new(
        Arc::new(dash_sdk::Sdk::new_mock()),
        Network::Testnet,
        db_path,
        store,
    ))
}

/// Build a `PlatformWallet` over a fresh test wallet manager with the
/// given persister. Each call mimics one app process: a fresh handle
/// whose shielded slot starts unbound.
async fn platform_wallet_with<P>(persister: Arc<P>) -> PlatformWallet
where
    P: PlatformWalletPersistence + 'static,
{
    let (wallet_manager, wallet_id, balance, _signer) =
        funded_wallet_manager(StandardAccountType::BIP44Account).await;
    let sdk = Arc::new(dash_sdk::Sdk::new_mock());
    let spv = Arc::new(crate::spv::SpvRuntime::new(
        Arc::clone(&wallet_manager),
        Arc::new(crate::events::PlatformEventManager::new(Vec::new())),
    ));
    PlatformWallet::new(
        sdk,
        wallet_id,
        wallet_manager,
        balance,
        Arc::new(tokio::sync::Notify::new()),
        persister as Arc<dyn PlatformWalletPersistence>,
        Arc::new(crate::broadcaster::SpvBroadcaster::new(spv)),
    )
}

/// A shielded bind must not install ephemeral FVK state when the backend
/// cannot persist and reload it. The capability gate runs before derivation,
/// persistence, or coordinator registration.
#[tokio::test]
async fn bind_fails_closed_without_fvk_restart_capabilities() {
    let persister = Arc::new(crate::wallet::persister::NoPlatformPersistence);
    let wallet = platform_wallet_with(persister).await;
    let coordinator = coordinator_at(&temp_dir("missing_capabilities"));

    let err = wallet
        .bind_shielded(&[0x42u8; 64], &[0], &coordinator)
        .await
        .expect_err("missing FVK callbacks must fail closed");
    assert!(
        format!("{err}").contains("shielded_viewing_keys"),
        "error must name the missing feature capability: {err}"
    );
    assert!(!wallet.is_shielded_bound().await);
}

/// The full launch contract in one pass:
/// 1. the seed bind persists one 96-byte FVK row per account;
/// 2. a "restarted" wallet (fresh handle + fresh coordinator, no seed
///    anywhere — the API takes none) rebinds from those rows and
///    reports `true`;
/// 3. the rebound viewing state is identical per account (default
///    address ⇐ FVK ⇐ IVK/OVK, all pure functions of the persisted
///    bytes) and sync-capable (a coordinator-backed balance read
///    succeeds for every account).
#[tokio::test]
async fn bind_persists_viewing_keys_and_restart_rebinds_seedlessly() {
    let seed = [0x42u8; 64];
    let accounts = [0u32, 1u32];

    // Session 1: seed-backed bind.
    let persister1 = Arc::new(CapturingPersistence::default());
    let wallet1 = platform_wallet_with(Arc::clone(&persister1)).await;
    let coordinator1 = coordinator_at(&temp_dir("session1"));
    wallet1
        .bind_shielded(&seed, &accounts, &coordinator1)
        .await
        .expect("seed bind succeeds");

    let original_addresses = wallet1.shielded_default_addresses().await;
    assert_eq!(original_addresses.len(), 2, "both accounts bound");

    // The bind queued exactly one FVK row per account, 96 bytes each.
    let captured = persister1.captured_viewing_keys();
    assert_eq!(
        captured.len(),
        2,
        "one persisted viewing key per bound account"
    );
    for &account in &accounts {
        let row = captured
            .get(&SubwalletId::new(wallet1.wallet_id(), account))
            .expect("viewing key persisted for account");
        assert_eq!(row.len(), 96, "raw FVK encoding is 96 bytes");
    }

    // Session 2 ("restart"): fresh handle, fresh coordinator. The test
    // manager mints a new random wallet per call, so re-key session 1's
    // rows to the new wallet id — the exact bytes a real restart hands
    // back for its own (stable) id.
    let persister2 = Arc::new(CapturingPersistence::default());
    let wallet2 = platform_wallet_with(Arc::clone(&persister2)).await;
    let restored_rows: BTreeMap<SubwalletId, Vec<u8>> = accounts
        .iter()
        .map(|&account| {
            (
                SubwalletId::new(wallet2.wallet_id(), account),
                captured[&SubwalletId::new(wallet1.wallet_id(), account)].clone(),
            )
        })
        .collect();
    persister2.serve_viewing_keys(restored_rows);
    let coordinator2 = coordinator_at(&temp_dir("session2"));

    let rebound = wallet2
        .bind_shielded_from_persisted(&accounts, &coordinator2)
        .await
        .expect("seedless rebind succeeds");
    assert!(rebound, "persisted rows cover all accounts → rebound");
    assert!(
        persister2.load_calls() >= 1,
        "the seedless rebind actually read persistence"
    );
    assert!(wallet2.is_shielded_bound().await);

    // Identical viewing state per account. The default address is a
    // pure function of the FVK (as are IVK / OVK), so address equality
    // pins the whole viewing-grade set.
    let restored_addresses = wallet2.shielded_default_addresses().await;
    assert_eq!(restored_addresses.len(), 2);
    for &account in &accounts {
        assert_eq!(
            restored_addresses.get(&account),
            original_addresses.get(&account),
            "account {account} default address must survive the restart"
        );
    }

    // Sync capability: the coordinator-backed balance read works for
    // every rebound account (empty store → zero balances, not errors).
    let balances = wallet2
        .shielded_balances(&coordinator2)
        .await
        .expect("balance read over rebound viewing keys");
    for &account in &accounts {
        assert_eq!(balances.get(&account), Some(&0));
    }

    // No seed-backed bind ran in session 2, so nothing re-persisted:
    // the persister captured no viewing-key rows of its own.
    assert!(
        persister2.captured_viewing_keys().is_empty(),
        "seedless rebind must not re-emit viewing keys"
    );
}

/// A seed whose derived key disagrees with the persisted one must not
/// bind. The wallet's durable notes, activity and watermark are keyed by
/// `(wallet_id, account_index)` alone, so nothing marks which key
/// produced them: upserting the new key would leave the old key's notes
/// counted but unspendable, and its watermark in force — hiding the new
/// key's own history from every subsequent scan. Fail closed instead,
/// changing neither the persisted rows nor the coordinator.
#[tokio::test]
async fn bind_rejects_a_viewing_key_change_for_an_already_persisted_account() {
    let persister = Arc::new(CapturingPersistence::default());
    let wallet = platform_wallet_with(Arc::clone(&persister)).await;
    let coordinator = coordinator_at(&temp_dir("rekey_refused"));

    // Durable row from an earlier bind, under a different seed. Derived
    // on the wallet's own network so the SEED is the only difference —
    // otherwise the refusal could be a network mismatch instead.
    let persisted =
        crate::wallet::shielded::OrchardKeySet::from_seed(&[0x99u8; 64], wallet.sdk().network, 0)
            .expect("derive")
            .viewing_keys();
    let mut rows = BTreeMap::new();
    rows.insert(
        SubwalletId::new(wallet.wallet_id(), 0),
        persisted.to_fvk_bytes().to_vec(),
    );
    persister.serve_viewing_keys(rows);

    let err = wallet
        .bind_shielded(&[0x42u8; 64], &[0], &coordinator)
        .await
        .expect_err("a re-keyed account must not bind over the old key's state");
    assert!(
        format!("{err}").contains("differs"),
        "error must name the key conflict: {err}"
    );

    assert!(!wallet.is_shielded_bound().await, "nothing was installed");
    assert!(
        persister.captured_viewing_keys().is_empty(),
        "the conflicting key must not be upserted over the persisted row"
    );
    assert!(
        coordinator.registered_subwallets().await.is_empty(),
        "the coordinator must not be registered with the rejected key"
    );
}

/// A wallet the manager has removed cannot bind shielded state back onto
/// the coordinator. Callers resolve an `Arc<PlatformWallet>` and keep it
/// across the bind (which may resolve a mnemonic through the host, so it
/// is not short), so a removal can land in between; re-registering would
/// resurrect shielded history the host believes it deleted, on the very
/// next sync pass.
#[tokio::test]
async fn bind_after_wallet_removal_is_refused() {
    let persister = Arc::new(CapturingPersistence::default());
    let wallet = platform_wallet_with(Arc::clone(&persister)).await;
    let coordinator = coordinator_at(&temp_dir("detached_bind"));

    wallet
        .bind_shielded(&[0x42u8; 64], &[0], &coordinator)
        .await
        .expect("first bind succeeds");
    assert_eq!(coordinator.registered_subwallets().await.len(), 1);

    // What `PlatformWalletManager::remove_wallet` does to this handle.
    wallet.mark_shielded_detached();
    coordinator.unregister_wallet(wallet.wallet_id()).await;
    let persisted_before = persister.stored_count();

    let err = wallet
        .bind_shielded(&[0x42u8; 64], &[0], &coordinator)
        .await
        .expect_err("a removed wallet must not re-register itself");
    assert!(
        format!("{err}").contains("removed from the manager"),
        "error must name the removal: {err}"
    );
    assert!(
        coordinator.registered_subwallets().await.is_empty(),
        "the removed wallet must stay unregistered"
    );
    // The refusal must land BEFORE the viewing-key write, not after it.
    // Hosts delete their own wallet data once removal returns, so a row
    // written here survives with no wallet and no mnemonic to match it —
    // and an FVK discloses every note of its account.
    assert_eq!(
        persister.stored_count(),
        persisted_before,
        "a refused bind must not persist anything for a removed wallet"
    );
}

/// `shielded_add_account` is the other writer of a subwallet's
/// viewing-key row, so it owes the same refusal as the bind paths. An
/// account missing from the in-memory map can still have durable rows
/// from an earlier session; overwriting its key would leave those notes
/// and their watermark attributed to a key that did not produce them,
/// and nothing downstream could detect it — the next seedless bind
/// derives its keys FROM the row that was overwritten.
#[tokio::test]
async fn add_account_rejects_a_viewing_key_change_for_a_persisted_account() {
    let persister = Arc::new(CapturingPersistence::default());
    let wallet = platform_wallet_with(Arc::clone(&persister)).await;
    let coordinator = coordinator_at(&temp_dir("add_account_rekey"));
    let seed = [0x42u8; 64];

    // Durable rows: account 0 under this seed (so the bind agrees),
    // account 1 under a different one. Derived on the wallet's own
    // network — the bind derives with `sdk.network`, and a mismatch there
    // would make this test pass for the wrong reason.
    let network = wallet.sdk().network;
    let views0 = crate::wallet::shielded::OrchardKeySet::from_seed(&seed, network, 0)
        .expect("derive")
        .viewing_keys();
    let foreign1 = crate::wallet::shielded::OrchardKeySet::from_seed(&[0x99u8; 64], network, 1)
        .expect("derive")
        .viewing_keys();
    let mut rows = BTreeMap::new();
    rows.insert(
        SubwalletId::new(wallet.wallet_id(), 0),
        views0.to_fvk_bytes().to_vec(),
    );
    rows.insert(
        SubwalletId::new(wallet.wallet_id(), 1),
        foreign1.to_fvk_bytes().to_vec(),
    );
    persister.serve_viewing_keys(rows);

    wallet
        .bind_shielded(&seed, &[0], &coordinator)
        .await
        .expect("bind of the matching account succeeds");
    let persisted_before = persister.stored_count();

    let err = wallet
        .shielded_add_account(&seed, 1)
        .await
        .expect_err("adding an account whose persisted key differs must fail closed");
    assert!(
        format!("{err}").contains("differs"),
        "error must name the key conflict: {err}"
    );
    assert_eq!(
        wallet.shielded_account_indices().await,
        vec![0],
        "the conflicting account must not be installed on the handle"
    );
    assert_eq!(
        persister.stored_count(),
        persisted_before,
        "the conflicting key must not be upserted over the persisted row"
    );
}

/// A bind that read the host snapshot before a Clear must not put that
/// snapshot back afterwards.
///
/// The snapshot has to be read before the install transaction opens (the
/// seedless path reconstructs its keys from it, and a host callback must
/// not run under the coordinator's lifecycle mutex), so serializing the
/// transaction alone cannot cover this: Clear takes the mutex, finishes,
/// and the host then wipes its own rows — while a bind still holds the
/// pre-Clear notes and watermark in hand. Restoring them resurrects
/// history the user deleted and re-arms a watermark that reports
/// caught-up, so the cold rebuild from index 0 that Clear promises never
/// runs.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn bind_does_not_restore_a_snapshot_that_predates_a_clear() {
    let persister = Arc::new(CapturingPersistence::default());
    let wallet = platform_wallet_with(Arc::clone(&persister)).await;
    let coordinator = coordinator_at(&temp_dir("clear_during_bind"));
    let id = SubwalletId::new(wallet.wallet_id(), 0);

    // Pre-Clear host state: one note and a high watermark.
    let mut snapshot = BTreeMap::new();
    snapshot.insert(
        id,
        ShieldedSubwalletStartState {
            notes: vec![crate::wallet::shielded::ShieldedNote {
                position: 7,
                cmx: [0x2A; 32],
                nullifier: [0xE0; 32],
                block_height: 100,
                is_spent: false,
                value: 1_000,
                note_data: vec![0u8; 115],
            }],
            last_synced_index: 900_000,
            ..Default::default()
        },
    );
    persister.serve_subwallets(snapshot);

    // Park the bind inside load(): it has sampled the clear generation
    // and is about to read the pre-Clear snapshot.
    let release = persister.gate_next_load();
    let bind = {
        let wallet = wallet.clone();
        let coordinator = Arc::clone(&coordinator);
        tokio::spawn(async move {
            wallet
                .bind_shielded(&[0x42u8; 64], &[0], &coordinator)
                .await
        })
    };
    // Wait for the bind to actually REACH the gate. Without this
    // handshake the Clear below could land before the bind samples the
    // generation — a different ordering, which the fix is not about and
    // which would make this test pass either way.
    for _ in 0..400 {
        if persister.load_entries() > 0 {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    assert!(
        persister.load_entries() > 0,
        "bind never reached the persister load"
    );

    // The user taps Clear and it completes — store purged, registries
    // dropped — then the host wipes its own rows.
    coordinator.clear().await.expect("clear succeeds");

    release.send(()).expect("release the parked load");
    bind.await.expect("bind task").expect("bind succeeds");

    let store = coordinator.store().read().await;
    assert!(
        store.get_all_notes(id).unwrap().is_empty(),
        "a snapshot read before the Clear must not be restored after it — those notes \
         are exactly what the user asked to delete"
    );
    assert_eq!(
        store.last_synced_note_index(id).unwrap(),
        0,
        "the pre-Clear watermark must not come back; it would report caught-up and \
         suppress the cold rebuild from index 0"
    );
}

/// Two binds of one wallet must not interleave. Each publishes the
/// viewing-grade map on the handle and then replaces the coordinator's
/// registration; interleaved, the two commits can land in opposite
/// orders, leaving sync trial-decrypting under one bind's keys while
/// addresses, balances and spends use the other's.
///
/// The first bind is parked mid-transaction by holding the store lock its
/// restore needs, then a second bind is started: it must not be able to
/// publish its registration until the first one commits.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_second_bind_cannot_commit_inside_another_binds_transaction() {
    let persister = Arc::new(CapturingPersistence::default());
    let wallet = platform_wallet_with(Arc::clone(&persister)).await;
    let coordinator = coordinator_at(&temp_dir("bind_interleave"));
    let wallet_id = wallet.wallet_id();

    // Give the restore something to apply so it reaches the store lock.
    let mut snapshot = BTreeMap::new();
    snapshot.insert(
        SubwalletId::new(wallet_id, 0),
        ShieldedSubwalletStartState {
            last_synced_index: 1,
            ..Default::default()
        },
    );
    persister.serve_subwallets(snapshot);

    // Park bind A inside its transaction: it registers, then blocks in
    // the restore on the store write lock this test holds.
    let store_guard = coordinator.store().write().await;
    let bind_a = {
        let wallet = wallet.clone();
        let coordinator = Arc::clone(&coordinator);
        tokio::spawn(async move {
            wallet
                .bind_shielded(&[0x42u8; 64], &[0], &coordinator)
                .await
        })
    };
    for _ in 0..200 {
        if !coordinator.registered_subwallets().await.is_empty() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    assert_eq!(
        coordinator.registered_subwallets().await.len(),
        1,
        "bind A must have registered and parked in its restore"
    );

    // Bind B adds an account, so its registration would not need the
    // store lock: only the transaction can hold it back.
    let bind_b = {
        let wallet = wallet.clone();
        let coordinator = Arc::clone(&coordinator);
        tokio::spawn(async move {
            wallet
                .bind_shielded(&[0x42u8; 64], &[0, 1], &coordinator)
                .await
        })
    };
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    assert_eq!(
        coordinator.registered_subwallets().await.len(),
        1,
        "bind B must not publish its registration while bind A's transaction is open"
    );

    drop(store_guard);
    bind_a.await.expect("bind A task").expect("bind A");
    bind_b.await.expect("bind B task").expect("bind B");

    let registered: Vec<u32> = coordinator
        .registered_subwallets()
        .await
        .into_iter()
        .map(|id| id.account_index)
        .collect();
    assert_eq!(
        registered,
        wallet.shielded_account_indices().await,
        "the coordinator's registration and the wallet's own keys must agree once \
         both binds have run"
    );
    assert_eq!(registered, vec![0, 1], "the last bind to run wins");
}

/// First-launch shape: no persisted rows → `Ok(false)`, no state
/// change, so the caller knows to fall back to the seed path. Partial
/// coverage (a requested account missing) behaves the same.
#[tokio::test]
async fn rebind_without_persisted_rows_reports_false_and_binds_nothing() {
    let persister = Arc::new(CapturingPersistence::default());
    let wallet = platform_wallet_with(Arc::clone(&persister)).await;
    let coordinator = coordinator_at(&temp_dir("no_rows"));

    let rebound = wallet
        .bind_shielded_from_persisted(&[0], &coordinator)
        .await
        .expect("load succeeds with no rows");
    assert!(!rebound, "no persisted rows → caller falls back to seed");
    assert!(!wallet.is_shielded_bound().await);

    // Partial coverage: stage account 0 only, request 0 and 1.
    let seed = [0x42u8; 64];
    let views0 = crate::wallet::shielded::OrchardKeySet::from_seed(&seed, Network::Testnet, 0)
        .expect("derive")
        .viewing_keys();
    let mut rows = BTreeMap::new();
    rows.insert(
        SubwalletId::new(wallet.wallet_id(), 0),
        views0.to_fvk_bytes().to_vec(),
    );
    persister.serve_viewing_keys(rows);

    let rebound = wallet
        .bind_shielded_from_persisted(&[0, 1], &coordinator)
        .await
        .expect("load succeeds");
    assert!(
        !rebound,
        "any requested account without a persisted row → fall back to seed"
    );
    assert!(!wallet.is_shielded_bound().await);

    // A corrupt row is an error, not a silent fallback.
    let mut rows = BTreeMap::new();
    rows.insert(SubwalletId::new(wallet.wallet_id(), 0), vec![0u8; 5]);
    persister.serve_viewing_keys(rows);
    assert!(
        wallet
            .bind_shielded_from_persisted(&[0], &coordinator)
            .await
            .is_err(),
        "malformed persisted viewing key must surface as an error"
    );
}
