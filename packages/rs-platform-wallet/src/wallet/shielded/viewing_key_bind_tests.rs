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
use crate::wallet::persister::WalletPersister;
use crate::wallet::platform_wallet::{PlatformWallet, WalletId};
use crate::wallet::shielded::keys::OrchardKeySet;
use crate::wallet::shielded::store::{PendingRedrive, ShieldedStore};
use crate::wallet::shielded::{
    FileBackedShieldedStore, NetworkShieldedCoordinator, ShieldedBalanceSource,
    ShieldedLocalBalanceState, ShieldedNote, SubwalletId,
};

/// Persister double for both halves of the round trip:
/// - `store` captures every queued changeset (the persist half);
/// - `load` serves whatever viewing-key rows the test staged via
///   [`Self::serve_viewing_keys`] (the restart half). Load calls are
///   counted so the test can assert the seedless path actually read
///   persistence rather than short-circuiting.
#[derive(Default)]
struct CapturingPersistence {
    stored: Mutex<Vec<PlatformWalletChangeSet>>,
    durability_events: Mutex<Vec<&'static str>>,
    fail_flush: std::sync::atomic::AtomicBool,
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
    fail_next_load: Mutex<bool>,
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
        self.durability_events.lock().unwrap().push("store");
        self.stored.lock().expect("stored lock").push(changeset);
        Ok(())
    }

    fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
        self.durability_events.lock().unwrap().push("flush");
        if self.fail_flush.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(PersistenceError::backend(std::io::Error::other(
                "injected flush failure",
            )));
        }
        Ok(())
    }

    fn load(&self) -> Result<ClientStartState, PersistenceError> {
        *self.load_entries.lock().expect("load_entries lock") += 1;
        if std::mem::take(&mut *self.fail_next_load.lock().expect("failure lock")) {
            return Err(PersistenceError::backend(
                "injected local restore load failure",
            ));
        }
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
    coordinator_for(dir, Network::Testnet)
}

