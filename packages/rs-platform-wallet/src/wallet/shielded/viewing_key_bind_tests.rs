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
use crate::changeset::{ClientStartState, PlatformWalletChangeSet};
use crate::test_support::funded_wallet_manager;
use crate::wallet::platform_wallet::{PlatformWallet, WalletId};
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
    load_calls: Mutex<usize>,
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

    fn load_calls(&self) -> usize {
        *self.load_calls.lock().expect("load_calls lock")
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
        *self.load_calls.lock().expect("load_calls lock") += 1;
        let mut start = ClientStartState::default();
        start.shielded.viewing_keys = self.serve.lock().expect("serve lock").clone();
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
        Arc::clone(&sdk),
        wallet_id,
        wallet_manager,
        balance,
        Arc::new(tokio::sync::Notify::new()),
        persister as Arc<dyn PlatformWalletPersistence>,
        Arc::new(crate::broadcaster::SpvBroadcaster::new(spv, sdk)),
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
