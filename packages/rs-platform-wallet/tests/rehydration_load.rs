//! Item E — `load_from_persistor` (seedless / watch-only) end-to-end
//! through a real `PlatformWalletManager`.
//!
//! Scope after the seedless rework: load reconstructs every persisted
//! wallet **watch-only** from its keyless account manifest. The load
//! path never touches the seed, so it performs no wrong-seed check;
//! wrong-seed validation lives in the resolver-backed signing
//! entrypoints, not in this load path. Per-row decode failures surface
//! as [`SkipReason::CorruptPersistedRow`] without aborting the batch.
//!
//! RT cases here:
//! - RT-WO: round-trip — watch-only wallet is registered after reload.
//! - RT-Corrupt: a row with an empty manifest is skipped with
//!   `MissingManifest`, the other row loads, `on_wallet_skipped_on_load`
//!   fires on the registered handler, `load` returns `Ok`.
//! - RT-Z: no key/seed material in any `LoadOutcome` / `SkipReason`
//!   surface (the structural-only contract).
//! - RT-Snapshot: a carried `core_wallet_info` snapshot is consumed
//!   verbatim — per-account UTXO attribution and derived-but-unused
//!   deep pool addresses survive the reload; a snapshot whose
//!   `wallet_id` mismatches its row is skipped as corrupt.

use std::sync::{Arc, Mutex};

use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::wallet::Wallet;
use platform_wallet::changeset::{
    AccountRegistrationEntry, ClientStartState, ClientWalletStartState, CoreChangeSet,
    PersistenceError, PlatformWalletChangeSet, PlatformWalletPersistence,
};
use platform_wallet::events::{EventHandler, PlatformEventHandler};
use platform_wallet::manager::load_outcome::CorruptKind;
use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet::{PlatformWalletManager, SkipReason};

// ---- test doubles ----

/// Persister whose `load()` returns a fixed keyless `ClientStartState`.
struct FixedLoadPersister {
    state: Mutex<Option<ClientStartState>>,
}

impl FixedLoadPersister {
    fn new() -> Self {
        Self {
            state: Mutex::new(None),
        }
    }
    fn set(&self, s: ClientStartState) {
        *self.state.lock().unwrap() = Some(s);
    }
}

impl PlatformWalletPersistence for FixedLoadPersister {
    fn store(&self, _: WalletId, _: PlatformWalletChangeSet) -> Result<(), PersistenceError> {
        Ok(())
    }
    fn flush(&self, _: WalletId) -> Result<(), PersistenceError> {
        Ok(())
    }
    fn load(&self) -> Result<ClientStartState, PersistenceError> {
        // Rebuild a fresh ClientStartState each call (load may be
        // called twice for the recoverability sub-case).
        let guard = self.state.lock().unwrap();
        match guard.as_ref() {
            None => Ok(ClientStartState::default()),
            Some(s) => {
                let mut out = ClientStartState::default();
                for (id, w) in &s.wallets {
                    out.wallets.insert(
                        *id,
                        ClientWalletStartState {
                            network: w.network,
                            birth_height: w.birth_height,
                            account_manifest: w.account_manifest.clone(),
                            core_wallet_info: w.core_wallet_info.clone(),
                            core_state: w.core_state.clone(),
                            identity_manager: Default::default(),
                            unused_asset_locks: Default::default(),
                            contacts: Default::default(),
                            identity_keys: Default::default(),
                            used_core_addresses: w.used_core_addresses.clone(),
                        },
                    );
                }
                out.skipped = s.skipped.clone();
                Ok(out)
            }
        }
    }
}

/// Event handler that records every wallet-skipped-on-load notification.
#[derive(Default)]
struct RecordingHandler {
    skipped: Mutex<Vec<(WalletId, SkipReason)>>,
}
impl EventHandler for RecordingHandler {}
impl PlatformEventHandler for RecordingHandler {
    fn on_wallet_skipped_on_load(&self, wallet_id: WalletId, reason: &SkipReason) {
        self.skipped
            .lock()
            .unwrap()
            .push((wallet_id, reason.clone()));
    }
}