fn coordinator_for(dir: &std::path::Path, network: Network) -> Arc<NetworkShieldedCoordinator> {
    std::fs::create_dir_all(dir).expect("create temp dir");
    let db_path = dir.join("tree.sqlite");
    let store = FileBackedShieldedStore::open_path(&db_path, 100).expect("open file store");
    Arc::new(NetworkShieldedCoordinator::new(
        Arc::new(dash_sdk::Sdk::new_mock()),
        network,
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
    // On the wallet manager's and the coordinators' network: the bare mock
    // SDK is a mainnet one, and a bind refuses a coordinator of another
    // network than the SDK's.
    let sdk = Arc::new(
        dash_sdk::SdkBuilder::new_mock()
            .with_network(Network::Testnet)
            .build()
            .expect("mock sdk"),
    );
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
        Arc::new(std::sync::atomic::AtomicBool::new(false)),
        crate::changeset::DurableCursors::default(),
    )
}

mod zero_amount_tests {
    use super::*;
    use crate::error::PlatformWalletError;
    use dpp::address_funds::PlatformAddress;
    use dpp::identity::IdentityPublicKey;
    use dpp::prelude::Identifier;
    use dpp::shielded::builder::OrchardProver;
    use dpp::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
    use futures::FutureExt;
    use grovedb_commitment_tree::ProvingKey;

    struct PanicProver;

    impl OrchardProver for PanicProver {
        fn proving_key(&self) -> &ProvingKey {
            panic!("zero-amount spends must not invoke the prover");
        }
    }

    #[derive(Clone, Copy, Debug)]
    enum Spend {
        Transfer,
        Unshield,
        Withdraw,
        IdentityTopUp,
        IdentityCreate,
    }

    #[derive(Clone, Copy, Debug)]
    enum Held {
        StoreLock,
        ShieldGuard,
        Nothing,
    }

    async fn assert_zero_amount_rejected(spend: Spend) {
        let dir = tempfile::tempdir().expect("temporary shielded store");
        let coordinator = coordinator_at(dir.path());
        let wallet = platform_wallet_with(Arc::new(CapturingPersistence::default())).await;
        let network = wallet.network();
        let seed = [0x42; 64];
        wallet
            .bind_shielded(&seed, &[0], &coordinator)
            .await
            .unwrap();
        let recipient = OrchardKeySet::from_seed(&[0x43; 64], network, 0)
            .unwrap()
            .default_address
            .to_raw_address_bytes();
        let platform_address = PlatformAddress::P2pkh([0x11; 20]).to_bech32m_string(network);
        let core_address = dashcore::Address::dummy(network, 1).to_string();
        let identity_id = Identifier::from([0x11; 32]);
        let (identity_key, _) = IdentityPublicKey::random_ecdsa_master_authentication_key(
            0,
            Some(1),
            dpp::version::PlatformVersion::latest(),
        )
        .expect("identity master key");
        let identity_signer = simple_signer::signer::SimpleSigner::default();

        // Exercise a contended sync store, a long-running shielded operation holding
        // `shield_guard`, and empty-wallet error precedence.
        for held in [Held::StoreLock, Held::ShieldGuard, Held::Nothing] {
            let _store_guard = match held {
                Held::StoreLock => Some(coordinator.store().write().await),
                _ => None,
            };
            let _shield_guard = match held {
                Held::ShieldGuard => Some(wallet.shield_guard.lock().await),
                _ => None,
            };
            let result = async {
                match spend {
                    Spend::Transfer => {
                        wallet
                            .shielded_transfer_to(
                                &coordinator,
                                &seed,
                                0,
                                &recipient,
                                0,
                                [0; 36],
                                PanicProver,
                            )
                            .await
                    }
                    Spend::Unshield => {
                        wallet
                            .shielded_unshield_to(
                                &coordinator,
                                &seed,
                                0,
                                &platform_address,
                                0,
                                PanicProver,
                            )
                            .await
                    }
                    Spend::Withdraw => {
                        wallet
                            .shielded_withdraw_to(
                                &coordinator,
                                &seed,
                                0,
                                &core_address,
                                0,
                                1,
                                PanicProver,
                            )
                            .await
                    }
                    Spend::IdentityTopUp => wallet
                        .shielded_identity_top_up_from_pool(
                            &coordinator,
                            &seed,
                            0,
                            &identity_id,
                            0,
                            PanicProver,
                        )
                        .await
                        .map(|_| ()),
                    Spend::IdentityCreate => wallet
                        .shielded_identity_create_from_pool(
                            &coordinator,
                            &seed,
                            0,
                            0,
                            vec![(
                                identity_key.clone(),
                                IdentityPublicKeyInCreation::from(&identity_key),
                            )],
                            0,
                            PlatformAddress::P2pkh([0x11; 20]),
                            &identity_signer,
                            PanicProver,
                        )
                        .await
                        .map(|_| ()),
                }
            }
            .now_or_never()
            .unwrap_or_else(|| panic!("{spend:?} waited while {held:?} was held"));
            // Identity-create spends a fixed denomination, rejected by the allow-list check.
            let expected = match spend {
                Spend::IdentityCreate => "denomination 0 is not a member",
                _ => "amount must be > 0",
            };
            assert!(
                matches!(&result, Err(PlatformWalletError::ShieldedBuildError(message))
                    if message.starts_with(expected)),
                "{spend:?} returned {result:?} while {held:?} was held"
            );
        }
    }

    #[tokio::test]
    async fn should_reject_zero_amount_transfer_before_reserving_notes() {
        assert_zero_amount_rejected(Spend::Transfer).await;
    }

    #[tokio::test]
    async fn should_reject_zero_amount_unshield_before_reserving_notes() {
        assert_zero_amount_rejected(Spend::Unshield).await;
    }

    #[tokio::test]
    async fn should_reject_zero_amount_withdraw_before_reserving_notes() {
        assert_zero_amount_rejected(Spend::Withdraw).await;
    }

    #[tokio::test]
    async fn should_reject_zero_amount_identity_top_up_before_reserving_notes() {
        assert_zero_amount_rejected(Spend::IdentityTopUp).await;
    }

    #[tokio::test]
    async fn should_reject_zero_denomination_identity_create_before_reserving_notes() {
        assert_zero_amount_rejected(Spend::IdentityCreate).await;
    }
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

#[tokio::test]
async fn local_balance_bind_load_error_surfaces_and_retry_hydrates() {
    let persister = Arc::new(CapturingPersistence::default());
    let wallet = platform_wallet_with(Arc::clone(&persister)).await;
    let coordinator = coordinator_at(&temp_dir("local_load_retry"));
    *persister.fail_next_load.lock().unwrap() = true;
    let error = wallet
        .bind_shielded(&[42; 64], &[0], &coordinator)
        .await
        .expect_err("local storage failure must fail binding");
    assert!(error
        .to_string()
        .contains("injected local restore load failure"));
    assert_eq!(
        coordinator
            .local_balance_snapshot(wallet.wallet_id())
            .await
            .unwrap(),
        ShieldedLocalBalanceState::Unbound
    );
    assert!(!wallet.is_shielded_bound().await);

    wallet
        .bind_shielded(&[42; 64], &[0], &coordinator)
        .await
        .expect("retry bind");
    let ShieldedLocalBalanceState::Ready(snapshot) = coordinator
        .local_balance_snapshot(wallet.wallet_id())
        .await
        .unwrap()
    else {
        panic!("retry must hydrate")
    };
    assert_eq!(
        snapshot.accounts[&0].source,
        ShieldedBalanceSource::NoHistory
    );
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
    let session2_dir = temp_dir("session2");
    let coordinator2 = coordinator_at(&session2_dir);
    let subwallet = SubwalletId::new(wallet2.wallet_id(), 0);
    persister2.serve_subwallets(BTreeMap::from([(
        subwallet,
        ShieldedSubwalletStartState {
            notes: [100, 200]
                .into_iter()
                .enumerate()
                .map(|(index, value)| ShieldedNote {
                    position: index as u64,
                    cmx: [index as u8 + 1; 32],
                    nullifier: [index as u8 + 1; 32],
                    value,
                    block_height: 12,
                    is_spent: false,
                    note_data: vec![0; 115],
                })
                .collect(),
            has_sync_state: true,
            last_synced_index: 2,
            ..Default::default()
        },
    )]));
    coordinator2
        .store()
        .write()
        .await
        .arm_redrive(
            subwallet,
            crate::wallet::shielded::store::PendingRedrive {
                activity_id: [7; 32],
                anchor: [8; 32],
                nullifiers: vec![[2; 32]],
                st_bytes: vec![9; 32],
                attempts: 0,
                identity_nonce_finalized: false,
                identity_user_abandoned: false,
                anchor_height: None,
            },
        )
        .expect("persist pending reservation");
    // Opening the durable tree recreates reservations from its redrive rows,
    // exactly as on launch, before the host's notes have been restored.
    drop(coordinator2);
    let coordinator2 = coordinator_at(&session2_dir);

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

    // Local startup is complete before any network scan: notes from the
    // host snapshot are hydrated, and pending inputs remain unavailable.
    let balances = wallet2
        .shielded_balances(&coordinator2)
        .await
        .expect("balance read over rebound viewing keys");
    assert_eq!(balances.get(&0), Some(&100));
    assert_eq!(balances.get(&1), Some(&0));
    let ShieldedLocalBalanceState::Ready(local) = coordinator2
        .local_balance_snapshot(wallet2.wallet_id())
        .await
        .expect("local snapshot")
    else {
        panic!("seedless bind must hydrate the local ledger")
    };
    assert_eq!(local.accounts[&0].spendable_credits, 100);
    assert_eq!(local.accounts[&0].last_scanned_index, Some(2));
    assert_eq!(local.accounts[&0].source, ShieldedBalanceSource::Restored);
    assert_eq!(local.accounts[&1].spendable_credits, 0);
    assert_eq!(local.accounts[&1].last_scanned_index, None);
    assert_eq!(local.accounts[&1].source, ShieldedBalanceSource::NoHistory);

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

/// What `PlatformWalletManager::remove_wallet` does to this handle's
/// shielded side once it holds `teardown`, the lifecycle gate of the
/// wallet's generation.
async fn finish_removal(
    wallet: &PlatformWallet,
    coordinator: &NetworkShieldedCoordinator,
    teardown: tokio::sync::OwnedRwLockWriteGuard<()>,
) {
    wallet.mark_shielded_detached();
    coordinator.unregister_wallet(wallet.wallet_id()).await;
    drop(teardown);
}

/// A bind that starts while the wallet's removal is under way stores no
/// viewing key. The attachment check alone is a point-in-time observation:
/// the removal takes the lifecycle gate and only then marks the handle
/// detached, so a bind checking in between would pass and store its key
/// after the host deleted the wallet's rows. The bind waits at the gate
/// instead, and is refused once the removal returns.
#[tokio::test]
async fn should_store_no_viewing_key_for_a_bind_admitted_behind_a_removal() {
    let persister = Arc::new(CapturingPersistence::default());
    let wallet = platform_wallet_with(Arc::clone(&persister)).await;
    let coordinator = coordinator_at(&temp_dir("bind_behind_removal"));
    let seed = [0x42u8; 64];

    // The removal holds the gate and has not marked the handle yet.
    let teardown = wallet.generation().teardown_guard().await;
    let mut bind = std::pin::pin!(wallet.bind_shielded(&seed, &[0], &coordinator));
    assert!(
        futures::poll!(bind.as_mut()).is_pending(),
        "the bind must wait for the removal under way"
    );
    finish_removal(&wallet, &coordinator, teardown).await;

    let err = bind.await.expect_err("a removed wallet must not bind");
    assert!(
        matches!(err, crate::PlatformWalletError::WalletNotFound(_)),
        "{err:?}"
    );
    assert_eq!(
        persister.stored_count(),
        0,
        "no viewing key may be stored once the removal returned"
    );
    assert!(coordinator.registered_subwallets().await.is_empty());
}

/// The same for `shielded_add_account`, the other writer of a viewing-key
/// row: admitted behind a removal, it stores nothing and adds no account.
#[tokio::test]
async fn should_store_no_viewing_key_for_an_account_added_behind_a_removal() {
    let persister = Arc::new(CapturingPersistence::default());
    let wallet = platform_wallet_with(Arc::clone(&persister)).await;
    let coordinator = coordinator_at(&temp_dir("add_account_behind_removal"));
    let seed = [0x42u8; 64];
    wallet
        .bind_shielded(&seed, &[0], &coordinator)
        .await
        .expect("bind succeeds");
    let stored_before = persister.stored_count();

    let teardown = wallet.generation().teardown_guard().await;
    let mut add = std::pin::pin!(wallet.shielded_add_account(&seed, 1));
    assert!(
        futures::poll!(add.as_mut()).is_pending(),
        "the account add must wait for the removal under way"
    );
    finish_removal(&wallet, &coordinator, teardown).await;

    let err = add
        .await
        .expect_err("a removed wallet must not add accounts");
    assert!(
        matches!(err, crate::PlatformWalletError::WalletNotFound(_)),
        "{err:?}"
    );
    assert_eq!(
        persister.stored_count(),
        stored_before,
        "no viewing key may be stored once the removal returned"
    );
    assert_eq!(wallet.shielded_account_indices().await, vec![0]);
}

/// A removal waits for a bind that already passed its attachment check, so
/// the bind's viewing-key write lands before the removal returns and the
/// host deletes the wallet's rows, never after.
///
/// Parks a bind inside the persister load, which follows the check and
/// precedes the write, and asserts the removal cannot take the gate. The
/// bind then finishes with the removal queued behind it, which it could
/// not if anything in it took the gate a second time.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn should_hold_a_removal_until_a_bind_in_flight_returns() {
    let persister = Arc::new(CapturingPersistence::default());
    let wallet = platform_wallet_with(Arc::clone(&persister)).await;
    let coordinator = coordinator_at(&temp_dir("removal_behind_bind"));
    let seed = [0x42u8; 64];

    let release = persister.gate_next_load();
    let entries_before = persister.load_entries();
    let binder = {
        let (wallet, coordinator) = (wallet.clone(), Arc::clone(&coordinator));
        tokio::spawn(async move { wallet.bind_shielded(&seed, &[0], &coordinator).await })
    };
    for _ in 0..400 {
        if persister.load_entries() > entries_before {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    assert!(
        persister.load_entries() > entries_before,
        "the bind never reached the persister load"
    );

    let mut removal = std::pin::pin!(wallet.generation().teardown_guard());
    assert!(
        futures::poll!(removal.as_mut()).is_pending(),
        "the removal must wait for the bind in flight"
    );

    release.send(()).expect("release the parked load");
    binder
        .await
        .expect("bind task")
        .expect("a bind admitted before the removal completes");
    let stored = persister.stored_count();
    assert!(
        persister
            .captured_viewing_keys()
            .contains_key(&SubwalletId::new(wallet.wallet_id(), 0)),
        "the bind stored its viewing key before the removal"
    );

    finish_removal(&wallet, &coordinator, removal.await).await;
    assert_eq!(
        persister.stored_count(),
        stored,
        "nothing is stored once the removal returned"
    );
    assert!(coordinator.registered_subwallets().await.is_empty());
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

/// A bind that waited for a Clear must not restore the host snapshot it
/// then reads.
///
/// The host wipes its own rows only once the Clear returns, so a bind
/// admitted right behind the Clear can still read the pre-Clear notes and
/// watermark. Restoring them resurrects history the user deleted and
/// re-arms a watermark that reports caught-up, so the cold rebuild from
/// index 0 that Clear promises never runs. The bind samples the clear
/// generation before it waits, so its install sees the Clear and restores
/// nothing.
#[tokio::test]
async fn should_not_restore_a_snapshot_for_a_bind_admitted_behind_a_clear() {
    let persister = Arc::new(CapturingPersistence::default());
    let wallet = platform_wallet_with(Arc::clone(&persister)).await;
    let coordinator = coordinator_at(&temp_dir("bind_behind_clear"));
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

    // Admission is held, so the Clear queues for it and the bind queues
    // behind the Clear (the lock is fair: a waiting writer goes before a
    // later reader). Releasing it admits the Clear first, and the bind
    // then reads the rows the host has not wiped yet.
    let admission = coordinator.operation_guard().await;
    let (cleared, bound, ()) = tokio::join!(
        coordinator.clear(),
        wallet.bind_shielded(&[0x42u8; 64], &[0], &coordinator),
        async move { drop(admission) },
    );
    cleared.expect("clear succeeds");
    bound.expect("bind succeeds");
    assert!(
        persister.load_calls() > 0,
        "the bind read the host snapshot"
    );

    let store = coordinator.store().read().await;
    assert!(
        store.get_all_notes(id).unwrap().is_empty(),
        "a snapshot read behind the Clear must not be restored after it: those notes \
         are exactly what the user asked to delete"
    );
    assert_eq!(
        store.last_synced_note_index(id).unwrap(),
        0,
        "the pre-Clear watermark must not come back; it would report caught-up and \
         suppress the cold rebuild from index 0"
    );
}

/// Park `bind` inside its host load, and require a Clear started meanwhile
/// to wait until the bind has returned.
async fn assert_clear_waits_for_bind<F>(
    persister: &CapturingPersistence,
    coordinator: &NetworkShieldedCoordinator,
    bind: F,
) -> F::Output
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    let entries_before = persister.load_entries();
    let release = persister.gate_next_load();
    let bind = tokio::spawn(bind);
    for _ in 0..400 {
        if persister.load_entries() > entries_before {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    assert!(
        persister.load_entries() > entries_before,
        "the bind never reached the persister load"
    );

    let mut clear = std::pin::pin!(coordinator.clear());
    assert!(
        futures::poll!(clear.as_mut()).is_pending(),
        "the Clear must wait for the bind in flight"
    );
    release.send(()).expect("release the parked load");
    let bound = bind.await.expect("bind task");
    clear.await.expect("clear succeeds");
    bound
}

/// A Clear waits for a bind in flight. The host wipes its own rows once the
/// Clear returns, so a Clear that completed while a bind sat between its
/// host load and its host store would have the bind write its viewing keys
/// after that wipe: rows for shielded state the user cleared. Waiting puts
/// the bind's write before the Clear, and so before the wipe.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn should_hold_a_clear_until_a_bind_in_flight_returns() {
    let persister = Arc::new(CapturingPersistence::default());
    let wallet = platform_wallet_with(Arc::clone(&persister)).await;
    let coordinator = coordinator_at(&temp_dir("clear_waits_for_bind"));

    let bind = {
        let wallet = wallet.clone();
        let coordinator = Arc::clone(&coordinator);
        async move {
            wallet
                .bind_shielded(&[0x42u8; 64], &[0], &coordinator)
                .await
        }
    };
    assert_clear_waits_for_bind(&persister, &coordinator, bind)
        .await
        .expect("bind succeeds");
    assert!(
        !persister.captured_viewing_keys().is_empty(),
        "the bind stored its viewing key, and did so before the Clear ran"
    );
    assert!(
        coordinator.registered_subwallets().await.is_empty(),
        "the Clear ran after the bind and dropped its registration"
    );
}

/// The seedless bind is held the same way: it restores the notes and
/// watermark of the snapshot it loaded, so a Clear completing while it sat
/// in that load must wait as well.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn should_hold_a_clear_until_a_persisted_bind_in_flight_returns() {
    let persister = Arc::new(CapturingPersistence::default());
    let wallet = platform_wallet_with(Arc::clone(&persister)).await;
    let coordinator = coordinator_at(&temp_dir("clear_waits_for_persisted_bind"));
    wallet
        .bind_shielded(&[0x42u8; 64], &[0], &coordinator)
        .await
        .expect("seed bind succeeds");
    persister.serve_viewing_keys(persister.captured_viewing_keys());

    let bind = {
        let wallet = wallet.clone();
        let coordinator = Arc::clone(&coordinator);
        async move {
            wallet
                .bind_shielded_from_persisted(&[0], &coordinator)
                .await
        }
    };
    let found = assert_clear_waits_for_bind(&persister, &coordinator, bind)
        .await
        .expect("persisted bind succeeds");
    assert!(found, "the bind found its persisted key");
    assert!(
        coordinator.registered_subwallets().await.is_empty(),
        "the Clear ran after the bind and dropped its registration"
    );
}

/// Two binds of one wallet must not interleave: neither may publish its
/// registration while the other's transaction is open. That is what
/// keeps the handle's key slot and the coordinator's registration from
/// committing in opposite orders — which would leave sync
/// trial-decrypting under one bind's keys while addresses, balances and
/// spends use the other's. (This test drives both binds from one seed,
/// so it pins the account-set half of that agreement; the key bytes are
/// identical by construction.)
///
/// The first bind is parked mid-transaction by holding the store lock its
/// restore needs, then a second bind is started.
///
/// The paused clock is load-bearing. `sleep` on a paused runtime only
/// fires once nothing else is runnable, so the assertion below reads
/// "the runtime had no work left and bind B still had not registered".
/// With a real 200 ms sleep the same assertion passes whenever bind B is
/// merely slow to be scheduled — it derives two Orchard keysets and
/// round-trips the persister before it ever reaches the lock — which on
/// a loaded CI box makes it silently green even with the lock removed.
#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn a_second_bind_cannot_commit_inside_another_binds_transaction() {
    let persister = Arc::new(CapturingPersistence::default());
    let wallet = platform_wallet_with(Arc::clone(&persister)).await;
    let coordinator = coordinator_at(&temp_dir("bind_interleave"));
    let wallet_id = wallet.wallet_id();

    // Establish the account registry without hydrating the wallet handle.
    // Startup registration now checks durable identity guards under the store
    // lock; this test isolates the later restore interleaving instead.
    let views = OrchardKeySet::from_seed(&[0x42; 64], wallet.network(), 0)
        .unwrap()
        .viewing_keys();
    coordinator
        .register_wallet(
            wallet_id,
            BTreeMap::from([(0, views)]),
            WalletPersister::new(wallet_id, persister.clone()),
        )
        .await
        .unwrap();

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
        if !wallet.shielded_account_indices().await.is_empty() {
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

/// `shielded_add_account` must not hold the key slot across its host
/// persistence calls.
///
/// A host callback invoked under that write guard deadlocks against a
/// concurrent bind: the bind holds the coordinator's lifecycle mutex and
/// waits for this slot, while the callback re-enters the FFI and waits
/// for that mutex. It also freezes every address and balance read for
/// the duration of host I/O, which is unbounded.
///
/// Parks `add_account` inside the persister load and asserts a slot
/// reader still completes.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn add_account_does_not_hold_the_key_slot_across_host_persistence() {
    let persister = Arc::new(CapturingPersistence::default());
    let wallet = platform_wallet_with(Arc::clone(&persister)).await;
    let coordinator = coordinator_at(&temp_dir("add_account_slot"));
    let seed = [0x42u8; 64];

    wallet
        .bind_shielded(&seed, &[0], &coordinator)
        .await
        .expect("bind succeeds");

    let release = persister.gate_next_load();
    let entries_before = persister.load_entries();
    let adder = {
        let wallet = wallet.clone();
        tokio::spawn(async move { wallet.shielded_add_account(&seed, 1).await })
    };
    for _ in 0..400 {
        if persister.load_entries() > entries_before {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    assert!(
        persister.load_entries() > entries_before,
        "add_account never reached the persister load"
    );

    // The slot must still be readable while the host call is parked.
    let read = tokio::time::timeout(
        std::time::Duration::from_millis(500),
        wallet.shielded_account_indices(),
    )
    .await;
    assert_eq!(
        read.expect("reading the key slot must not block on host persistence"),
        vec![0],
        "the account is not installed until its key is persisted"
    );

    release.send(()).expect("release the parked load");
    adder
        .await
        .expect("add task")
        .expect("add_account succeeds");
    assert_eq!(wallet.shielded_account_indices().await, vec![0, 1]);
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

#[tokio::test]
async fn should_keep_wallet_and_coordinator_keys_when_guarded_registration_is_rejected() {
    let persister = Arc::new(CapturingPersistence::default());
    let wallet = platform_wallet_with(Arc::clone(&persister)).await;
    let coordinator = coordinator_at(&temp_dir("bind_identity_guard"));
    wallet
        .bind_shielded(&[0x42; 64], &[0], &coordinator)
        .await
        .unwrap();
    let id = SubwalletId::new(wallet.wallet_id(), 0);
    coordinator
        .store()
        .write()
        .await
        .arm_redrive(
            id,
            PendingRedrive {
                activity_id: [1; 32],
                anchor: [2; 32],
                nullifiers: vec![],
                st_bytes: vec![3; 64],
                attempts: 0,
                identity_nonce_finalized: false,
                identity_user_abandoned: false,
                anchor_height: None,
            },
        )
        .unwrap();
    wallet
        .bind_shielded(&[0x42; 64], &[1], &coordinator)
        .await
        .expect_err("cannot remove the unresolved debit's owner");
    assert_eq!(wallet.shielded_account_indices().await, vec![0]);
    assert_eq!(coordinator.registered_subwallets().await, vec![id]);
    assert_eq!(
        coordinator
            .store()
            .read()
            .await
            .pending_redrives(id)
            .unwrap()
            .len(),
        1
    );
}

/// `shielded_fresh_address` / `is_own_shielded_address` report "not
/// bound" before a bind and answer over the bound account's keys after:
/// a fresh address is 43 bytes, differs from the default, and is own;
/// another seed's address is not.
#[tokio::test]
async fn fresh_address_and_own_check_follow_the_bound_account() {
    let wallet = platform_wallet_with(Arc::new(CapturingPersistence::default())).await;

    assert_eq!(
        wallet
            .shielded_fresh_address(0)
            .await
            .expect("no CSPRNG failure"),
        None,
        "unbound wallet has no fresh address"
    );
    assert_eq!(wallet.is_own_shielded_address(0, &[0u8; 43]).await, None);

    let coordinator = coordinator_at(&temp_dir("fresh_address"));
    wallet
        .bind_shielded(&[0x42u8; 64], &[0], &coordinator)
        .await
        .expect("seed bind succeeds");

    let default = wallet.shielded_default_address(0).await.expect("bound");
    let fresh = wallet
        .shielded_fresh_address(0)
        .await
        .expect("no CSPRNG failure")
        .expect("bound account has a fresh address");
    assert_ne!(fresh, default, "fresh address is not the default");
    assert_eq!(
        wallet
            .shielded_fresh_address(1)
            .await
            .expect("no CSPRNG failure"),
        None,
        "unbound account has no fresh address"
    );

    assert_eq!(wallet.is_own_shielded_address(0, &fresh).await, Some(true));
    assert_eq!(
        wallet.is_own_shielded_address(0, &default).await,
        Some(true)
    );
    let foreign = OrchardKeySet::from_seed(&[0x24u8; 64], Network::Testnet, 0)
        .expect("derivation succeeds")
        .default_address
        .to_raw_address_bytes();
    assert_eq!(
        wallet.is_own_shielded_address(0, &foreign).await,
        Some(false)
    );
    assert_eq!(
        wallet.is_own_shielded_address(0, &fresh[..42]).await,
        Some(false)
    );
    assert_eq!(wallet.is_own_shielded_address(1, &fresh).await, None);
}

/// The MASTER authentication key this seed derives for `identity_index` on
/// testnet, as an identity public-key map: what discovery and registration
/// leave on a wallet-owned identity, and what tip preparation verifies.
fn seed_master_key(
    seed: &[u8],
    identity_index: u32,
) -> BTreeMap<dpp::identity::KeyID, dpp::identity::IdentityPublicKey> {
    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
    use dpp::platform_value::BinaryData;
    let master = key_wallet::bip32::ExtendedPrivKey::new_master(Network::Testnet, seed).unwrap();
    let derived = crate::wallet::identity::network::derive_ecdsa_identity_auth_keypair_from_master(
        &master,
        Network::Testnet,
        identity_index,
        crate::wallet::identity::network::MASTER_KEY_INDEX,
    )
    .unwrap();
    BTreeMap::from([(
        0,
        IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: 0,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::MASTER,
            contract_bounds: None,
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: BinaryData::new(derived.public_key.to_vec()),
            disabled_at: None,
        }),
    )])
}

/// Cold reload of a host that cannot store "index unknown": a wallet-attached
/// identity whose index was lost comes back at placeholder slot 0 while
/// identity 0's tip account (and its funds) is retained and bound. Tip
/// preparation must refuse it instead of handing out identity 0's address,
/// and must accept it once rediscovery proves its real index.
#[tokio::test]
async fn should_refuse_tip_account_for_a_placeholder_index_until_it_is_verified() {
    use crate::shielded_identity_account_index;
    use dpp::identity::{Identity, IdentityV0};
    use dpp::prelude::Identifier;
    let phrase = crate::test_support::MESSAGE_SIGNING_TEST_MNEMONIC;
    let seed = key_wallet::Mnemonic::from_phrase(phrase)
        .unwrap()
        .to_seed("");
    let (wallet_manager, wallet_id, _, _) =
        crate::test_support::mnemonic_wallet_manager(phrase).await;
    let generation = wallet_manager
        .read()
        .await
        .get_wallet_info(&wallet_id)
        .unwrap()
        .generation
        .clone();
    let spv = Arc::new(crate::spv::SpvRuntime::new(
        Arc::clone(&wallet_manager),
        Arc::new(crate::events::PlatformEventManager::new(Vec::new())),
    ));
    let sdk = dash_sdk::SdkBuilder::new_mock()
        .with_network(Network::Testnet)
        .build()
        .unwrap();
    let persister = Arc::new(CapturingPersistence::default());
    let wallet = PlatformWallet::new(
        Arc::new(sdk),
        wallet_id,
        wallet_manager,
        generation,
        Arc::new(tokio::sync::Notify::new()),
        persister.clone(),
        Arc::new(crate::broadcaster::SpvBroadcaster::new(spv)),
        Arc::new(std::sync::atomic::AtomicBool::new(false)),
        crate::changeset::DurableCursors::default(),
    );
    // Restored at the host's placeholder 0, but its keys are identity 5's.
    let id = Identifier::from([0x55; 32]);
    {
        let mut wm = wallet.wallet_manager.write().await;
        wm.get_wallet_info_mut(&wallet.wallet_id())
            .unwrap()
            .identity_manager
            .add_identity(
                Identity::V0(IdentityV0 {
                    id,
                    public_keys: seed_master_key(&seed[..], 5),
                    balance: 0,
                    revision: 0,
                }),
                0,
                wallet.wallet_id(),
                wallet.persister(),
            )
            .unwrap();
    }
    let coordinator = coordinator_at(&temp_dir("tips_placeholder_index"));
    let identity_zero_tips = shielded_identity_account_index(0).unwrap();
    wallet
        .bind_shielded(&seed[..], &[0, identity_zero_tips], &coordinator)
        .await
        .unwrap();
    let identity_zero_address = wallet
        .shielded_default_address(identity_zero_tips)
        .await
        .unwrap();

    let refused = wallet
        .prepare_shielded_identity_address(&seed[..], &id, &coordinator)
        .await;
    assert!(
        matches!(refused, Err(crate::PlatformWalletError::InvalidIdentityData(ref msg)) if msg.contains("not verified")),
        "{refused:?}"
    );

    // Rediscovery proves index 5, and the identity is stored at that slot.
    {
        let mut wm = wallet.wallet_manager.write().await;
        let manager = &mut wm
            .get_wallet_info_mut(&wallet.wallet_id())
            .unwrap()
            .identity_manager;
        let identity = manager.remove_identity(&id, wallet.persister()).unwrap();
        manager
            .add_identity(identity, 5, wallet.wallet_id(), wallet.persister())
            .unwrap();
    }
    let address = wallet
        .prepare_shielded_identity_address(&seed[..], &id, &coordinator)
        .await
        .unwrap();
    assert_ne!(address, identity_zero_address);
    assert_eq!(
        wallet
            .shielded_default_address(shielded_identity_account_index(5).unwrap())
            .await
            .unwrap(),
        address
    );
    assert_eq!(
        wallet.prepared_shielded_identity_account(&id).await,
        Some(shielded_identity_account_index(5).unwrap())
    );
}

/// A profile is not a recovery record. Identity discovery alone must restore
/// the reserved account, even when the owner removed the tip address entirely.
#[tokio::test]
async fn should_restore_tip_account_from_identity_discovery_and_register_for_sync() {
    use crate::shielded_identity_account_index;
    use dpp::identity::{Identity, IdentityV0};
    use dpp::prelude::Identifier;
    let phrase = crate::test_support::MESSAGE_SIGNING_TEST_MNEMONIC;
    let seed = key_wallet::Mnemonic::from_phrase(phrase)
        .unwrap()
        .to_seed("");
    let id = Identifier::from([0x33; 32]);
    let account = shielded_identity_account_index(3).unwrap();
    let mut original = None;
    for session in 0..2 {
        let persister = Arc::new(CapturingPersistence::default());
        let (wallet_manager, wallet_id, _, _) =
            crate::test_support::mnemonic_wallet_manager(phrase).await;
        let generation = wallet_manager
            .read()
            .await
            .get_wallet_info(&wallet_id)
            .unwrap()
            .generation
            .clone();
        let spv = Arc::new(crate::spv::SpvRuntime::new(
            Arc::clone(&wallet_manager),
            Arc::new(crate::events::PlatformEventManager::new(Vec::new())),
        ));
        let sdk = dash_sdk::SdkBuilder::new_mock()
            .with_network(Network::Testnet)
            .build()
            .unwrap();
        let wallet = PlatformWallet::new(
            Arc::new(sdk),
            wallet_id,
            wallet_manager,
            generation,
            Arc::new(tokio::sync::Notify::new()),
            persister.clone(),
            Arc::new(crate::broadcaster::SpvBroadcaster::new(spv)),
            Arc::new(std::sync::atomic::AtomicBool::new(false)),
            crate::changeset::DurableCursors::default(),
        );
        {
            let mut wm = wallet.wallet_manager.write().await;
            wm.get_wallet_info_mut(&wallet.wallet_id())
                .unwrap()
                .identity_manager
                .add_identity(
                    Identity::V0(IdentityV0 {
                        id,
                        public_keys: seed_master_key(&seed[..], 3),
                        balance: 0,
                        revision: 0,
                    }),
                    3,
                    wallet.wallet_id(),
                    wallet.persister(),
                )
                .unwrap();
        }
        {
            let mut wm = wallet.wallet_manager.write().await;
            wm.get_wallet_info_mut(&wallet.wallet_id())
                .unwrap()
                .identity_manager
                .add_identity(
                    Identity::V0(IdentityV0 {
                        id: Identifier::from([0x44; 32]),
                        public_keys: BTreeMap::new(),
                        balance: 0,
                        revision: 0,
                    }),
                    crate::SHIELDED_IDENTITY_ACCOUNT_BASE,
                    wallet.wallet_id(),
                    wallet.persister(),
                )
                .unwrap();
        }
        let coordinator = coordinator_at(&temp_dir(&format!("tips_restore_{session}")));
        // Fresh database, no viewing keys, no published profile, and caller
        // only knows about ordinary account zero.
        assert!(!wallet
            .bind_shielded_from_persisted(&[0], &coordinator)
            .await
            .unwrap());
        // Upgrading an existing wallet can leave account zero persisted while
        // identity discovery introduces a tip account without an FVK. Seedless
        // success here would suppress the host's seed fallback and lose scans
        // for that identity's tip history.
        let ordinary = super::OrchardKeySet::from_seed(&seed[..], Network::Testnet, 0).unwrap();
        persister.serve_viewing_keys(BTreeMap::from([(
            SubwalletId::new(wallet.wallet_id(), 0),
            ordinary.full_viewing_key.to_bytes().to_vec(),
        )]));
        assert!(!wallet
            .bind_shielded_from_persisted(&[0], &coordinator)
            .await
            .unwrap());
        assert!(!wallet.is_shielded_bound().await);
        assert!(wallet
            .prepare_shielded_identity_address(&[0x11; 64], &id, &coordinator)
            .await
            .is_err());
        assert!(matches!(
            wallet
                .prepare_shielded_identity_address(&seed[..], &id, &coordinator)
                .await,
            Err(crate::PlatformWalletError::ShieldedNotBound)
        ));
        assert!(!wallet.is_shielded_bound().await);
        assert!(persister.captured_viewing_keys().is_empty());
        assert!(coordinator.registered_subwallets().await.is_empty());
        wallet
            .bind_shielded(&seed[..], &[0], &coordinator)
            .await
            .unwrap();
        assert!(coordinator
            .registered_subwallets()
            .await
            .contains(&SubwalletId::new(wallet.wallet_id(), account)));
        assert!(wallet
            .prepare_shielded_identity_address(&[0x11; 64], &id, &coordinator)
            .await
            .is_err());
        // A queued FVK is insufficient: no publishable address may escape a
        // failed durability barrier, even though bind installed keys in memory.
        persister.durability_events.lock().unwrap().clear();
        persister
            .fail_flush
            .store(true, std::sync::atomic::Ordering::SeqCst);
        assert!(matches!(
            wallet
                .prepare_shielded_identity_address(&seed[..], &id, &coordinator)
                .await,
            Err(crate::PlatformWalletError::PersisterFlush(_))
        ));
        assert_eq!(
            *persister.durability_events.lock().unwrap(),
            ["store", "flush"]
        );
        assert!(wallet.shielded_default_address(account).await.is_some());
        persister
            .fail_flush
            .store(false, std::sync::atomic::Ordering::SeqCst);
        persister.durability_events.lock().unwrap().clear();
        let address = wallet
            .prepare_shielded_identity_address(&seed[..], &id, &coordinator)
            .await
            .unwrap();
        assert!(wallet
            .prepare_shielded_identity_address(
                &seed[..],
                &Identifier::from([0x44; 32]),
                &coordinator
            )
            .await
            .is_err());
        assert_eq!(
            *persister.durability_events.lock().unwrap(),
            ["store", "flush"]
        );
        assert_ne!(wallet.shielded_default_address(0).await.unwrap(), address);
        assert_eq!(
            wallet
                .prepare_shielded_identity_address(&seed[..], &id, &coordinator)
                .await
                .unwrap(),
            address
        );
        assert!(persister
            .captured_viewing_keys()
            .contains_key(&SubwalletId::new(wallet.wallet_id(), account)));
        if let Some(first) = original {
            assert_eq!(first, address);
        } else {
            original = Some(address);
        }
    }
}

#[tokio::test]
async fn should_rebind_retired_tip_accounts_without_profile_or_identity() {
    let seed = [0x42; 64];
    let account = crate::shielded_identity_account_index(7).unwrap();
    let persister = Arc::new(CapturingPersistence::default());
    let wallet = platform_wallet_with(Arc::clone(&persister)).await;
    let keys = super::OrchardKeySet::from_seed(&seed[..], Network::Testnet, account).unwrap();
    let ordinary = super::OrchardKeySet::from_seed(&seed[..], Network::Testnet, 0).unwrap();
    persister.serve_viewing_keys(BTreeMap::from([
        (
            SubwalletId::new(wallet.wallet_id(), 0),
            ordinary.full_viewing_key.to_bytes().to_vec(),
        ),
        (
            SubwalletId::new(wallet.wallet_id(), account),
            keys.full_viewing_key.to_bytes().to_vec(),
        ),
    ]));
    let coordinator = coordinator_at(&temp_dir("retired_tips"));
    assert!(wallet
        .bind_shielded_from_persisted(&[0], &coordinator)
        .await
        .unwrap());
    assert_eq!(
        wallet.shielded_default_address(account).await.unwrap(),
        keys.default_address.to_raw_address_bytes()
    );
    assert!(coordinator
        .registered_subwallets()
        .await
        .contains(&SubwalletId::new(wallet.wallet_id(), account)));
}

/// A host rebind passes only ordinary accounts (FFI and host SDKs cap the
/// request at 64 entries); Rust adds every persisted tip account itself. With
/// more than 64 tip accounts on record, a `[0]` request must still bind them
/// all, both seedlessly and from the seed, and the resulting
/// `shielded_account_indices` snapshot exceeds the request cap, which is why
/// hosts must not echo it back as a bind request.
#[tokio::test]
async fn should_bind_more_than_64_persisted_tip_accounts_from_an_ordinary_request() {
    let seed = [0x42; 64];
    let tips: Vec<u32> = (0..70)
        .map(|index| crate::shielded_identity_account_index(index).unwrap())
        .collect();
    for (tag, seeded) in [("many_tips_persisted", false), ("many_tips_seeded", true)] {
        let persister = Arc::new(CapturingPersistence::default());
        let wallet = platform_wallet_with(Arc::clone(&persister)).await;
        let mut rows = BTreeMap::new();
        for &account in std::iter::once(&0).chain(tips.iter()) {
            let keys =
                super::OrchardKeySet::from_seed(&seed[..], wallet.sdk().network, account).unwrap();
            rows.insert(
                SubwalletId::new(wallet.wallet_id(), account),
                keys.full_viewing_key.to_bytes().to_vec(),
            );
        }
        persister.serve_viewing_keys(rows);
        let coordinator = coordinator_at(&temp_dir(tag));
        if seeded {
            wallet
                .bind_shielded(&seed[..], &[0], &coordinator)
                .await
                .unwrap();
        } else {
            assert!(wallet
                .bind_shielded_from_persisted(&[0], &coordinator)
                .await
                .unwrap());
        }
        let bound = wallet.shielded_account_indices().await;
        assert_eq!(bound.len(), 71, "{tag}");
        assert!(
            bound.len() > 64,
            "{tag}: the snapshot exceeds the request cap"
        );
        let ordinary: Vec<u32> = bound
            .iter()
            .copied()
            .filter(|&account| !super::is_shielded_identity_account(account))
            .collect();
        assert_eq!(ordinary, vec![0], "{tag}");
        let registered = coordinator.registered_subwallets().await;
        for &account in &tips {
            assert!(
                registered.contains(&SubwalletId::new(wallet.wallet_id(), account)),
                "{tag}: tip account {account} registered for sync"
            );
        }
    }
}

/// A wallet bound to account 0 whose identity at index 2 can prepare a tip
/// address, for racing preparation against other account-set writers.
async fn tip_race_fixture(
    tag: &str,
) -> (
    PlatformWallet,
    [u8; 64],
    dpp::prelude::Identifier,
    Arc<crate::wallet::shielded::NetworkShieldedCoordinator>,
) {
    use dpp::identity::{Identity, IdentityV0};
    use dpp::prelude::Identifier;
    let phrase = crate::test_support::MESSAGE_SIGNING_TEST_MNEMONIC;
    let seed = key_wallet::Mnemonic::from_phrase(phrase)
        .unwrap()
        .to_seed("");
    let (wallet_manager, wallet_id, _, _) =
        crate::test_support::mnemonic_wallet_manager(phrase).await;
    let generation = wallet_manager
        .read()
        .await
        .get_wallet_info(&wallet_id)
        .unwrap()
        .generation
        .clone();
    let spv = Arc::new(crate::spv::SpvRuntime::new(
        Arc::clone(&wallet_manager),
        Arc::new(crate::events::PlatformEventManager::new(Vec::new())),
    ));
    let mock_dir = temp_dir(tag);
    std::fs::create_dir_all(&mock_dir).unwrap();
    let sdk = dash_sdk::SdkBuilder::new_mock()
        .with_network(Network::Testnet)
        .with_dump_dir(&mock_dir)
        .with_version(dpp::version::PlatformVersion::latest())
        .build()
        .unwrap();
    let persister = Arc::new(CapturingPersistence::default());
    let wallet = PlatformWallet::new(
        Arc::new(sdk),
        wallet_id,
        wallet_manager,
        generation,
        Arc::new(tokio::sync::Notify::new()),
        persister.clone(),
        Arc::new(crate::broadcaster::SpvBroadcaster::new(spv)),
        Arc::new(std::sync::atomic::AtomicBool::new(false)),
        crate::changeset::DurableCursors::default(),
    );
    let id = Identifier::from([0x66; 32]);
    {
        let mut wm = wallet.wallet_manager.write().await;
        wm.get_wallet_info_mut(&wallet.wallet_id())
            .unwrap()
            .identity_manager
            .add_identity(
                Identity::V0(IdentityV0 {
                    id,
                    public_keys: seed_master_key(&seed[..], 2),
                    balance: 0,
                    revision: 0,
                }),
                2,
                wallet.wallet_id(),
                wallet.persister(),
            )
            .unwrap();
    }
    let coordinator = coordinator_at(&temp_dir(tag));
    wallet
        .bind_shielded(&seed[..], &[0], &coordinator)
        .await
        .unwrap();
    (wallet, *seed, id, coordinator)
}

/// Pause tip preparation right after its account snapshot, run `writer`
/// against the same wallet, and require the writer to wait for preparation.
async fn race_tip_preparation<F, Fut>(
    wallet: &PlatformWallet,
    seed: [u8; 64],
    id: dpp::prelude::Identifier,
    coordinator: &Arc<crate::wallet::shielded::NetworkShieldedCoordinator>,
    writer: F,
) where
    F: FnOnce(PlatformWallet) -> Fut,
    Fut: std::future::Future<Output = Result<(), crate::PlatformWalletError>> + Send + 'static,
{
    let (reached, resume) = super::tips::test_hooks::pause_after_snapshot(wallet);
    let preparing = {
        let wallet = wallet.clone();
        let coordinator = Arc::clone(coordinator);
        tokio::spawn(async move {
            wallet
                .prepare_shielded_identity_address(&seed[..], &id, &coordinator)
                .await
        })
    };
    reached.await.expect("preparation reached its snapshot");
    let mut writing = tokio::spawn(writer(wallet.clone()));
    // Unserialized, the writer finishes now and the resumed preparation then
    // rebinds its stale {0} plus the tip account, dropping account 1.
    //
    // The callers run on a paused clock, which is load-bearing: the timeout
    // fires only once nothing else is runnable, so it reads "the runtime had
    // no work left and the writer still had not finished". On a real clock a
    // writer that is merely slow to be scheduled passes it too, with the
    // lock removed.
    let finished_early =
        tokio::time::timeout(std::time::Duration::from_millis(500), &mut writing).await;
    assert!(
        finished_early.is_err(),
        "an account-set writer must wait for tip preparation's read-modify-write"
    );
    resume.send(()).unwrap();
    preparing.await.unwrap().expect("tip address prepared");
    writing.await.unwrap().expect("account-set writer succeeds");
}

/// Tip preparation reads the bound account set and binds it plus the tip
/// account. A bind of ordinary accounts landing between those two steps must
/// not be undone by the stale snapshot: preparation is paused right after its
/// snapshot while another task binds accounts 0 and 1, and account 1 must
/// still be bound and registered once both finish.
#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn should_keep_an_ordinary_account_bound_during_tip_preparation() {
    use crate::shielded_identity_account_index;
    let (wallet, seed, id, coordinator) = tip_race_fixture("tips_concurrent_bind").await;
    let tip = shielded_identity_account_index(2).unwrap();
    let bind_coordinator = Arc::clone(&coordinator);
    race_tip_preparation(&wallet, seed, id, &coordinator, move |wallet| async move {
        wallet
            .bind_shielded(&seed[..], &[0, 1], &bind_coordinator)
            .await
    })
    .await;

    let bound = wallet.shielded_account_indices().await;
    for account in [0, 1, tip] {
        assert!(
            bound.contains(&account),
            "account {account} bound: {bound:?}"
        );
    }
    let registered = coordinator.registered_subwallets().await;
    for account in [0, 1, tip] {
        assert!(
            registered.contains(&SubwalletId::new(wallet.wallet_id(), account)),
            "account {account} registered"
        );
    }
}

/// `shielded_add_account` writes the same account set, so it must not land
/// between preparation's snapshot and its bind either, or that bind replaces
/// the key map without the added account.
#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn should_keep_an_added_account_during_tip_preparation() {
    use crate::shielded_identity_account_index;
    let (wallet, seed, id, coordinator) = tip_race_fixture("tips_concurrent_add").await;
    let tip = shielded_identity_account_index(2).unwrap();
    race_tip_preparation(&wallet, seed, id, &coordinator, move |wallet| async move {
        wallet.shielded_add_account(&seed[..], 1).await
    })
    .await;

    let bound = wallet.shielded_account_indices().await;
    for account in [0, 1, tip] {
        assert!(
            bound.contains(&account),
            "account {account} bound: {bound:?}"
        );
    }
}

/// The identity account's keys are derived for the wallet's network, so a
/// coordinator of another network is refused before anything is bound,
/// stored or recorded as prepared.
#[tokio::test]
async fn should_refuse_tip_preparation_on_a_coordinator_of_another_network() {
    let (wallet, seed, id, _coordinator) = tip_race_fixture("tips_other_network").await;
    let mainnet = coordinator_for(&temp_dir("tips_other_network_mainnet"), Network::Mainnet);
    let bound_before = wallet.shielded_account_indices().await;

    let refused = wallet
        .prepare_shielded_identity_address(&seed, &id, &mainnet)
        .await;

    assert!(
        matches!(
            refused,
            Err(crate::PlatformWalletError::InvalidParameter(_))
        ),
        "{refused:?}"
    );
    assert_eq!(wallet.shielded_account_indices().await, bound_before);
    assert_eq!(wallet.prepared_shielded_identity_account(&id).await, None);
}

/// A bind derives its viewing keys for the wallet's network, so both bind
/// routes refuse a coordinator of another network before anything is read,
/// derived, stored or registered.
#[tokio::test]
async fn should_refuse_a_bind_on_a_coordinator_of_another_network() {
    let persister = Arc::new(CapturingPersistence::default());
    let wallet = platform_wallet_with(Arc::clone(&persister)).await;
    let mainnet = coordinator_for(&temp_dir("bind_other_network"), Network::Mainnet);

    let seeded = wallet.bind_shielded(&[0x42u8; 64], &[0], &mainnet).await;
    assert!(
        matches!(seeded, Err(crate::PlatformWalletError::InvalidParameter(_))),
        "{seeded:?}"
    );
    let seedless = wallet.bind_shielded_from_persisted(&[0], &mainnet).await;
    assert!(
        matches!(
            seedless,
            Err(crate::PlatformWalletError::InvalidParameter(_))
        ),
        "{seedless:?}"
    );

    assert!(wallet.shielded_account_indices().await.is_empty());
    assert!(mainnet.registered_subwallets().await.is_empty());
    assert!(persister.stored.lock().unwrap().is_empty());
    assert_eq!(*persister.load_calls.lock().unwrap(), 0);
}

/// The seed proof accepts only the identity's ECDSA MASTER authentication
/// key. The derived key's hash on a hash-only key, or the derived key itself
/// below MASTER, says nothing about the index the identity was created at.
#[tokio::test]
async fn should_refuse_tip_preparation_when_the_derived_key_is_not_the_master_key() {
    use dpp::identity::identity_public_key::methods::hash::IdentityPublicKeyHashMethodsV0;
    use dpp::identity::{Identity, IdentityPublicKey, IdentityV0, KeyType, SecurityLevel};
    use dpp::platform_value::BinaryData;
    use dpp::prelude::Identifier;
    let (wallet, seed, _, coordinator) = tip_race_fixture("tips_not_master_key").await;

    // Index 3: the derived key's hash, carried by a hash-only key.
    let mut hash_only = seed_master_key(&seed[..], 3);
    let derived_hash = hash_only[&0].public_key_hash().unwrap();
    let Some(IdentityPublicKey::V0(key)) = hash_only.get_mut(&0) else {
        panic!("the helper builds a V0 key");
    };
    key.key_type = KeyType::ECDSA_HASH160;
    key.data = BinaryData::new(derived_hash.to_vec());
    assert_eq!(hash_only[&0].public_key_hash().unwrap(), derived_hash);
    // Index 4: the derived key itself, below MASTER.
    let mut below_master = seed_master_key(&seed[..], 4);
    let Some(IdentityPublicKey::V0(key)) = below_master.get_mut(&0) else {
        panic!("the helper builds a V0 key");
    };
    key.security_level = SecurityLevel::HIGH;

    for (id, index, public_keys) in [
        (Identifier::from([0x71; 32]), 3, hash_only),
        (Identifier::from([0x72; 32]), 4, below_master),
    ] {
        {
            let mut wm = wallet.wallet_manager.write().await;
            wm.get_wallet_info_mut(&wallet.wallet_id())
                .unwrap()
                .identity_manager
                .add_identity(
                    Identity::V0(IdentityV0 {
                        id,
                        public_keys,
                        balance: 0,
                        revision: 0,
                    }),
                    index,
                    wallet.wallet_id(),
                    wallet.persister(),
                )
                .unwrap();
        }
        let refused = wallet
            .prepare_shielded_identity_address(&seed, &id, &coordinator)
            .await;
        assert!(
            matches!(refused, Err(crate::PlatformWalletError::InvalidIdentityData(ref msg)) if msg.contains("not verified")),
            "index {index}: {refused:?}"
        );
        assert_eq!(wallet.prepared_shielded_identity_account(&id).await, None);
    }
}

/// A Clear that is admitted while a preparation waits for admission wins. The
/// host wipes its own rows only after that Clear returns, so a preparation
/// admitted right behind it would bind from the rows the Clear removed: it is
/// refused and nothing is prepared.
#[tokio::test]
async fn should_refuse_a_tip_preparation_admitted_behind_a_clear() {
    let (wallet, seed, id, coordinator) = tip_race_fixture("tips_behind_clear").await;
    // Admission is held, so the Clear queues for it and the preparation
    // queues behind the Clear (the lock is fair: a waiting writer goes
    // before a later reader). Releasing it admits the Clear first.
    let admission = coordinator.operation_guard().await;
    let (cleared, prepared, ()) = tokio::join!(
        coordinator.clear(),
        wallet.prepare_shielded_identity_address(&seed, &id, &coordinator),
        async move { drop(admission) },
    );

    cleared.unwrap();
    assert!(
        matches!(prepared, Err(crate::PlatformWalletError::ShieldedNotBound)),
        "{prepared:?}"
    );
    assert_eq!(wallet.prepared_shielded_identity_account(&id).await, None);
    assert!(coordinator.registered_subwallets().await.is_empty());
}

/// A Clear drops the coordinator's state for the account, so the preparation no
/// longer stands, and a rebind of the ordinary accounts does not bring it back: the
/// identity's account is proved and prepared again.
#[tokio::test]
async fn should_require_fresh_tip_preparation_after_clear_even_after_rebind() {
    let (wallet, seed, id, coordinator) = tip_race_fixture("tips_clear").await;
    let address = wallet
        .prepare_shielded_identity_address(&seed, &id, &coordinator)
        .await
        .unwrap();
    let account = wallet
        .prepared_shielded_identity_account(&id)
        .await
        .expect("the preparation stands");
    assert_eq!(
        wallet.shielded_default_address(account).await,
        Some(address)
    );

    coordinator.clear().await.unwrap();
    assert_eq!(wallet.prepared_shielded_identity_account(&id).await, None);
    wallet
        .bind_shielded(&seed, &[0], &coordinator)
        .await
        .unwrap();
    assert_eq!(wallet.prepared_shielded_identity_account(&id).await, None);

    assert_eq!(
        wallet
            .prepare_shielded_identity_address(&seed, &id, &coordinator)
            .await
            .unwrap(),
        address
    );
    assert_eq!(
        wallet.prepared_shielded_identity_account(&id).await,
        Some(account)
    );
}

/// Move the fixture's identity from slot 2 to slot 3, as its removal and a
/// rediscovery at another index do. It then carries the MASTER key the seed
/// derives at each of the two slots, so the seed proves either one.
async fn move_identity_to_slot_three(
    wallet: &PlatformWallet,
    seed: &[u8],
    id: &dpp::prelude::Identifier,
) {
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeySettersV0;
    let mut second_master = seed_master_key(seed, 3)
        .remove(&0)
        .expect("the master key derived at slot 3");
    second_master.set_id(1);
    let mut wm = wallet.wallet_manager.write().await;
    let manager = &mut wm
        .get_wallet_info_mut(&wallet.wallet_id())
        .unwrap()
        .identity_manager;
    let mut identity = manager.remove_identity(id, wallet.persister()).unwrap();
    identity.add_public_key(second_master);
    manager
        .add_identity(identity, 3, wallet.wallet_id(), wallet.persister())
        .unwrap();
}

/// The preparation proves the identity's slot, releases the wallet-manager
/// lock, and binds the account that slot selects. An identity moved to
/// another slot in between has another account, so the preparation must
/// neither hand out nor record the address of the one it bound.
#[tokio::test]
async fn should_refuse_a_tip_preparation_whose_identity_moved_while_it_bound() {
    use crate::shielded_identity_account_index;
    let (wallet, seed, id, coordinator) = tip_race_fixture("tips_identity_moved").await;
    let (reached, resume) = super::tips::test_hooks::pause_after_snapshot(&wallet);
    let preparing = {
        let wallet = wallet.clone();
        let coordinator = Arc::clone(&coordinator);
        tokio::spawn(async move {
            wallet
                .prepare_shielded_identity_address(&seed[..], &id, &coordinator)
                .await
        })
    };
    reached.await.expect("preparation reached its snapshot");
    move_identity_to_slot_three(&wallet, &seed, &id).await;
    resume.send(()).unwrap();

    let refused = preparing.await.unwrap();
    assert!(
        matches!(refused, Err(crate::PlatformWalletError::InvalidIdentityData(ref msg)) if msg.contains("moved to another slot")),
        "{refused:?}"
    );
    assert_eq!(wallet.prepared_shielded_identity_account(&id).await, None);

    // Prepared again, the identity gets the account of the slot it holds.
    let address = wallet
        .prepare_shielded_identity_address(&seed, &id, &coordinator)
        .await
        .unwrap();
    let account = shielded_identity_account_index(3).unwrap();
    assert_eq!(
        wallet.shielded_default_address(account).await,
        Some(address)
    );
    assert_eq!(
        wallet.prepared_shielded_identity_account(&id).await,
        Some(account)
    );
}

/// A preparation stands for the slot the identity held when it was made.
/// Moved to another slot, the identity has another account, and the old
/// one must not be reported as prepared for it; nor for an identity the
/// wallet no longer holds.
#[tokio::test]
async fn should_not_report_a_prepared_account_for_an_identity_that_left_its_slot() {
    use crate::shielded_identity_account_index;
    let (wallet, seed, id, coordinator) = tip_race_fixture("tips_identity_left_slot").await;
    wallet
        .prepare_shielded_identity_address(&seed, &id, &coordinator)
        .await
        .unwrap();
    assert_eq!(
        wallet.prepared_shielded_identity_account(&id).await,
        Some(shielded_identity_account_index(2).unwrap())
    );

    move_identity_to_slot_three(&wallet, &seed, &id).await;
    assert_eq!(wallet.prepared_shielded_identity_account(&id).await, None);

    wallet
        .wallet_manager
        .write()
        .await
        .get_wallet_info_mut(&wallet.wallet_id())
        .unwrap()
        .identity_manager
        .remove_identity(&id, wallet.persister())
        .unwrap();
    assert_eq!(wallet.prepared_shielded_identity_account(&id).await, None);
}

/// A removed wallet is no longer registered or persisted, so a handle that
/// outlives the removal must not report an account as prepared.
#[tokio::test]
async fn should_drop_prepared_accounts_when_the_wallet_is_removed() {
    let (wallet, seed, id, coordinator) = tip_race_fixture("tips_wallet_removed").await;
    wallet
        .prepare_shielded_identity_address(&seed, &id, &coordinator)
        .await
        .unwrap();
    assert!(wallet
        .prepared_shielded_identity_account(&id)
        .await
        .is_some());

    let teardown = wallet.generation().teardown_guard().await;
    finish_removal(&wallet, &coordinator, teardown).await;
    assert_eq!(wallet.prepared_shielded_identity_account(&id).await, None);
}