// ---- harness ----

fn manifest_and_id(seed: [u8; 64]) -> (Vec<AccountRegistrationEntry>, [u8; 32]) {
    let w = Wallet::from_seed_bytes(
        seed,
        key_wallet::Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .unwrap();
    let manifest = w
        .accounts
        .all_accounts()
        .into_iter()
        .map(|a| AccountRegistrationEntry {
            account_type: a.account_type,
            account_xpub: a.account_xpub,
        })
        .collect();
    (manifest, w.compute_wallet_id())
}

fn slice(seed: [u8; 64]) -> (WalletId, ClientWalletStartState) {
    let (manifest, id) = manifest_and_id(seed);
    (
        id,
        ClientWalletStartState {
            network: key_wallet::Network::Testnet,
            birth_height: 1,
            account_manifest: manifest,
            core_wallet_info: None,
            core_state: CoreChangeSet::default(),
            identity_manager: Default::default(),
            unused_asset_locks: Default::default(),
            contacts: Default::default(),
            identity_keys: Default::default(),
            used_core_addresses: Default::default(),
        },
    )
}

async fn manager(
    persister: Arc<FixedLoadPersister>,
    handler: Arc<RecordingHandler>,
) -> Arc<PlatformWalletManager<FixedLoadPersister>> {
    let sdk = Arc::new(dash_sdk::Sdk::new_mock());
    Arc::new(PlatformWalletManager::new(sdk, persister, handler))
}

// ---- tests ----

/// RT-WO: seedless watch-only round-trip — a persisted wallet loads and
/// is registered after reload (no signing material needed).
#[tokio::test]
async fn rt_wo_watch_only_roundtrip() {
    let seed = [0x11; 64];
    let p = Arc::new(FixedLoadPersister::new());
    let h = Arc::new(RecordingHandler::default());
    let (id, s) = slice(seed);
    let mut st = ClientStartState::default();
    st.wallets.insert(id, s);
    p.set(st);

    let mgr = manager(Arc::clone(&p), Arc::clone(&h)).await;
    let outcome = mgr.load_from_persistor().await.expect("Ok");

    assert_eq!(outcome.loaded, vec![id]);
    assert!(outcome.skipped.is_empty());
    assert!(
        mgr.get_wallet(&id).await.is_some(),
        "watch-only restored wallet must be registered"
    );
    assert_eq!(mgr.wallet_ids().await, vec![id]);
}

/// RT-Idem: a second `load_from_persistor` with the wallet already
/// registered (a repeat restore, or a wallet created at runtime) must be
/// idempotent. `WalletExists` from `insert_wallet` is treated as
/// already-satisfied — counted as loaded — not a fatal `WalletCreation`
/// that aborts the whole batch.
#[tokio::test]
async fn rt_idempotent_repeat_restore() {
    let seed = [0x55; 64];
    let p = Arc::new(FixedLoadPersister::new());
    let h = Arc::new(RecordingHandler::default());
    let (id, s) = slice(seed);
    let mut st = ClientStartState::default();
    st.wallets.insert(id, s);
    p.set(st);

    let mgr = manager(Arc::clone(&p), Arc::clone(&h)).await;

    let first = mgr.load_from_persistor().await.expect("first load Ok");
    assert_eq!(first.loaded, vec![id]);
    assert!(first.skipped.is_empty());

    // Second load: the wallet is already registered. Must NOT hard-error.
    let second = mgr
        .load_from_persistor()
        .await
        .expect("repeat load must be idempotent, not a hard error");
    assert!(
        second.loaded.contains(&id),
        "already-present wallet is reported loaded (already-satisfied)"
    );
    assert!(
        second.skipped.is_empty(),
        "an idempotent re-load is not a skip"
    );
    assert!(
        mgr.get_wallet(&id).await.is_some(),
        "wallet still present after the repeat load"
    );
    assert_eq!(mgr.wallet_ids().await, vec![id]);
}

/// RT-PersisterSkip: a wallet the persister itself rejected as corrupt
/// before reconstruction — surfaced via `ClientStartState::skipped` (e.g.
/// the FFI `load()` catching a malformed xpub per-row) — is folded into
/// `LoadOutcome::skipped` and fires `on_wallet_skipped_on_load`, while the
/// healthy wallet still loads. One bad persisted row never blocks the batch.
#[tokio::test]
async fn rt_persister_skipped_folds_into_outcome() {
    let seed_ok = [0x71; 64];
    let p = Arc::new(FixedLoadPersister::new());
    let h = Arc::new(RecordingHandler::default());
    let (id_ok, s_ok) = slice(seed_ok);

    // A wallet id the persister could not decode (fabricated skip).
    let bad_id: WalletId = [0x09; 32];
    let reason = SkipReason::CorruptPersistedRow {
        kind: CorruptKind::DecodeError("malformed account xpub".to_string()),
    };

    let mut st = ClientStartState::default();
    st.wallets.insert(id_ok, s_ok);
    st.skipped.push((bad_id, reason.clone()));
    p.set(st);

    let mgr = manager(Arc::clone(&p), Arc::clone(&h)).await;
    let outcome = mgr
        .load_from_persistor()
        .await
        .expect("Ok despite a persister-rejected row");

    assert!(
        outcome.loaded.contains(&id_ok),
        "healthy wallet still loads"
    );
    assert!(!outcome.loaded.contains(&bad_id));
    assert_eq!(outcome.skipped.len(), 1, "the rejected row surfaces once");
    assert_eq!(outcome.skipped[0], (bad_id, reason.clone()));
    assert!(mgr.get_wallet(&id_ok).await.is_some());
    assert!(
        mgr.get_wallet(&bad_id).await.is_none(),
        "the rejected row is never registered"
    );

    // The skip notification fired exactly once for the bad row.
    let skipped = h.skipped.lock().unwrap();
    assert_eq!(skipped.len(), 1, "exactly one skip notification");
    assert_eq!(skipped[0], (bad_id, reason));
}

/// RT-Corrupt: a corrupt row (empty manifest) is skipped with
/// `MissingManifest`; the other row loads cleanly; the load returns
/// `Ok`; `on_wallet_skipped_on_load` fires exactly once on the
/// registered handler for the skipped row.
#[tokio::test]
async fn rt_corrupt_row_skipped_and_other_loads() {
    let seed_a = [0x31; 64];
    let seed_b = [0x32; 64];
    let p = Arc::new(FixedLoadPersister::new());
    let h = Arc::new(RecordingHandler::default());
    let (id_a, sa) = slice(seed_a);
    let (id_b, _sb) = slice(seed_b);

    // B's row is structurally corrupt — empty manifest.
    let sb_corrupt = ClientWalletStartState {
        network: key_wallet::Network::Testnet,
        birth_height: 1,
        account_manifest: Vec::new(),
        core_wallet_info: None,
        core_state: CoreChangeSet::default(),
        identity_manager: Default::default(),
        unused_asset_locks: Default::default(),
        contacts: Default::default(),
        identity_keys: Default::default(),
        used_core_addresses: Default::default(),
    };

    let mut st = ClientStartState::default();
    st.wallets.insert(id_a, sa);
    st.wallets.insert(id_b, sb_corrupt);
    p.set(st);

    let mgr = manager(Arc::clone(&p), Arc::clone(&h)).await;
    let outcome = mgr
        .load_from_persistor()
        .await
        .expect("Ok despite per-row skip");

    assert!(outcome.loaded.contains(&id_a), "A loads fully");
    assert!(!outcome.loaded.contains(&id_b), "B is skipped, not loaded");
    assert_eq!(outcome.skipped.len(), 1);
    let (skipped_id, skipped_reason) = &outcome.skipped[0];
    assert_eq!(*skipped_id, id_b);
    assert!(matches!(
        skipped_reason,
        SkipReason::CorruptPersistedRow {
            kind: CorruptKind::MissingManifest
        }
    ));
    assert!(mgr.get_wallet(&id_a).await.is_some());
    assert!(
        mgr.get_wallet(&id_b).await.is_none(),
        "corrupt row must be ABSENT, not a degraded placeholder"
    );

    // Exactly one on_wallet_skipped_on_load notification for B.
    {
        let skipped = h.skipped.lock().unwrap();
        assert_eq!(skipped.len(), 1, "exactly one skip notification expected");
        let (skipped_wallet_id, skipped_reason) = &skipped[0];
        assert_eq!(*skipped_wallet_id, id_b);
        assert!(matches!(
            skipped_reason,
            SkipReason::CorruptPersistedRow {
                kind: CorruptKind::MissingManifest
            }
        ));
    }
}

/// RT-Z: no key/seed material leaks into `LoadOutcome` /
/// `SkipReason::CorruptPersistedRow` surfaces. The seedless load path
/// never sees seed bytes so this is mostly a sentinel guard against
/// future regression where someone embeds row contents in `DecodeError`.
#[tokio::test]
async fn rt_z_secret_hygiene_surfaces() {
    let seed = [0xAB; 64];
    let p = Arc::new(FixedLoadPersister::new());
    let h = Arc::new(RecordingHandler::default());
    let (id, _s) = slice(seed);

    // Corrupt row to force a skip and inspect every public surface.
    let corrupt = ClientWalletStartState {
        network: key_wallet::Network::Testnet,
        birth_height: 1,
        account_manifest: Vec::new(),
        core_wallet_info: None,
        core_state: CoreChangeSet::default(),
        identity_manager: Default::default(),
        unused_asset_locks: Default::default(),
        contacts: Default::default(),
        identity_keys: Default::default(),
        used_core_addresses: Default::default(),
    };
    let mut st = ClientStartState::default();
    st.wallets.insert(id, corrupt);
    p.set(st);

    let mgr = manager(Arc::clone(&p), Arc::clone(&h)).await;
    let outcome = mgr.load_from_persistor().await.expect("Ok");
    let dbg = format!("{outcome:?}");
    // 0xAB seed bytes must not appear hex-rendered anywhere.
    assert!(!dbg.to_lowercase().contains(&"ab".repeat(10)));
    // The structural skip reason renders without any row bytes.
    for (_, reason) in &outcome.skipped {
        let rendered = format!("{reason} {reason:?}");
        assert!(!rendered.to_lowercase().contains(&"ab".repeat(10)));
    }
}

/// RT-Snapshot: a carried `core_wallet_info` snapshot is consumed
/// verbatim. Two properties the projection replay could NOT provide:
/// - per-account UTXO attribution — a CoinJoin-account UTXO stays on the
///   CoinJoin account (the fallback path routed every UTXO to the first
///   funds account, zeroing non-first-account balances);
/// - derived-but-unused deep pool addresses (idx 40, past the eager gap
///   window) stay in the pool, so the SPV watch set still covers a
///   handed-out-but-unpaid receive address after restart.
#[tokio::test]
async fn rt_snapshot_preserves_attribution_and_pools() {
    use key_wallet::account::AccountType;
    use key_wallet::managed_account::address_pool::KeySource;
    use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
    use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
    use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;

    let seed = [0x66; 64];
    let wallet = Wallet::from_seed_bytes(
        seed,
        key_wallet::Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .unwrap();
    let id = wallet.compute_wallet_id();
    let manifest: Vec<AccountRegistrationEntry> = wallet
        .accounts
        .all_accounts()
        .into_iter()
        .map(|a| AccountRegistrationEntry {
            account_type: a.account_type,
            account_xpub: a.account_xpub,
        })
        .collect();

    let mut info = ManagedWalletInfo::from_wallet(&wallet, 1);

    // A UTXO on the CoinJoin account's own idx-0 address, inserted where
    // the persisted rows put it: on the CoinJoin account.
    let cj_value = 250_000u64;
    let (cj_type, cj_addr) = {
        let cj = info
            .accounts
            .all_funding_accounts()
            .into_iter()
            .find(|a| {
                matches!(
                    a.managed_account_type().to_account_type(),
                    AccountType::CoinJoin { .. }
                )
            })
            .expect("Default creation includes a CoinJoin account");
        let addr = cj
            .managed_account_type()
            .address_pools()
            .first()
            .expect("CoinJoin account has a pool")
            .address_at_index(0)
            .expect("eager window covers idx 0");
        (cj.managed_account_type().to_account_type(), addr)
    };
    {
        let cj = info
            .accounts
            .all_funding_accounts_mut()
            .into_iter()
            .find(|a| a.managed_account_type().to_account_type() == cj_type)
            .unwrap();
        cj.utxos.insert(
            dashcore::OutPoint {
                txid: dashcore::Txid::from([0x42u8; 32]),
                vout: 0,
            },
            key_wallet::Utxo {
                outpoint: dashcore::OutPoint {
                    txid: dashcore::Txid::from([0x42u8; 32]),
                    vout: 0,
                },
                txout: dashcore::TxOut {
                    value: cj_value,
                    script_pubkey: cj_addr.script_pubkey(),
                },
                address: cj_addr,
                height: 1,
                is_coinbase: false,
                is_confirmed: true,
                is_instantlocked: false,
                is_locked: false,
                is_trusted: false,
            },
        );
    }

    // Extend the FIRST funds account's first pool to idx 40 — a
    // derived-but-UNUSED deep address (handed out, not yet paid).
    let (first_type, deep_keys_total) = {
        let first = info
            .accounts
            .all_funding_accounts_mut()
            .into_iter()
            .next()
            .expect("a first funds account exists");
        let first_type = first.managed_account_type().to_account_type();
        let xpub = manifest
            .iter()
            .find(|e| e.account_type == first_type)
            .map(|e| e.account_xpub)
            .expect("first funds account xpub in manifest");
        let pools = first.managed_account_type_mut().address_pools_mut();
        let pool = pools.into_iter().next().expect("first pool");
        let highest = pool.highest_generated.expect("eager window derived");
        assert!(
            highest < 40,
            "fixture: idx 40 must be past the eager window"
        );
        pool.generate_addresses(40 - highest, &KeySource::Public(xpub), true)
            .unwrap();
        assert!(
            pool.address_at_index(40).is_some(),
            "fixture: idx 40 derived"
        );
        (first_type, pool.addresses.len() as u32)
    };
    info.update_balance();

    let (_, mut s) = slice(seed);
    s.core_wallet_info = Some(Box::new(info));
    let p = Arc::new(FixedLoadPersister::new());
    let h = Arc::new(RecordingHandler::default());
    let mut st = ClientStartState::default();
    st.wallets.insert(id, s);
    p.set(st);

    let mgr = manager(Arc::clone(&p), Arc::clone(&h)).await;
    let outcome = mgr.load_from_persistor().await.expect("Ok");
    assert_eq!(outcome.loaded, vec![id]);
    assert!(outcome.skipped.is_empty());

    let rows = {
        let mgr = Arc::clone(&mgr);
        tokio::task::spawn_blocking(move || mgr.account_balances_blocking(&id))
            .await
            .unwrap()
    };
    let cj_row = rows
        .iter()
        .find(|r| r.account_type == cj_type)
        .expect("CoinJoin account row");
    assert_eq!(
        cj_row.balance.total(),
        cj_value,
        "CoinJoin UTXO must stay attributed to the CoinJoin account"
    );
    let first_row = rows
        .iter()
        .find(|r| r.account_type == first_type)
        .expect("first funds account row");
    assert!(
        first_row.keys_total >= deep_keys_total,
        "derived-but-unused deep addresses must survive the reload \
         (watch-set coverage): got {} keys, snapshot had {}",
        first_row.keys_total,
        deep_keys_total,
    );
}

/// RT-Snapshot-Mismatch: a snapshot whose `wallet_id` does not match its
/// row key is a corrupt row — skipped with `SnapshotIdentityMismatch`,
/// never registered, and the batch continues.
#[tokio::test]
async fn rt_snapshot_wallet_id_mismatch_is_skipped() {
    use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;

    let seed = [0x77; 64];
    let other_seed = [0x78; 64];
    let p = Arc::new(FixedLoadPersister::new());
    let h = Arc::new(RecordingHandler::default());

    // Row keyed by wallet A, snapshot built from wallet B.
    let (id_a, mut s) = slice(seed);
    let wallet_b = Wallet::from_seed_bytes(
        other_seed,
        key_wallet::Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .unwrap();
    s.core_wallet_info = Some(Box::new(ManagedWalletInfo::from_wallet(&wallet_b, 1)));

    let mut st = ClientStartState::default();
    st.wallets.insert(id_a, s);
    p.set(st);

    let mgr = manager(Arc::clone(&p), Arc::clone(&h)).await;
    let outcome = mgr.load_from_persistor().await.expect("Ok");

    assert!(outcome.loaded.is_empty(), "mismatched row must not load");
    assert_eq!(outcome.skipped.len(), 1);
    let (skipped_id, reason) = &outcome.skipped[0];
    assert_eq!(*skipped_id, id_a);
    assert!(matches!(
        reason,
        SkipReason::CorruptPersistedRow {
            kind: CorruptKind::SnapshotIdentityMismatch
        }
    ));
    assert!(mgr.get_wallet(&id_a).await.is_none());
    assert_eq!(h.skipped.lock().unwrap().len(), 1);
}

/// RT-Snapshot-AccountMismatch: a snapshot whose `wallet_id`/`network`
/// agree with the row but whose account set diverges from the row's
/// account manifest is a wrong-row snapshot — skipped with
/// `SnapshotIdentityMismatch`, never registered.
#[tokio::test]
async fn rt_snapshot_account_set_mismatch_is_skipped() {
    use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;

    let seed = [0x79; 64];
    let p = Arc::new(FixedLoadPersister::new());
    let h = Arc::new(RecordingHandler::default());

    // Row keyed by wallet A with a full snapshot of A, but the row's
    // manifest is truncated to a single account — the account sets diverge
    // even though wallet_id and network match.
    let wallet_a = Wallet::from_seed_bytes(
        seed,
        key_wallet::Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .unwrap();
    let id_a = wallet_a.compute_wallet_id();
    let (full_manifest, _) = manifest_and_id(seed);
    assert!(
        full_manifest.len() > 1,
        "fixture: Default creation yields more than one account"
    );
    let truncated_manifest = vec![full_manifest[0].clone()];

    let (_, mut s) = slice(seed);
    s.account_manifest = truncated_manifest;
    s.core_wallet_info = Some(Box::new(ManagedWalletInfo::from_wallet(&wallet_a, 1)));

    let mut st = ClientStartState::default();
    st.wallets.insert(id_a, s);
    p.set(st);

    let mgr = manager(Arc::clone(&p), Arc::clone(&h)).await;
    let outcome = mgr.load_from_persistor().await.expect("Ok");

    assert!(
        outcome.loaded.is_empty(),
        "account-set mismatch must not load"
    );
    assert_eq!(outcome.skipped.len(), 1);
    let (skipped_id, reason) = &outcome.skipped[0];
    assert_eq!(*skipped_id, id_a);
    assert!(matches!(
        reason,
        SkipReason::CorruptPersistedRow {
            kind: CorruptKind::SnapshotIdentityMismatch
        }
    ));
    assert!(mgr.get_wallet(&id_a).await.is_none());
    assert_eq!(h.skipped.lock().unwrap().len(), 1);
}
