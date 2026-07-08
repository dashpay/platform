//! End-to-end coverage of `PlatformWalletManager::load_from_persistor`, the
//! seedless / watch-only load path, through a real `PlatformWalletManager`.
//!
//! Load reconstructs every persisted wallet **watch-only** from its keyless
//! account manifest. The load path never touches the seed, so it performs no
//! wrong-seed check; wrong-seed validation lives in the resolver-backed signing
//! entrypoints, not here. Per-row decode failures surface as
//! [`SkipReason::CorruptPersistedRow`] without aborting the batch.
//!
//! RT cases here:
//! - RT-WO: round-trip — watch-only wallet is registered after reload.
//! - RT-Corrupt: a row with an empty manifest is skipped with
//!   `MissingManifest`, the other row loads, `on_wallet_skipped_on_load`
//!   fires on the registered handler, `load` returns `Ok`.
//! - RT-Z: no key/seed material in any `LoadOutcome` / `SkipReason`
//!   surface (the structural-only contract).
//! - RT-Snapshot: a carried `wallet_info` snapshot is consumed
//!   verbatim — per-account UTXO attribution and derived-but-unused
//!   deep pool addresses survive the reload; a snapshot whose
//!   `wallet_id` mismatches its row is skipped as corrupt.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use dpp::prelude::Identifier;
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::wallet::Wallet;
use platform_wallet::changeset::{
    AccountRegistrationEntry, ClientStartState, ClientWalletStartState, IdentityManagerStartState,
    PersistenceError, PersistenceErrorKind, PlatformWalletChangeSet, PlatformWalletPersistence,
};
use platform_wallet::error::PlatformWalletError;
use platform_wallet::events::{EventHandler, PlatformEventHandler};
use platform_wallet::manager::load_outcome::CorruptKind;
use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet::ManagedIdentity;
use platform_wallet::{LoadOutcome, PlatformWalletManager, SkipReason};

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
                            wallet_info: w.wallet_info.clone(),
                            identity_manager: Default::default(),
                            unused_asset_locks: Default::default(),
                        },
                    );
                }
                out.skipped = s.skipped.clone();
                Ok(out)
            }
        }
    }
}

/// Persister whose `load()` always fails with a chosen [`PersistenceError`],
/// to exercise the typed error propagation out of `load_from_persistor`.
struct FailingLoadPersister {
    transient: bool,
}

impl PlatformWalletPersistence for FailingLoadPersister {
    fn store(&self, _: WalletId, _: PlatformWalletChangeSet) -> Result<(), PersistenceError> {
        Ok(())
    }
    fn flush(&self, _: WalletId) -> Result<(), PersistenceError> {
        Ok(())
    }
    fn load(&self) -> Result<ClientStartState, PersistenceError> {
        if self.transient {
            Err(PersistenceError::backend_with_kind(
                PersistenceErrorKind::Transient,
                "backend busy",
            ))
        } else {
            Err(PersistenceError::backend("schema corrupt"))
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
    let wallet = Wallet::from_seed_bytes(
        seed,
        key_wallet::Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .unwrap();
    let id = wallet.compute_wallet_id();
    let account_manifest = wallet
        .accounts
        .all_accounts()
        .into_iter()
        .map(|a| AccountRegistrationEntry {
            account_type: a.account_type,
            account_xpub: a.account_xpub,
        })
        .collect();
    let info = key_wallet::wallet::managed_wallet_info::ManagedWalletInfo::from_wallet(&wallet, 1);
    (
        id,
        ClientWalletStartState {
            network: key_wallet::Network::Testnet,
            birth_height: 1,
            account_manifest,
            wallet_info: Box::new(info),
            identity_manager: Default::default(),
            unused_asset_locks: Default::default(),
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

    assert_eq!(outcome.loaded(), vec![id].as_slice());
    assert!(outcome.skipped().is_empty());
    assert!(
        mgr.get_wallet(&id).await.is_some(),
        "watch-only restored wallet must be registered"
    );
    assert_eq!(mgr.wallet_ids().await, vec![id]);
}

/// RT-Idem: a second `load_from_persistor` with the wallet already
/// registered (a repeat restore, or a wallet created at runtime) must be
/// idempotent — never a hard error. The already-present wallet is
/// reported as an `AlreadyRegistered` skip (not a fresh load), so the
/// caller can tell it apart from one this pass genuinely loaded.
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
    assert_eq!(first.loaded(), vec![id].as_slice());
    assert!(first.skipped().is_empty());
    assert!(
        matches!(first, LoadOutcome::Loaded { .. }),
        "a clean first load is the Loaded variant"
    );

    // Second load: the wallet is already registered. Must NOT hard-error;
    // the row is reported as an AlreadyRegistered skip, not freshly loaded.
    let second = mgr
        .load_from_persistor()
        .await
        .expect("repeat load must be idempotent, not a hard error");
    assert!(
        !second.loaded().contains(&id),
        "an already-registered wallet is not reported as freshly loaded"
    );
    assert_eq!(
        second.skipped(),
        [(id, SkipReason::AlreadyRegistered)].as_slice(),
        "the already-present wallet surfaces as an AlreadyRegistered skip"
    );
    assert!(
        matches!(second, LoadOutcome::NoneUsable { .. }),
        "nothing freshly loaded on the repeat pass"
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
        outcome.loaded().contains(&id_ok),
        "healthy wallet still loads"
    );
    assert!(!outcome.loaded().contains(&bad_id));
    assert_eq!(outcome.skipped().len(), 1, "the rejected row surfaces once");
    assert_eq!(outcome.skipped()[0], (bad_id, reason.clone()));
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
    let (id_b, mut sb_corrupt) = slice(seed_b);

    // B's row is structurally corrupt — empty manifest. The empty manifest
    // aborts the watch-only rebuild before the snapshot is ever consulted.
    sb_corrupt.account_manifest = Vec::new();

    let mut st = ClientStartState::default();
    st.wallets.insert(id_a, sa);
    st.wallets.insert(id_b, sb_corrupt);
    p.set(st);

    let mgr = manager(Arc::clone(&p), Arc::clone(&h)).await;
    let outcome = mgr
        .load_from_persistor()
        .await
        .expect("Ok despite per-row skip");

    assert!(outcome.loaded().contains(&id_a), "A loads fully");
    assert!(
        !outcome.loaded().contains(&id_b),
        "B is skipped, not loaded"
    );
    assert_eq!(outcome.skipped().len(), 1);
    let (skipped_id, skipped_reason) = &outcome.skipped()[0];
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
    let (id, mut corrupt) = slice(seed);

    // Corrupt row to force a skip and inspect every public surface.
    corrupt.account_manifest = Vec::new();
    let mut st = ClientStartState::default();
    st.wallets.insert(id, corrupt);
    p.set(st);

    let mgr = manager(Arc::clone(&p), Arc::clone(&h)).await;
    let outcome = mgr.load_from_persistor().await.expect("Ok");
    let dbg = format!("{outcome:?}");
    // 0xAB seed bytes must not appear hex-rendered anywhere.
    assert!(!dbg.to_lowercase().contains(&"ab".repeat(10)));
    // The structural skip reason renders without any row bytes.
    for (_, reason) in outcome.skipped() {
        let rendered = format!("{reason} {reason:?}");
        assert!(!rendered.to_lowercase().contains(&"ab".repeat(10)));
    }
}

/// RT-Snapshot: a carried `wallet_info` snapshot is consumed
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
    s.wallet_info = Box::new(info);
    let p = Arc::new(FixedLoadPersister::new());
    let h = Arc::new(RecordingHandler::default());
    let mut st = ClientStartState::default();
    st.wallets.insert(id, s);
    p.set(st);

    let mgr = manager(Arc::clone(&p), Arc::clone(&h)).await;
    let outcome = mgr.load_from_persistor().await.expect("Ok");
    assert_eq!(outcome.loaded(), vec![id].as_slice());
    assert!(outcome.skipped().is_empty());

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
    s.wallet_info = Box::new(ManagedWalletInfo::from_wallet(&wallet_b, 1));

    let mut st = ClientStartState::default();
    st.wallets.insert(id_a, s);
    p.set(st);

    let mgr = manager(Arc::clone(&p), Arc::clone(&h)).await;
    let outcome = mgr.load_from_persistor().await.expect("Ok");

    assert!(outcome.loaded().is_empty(), "mismatched row must not load");
    assert_eq!(outcome.skipped().len(), 1);
    let (skipped_id, reason) = &outcome.skipped()[0];
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
    s.wallet_info = Box::new(ManagedWalletInfo::from_wallet(&wallet_a, 1));

    let mut st = ClientStartState::default();
    st.wallets.insert(id_a, s);
    p.set(st);

    let mgr = manager(Arc::clone(&p), Arc::clone(&h)).await;
    let outcome = mgr.load_from_persistor().await.expect("Ok");

    assert!(
        outcome.loaded().is_empty(),
        "account-set mismatch must not load"
    );
    assert_eq!(outcome.skipped().len(), 1);
    let (skipped_id, reason) = &outcome.skipped()[0];
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

/// RT-Snapshot-Mismatch-Combined: a snapshot-identity-mismatch skip and a
/// healthy snapshot load in the SAME batch. The mismatched row is skipped
/// with `SnapshotIdentityMismatch`; the healthy row loads fully; the batch
/// returns `Ok` and notifies the handler exactly once. Mirrors
/// `rt_corrupt_row_skipped_and_other_loads` for the snapshot path.
#[tokio::test]
async fn rt_snapshot_mismatch_skip_coexists_with_healthy_load() {
    use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;

    let seed_ok = [0x81; 64];
    let seed_bad = [0x82; 64];
    let seed_other = [0x83; 64];
    let p = Arc::new(FixedLoadPersister::new());
    let h = Arc::new(RecordingHandler::default());

    // Healthy row: snapshot built from its own wallet, matching its row.
    let wallet_ok = Wallet::from_seed_bytes(
        seed_ok,
        key_wallet::Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .unwrap();
    let id_ok = wallet_ok.compute_wallet_id();
    let (_, mut s_ok) = slice(seed_ok);
    s_ok.wallet_info = Box::new(ManagedWalletInfo::from_wallet(&wallet_ok, 1));

    // Mismatched row: keyed by wallet BAD, snapshot built from wallet OTHER.
    let (id_bad, mut s_bad) = slice(seed_bad);
    let wallet_other = Wallet::from_seed_bytes(
        seed_other,
        key_wallet::Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .unwrap();
    s_bad.wallet_info = Box::new(ManagedWalletInfo::from_wallet(&wallet_other, 1));

    let mut st = ClientStartState::default();
    st.wallets.insert(id_ok, s_ok);
    st.wallets.insert(id_bad, s_bad);
    p.set(st);

    let mgr = manager(Arc::clone(&p), Arc::clone(&h)).await;
    let outcome = mgr
        .load_from_persistor()
        .await
        .expect("Ok despite the per-row snapshot mismatch");

    assert_eq!(
        outcome.loaded(),
        vec![id_ok].as_slice(),
        "only the healthy row loads"
    );
    assert_eq!(outcome.skipped().len(), 1);
    let (skipped_id, reason) = &outcome.skipped()[0];
    assert_eq!(*skipped_id, id_bad);
    assert!(matches!(
        reason,
        SkipReason::CorruptPersistedRow {
            kind: CorruptKind::SnapshotIdentityMismatch
        }
    ));
    assert!(mgr.get_wallet(&id_ok).await.is_some());
    assert!(
        mgr.get_wallet(&id_bad).await.is_none(),
        "mismatched row must be absent, not a degraded placeholder"
    );

    let skipped = h.skipped.lock().unwrap();
    assert_eq!(skipped.len(), 1, "exactly one skip notification");
    assert_eq!(skipped[0].0, id_bad);
}

/// RT-PersisterLoad-Transient: a transient persister load failure
/// propagates as a typed `PersisterLoad` error whose retry classification
/// survives — `is_transient()` is `true` so callers may back off and retry.
#[tokio::test]
async fn rt_persister_load_transient_error_is_typed_and_retryable() {
    let p = Arc::new(FailingLoadPersister { transient: true });
    let h = Arc::new(RecordingHandler::default());
    let sdk = Arc::new(dash_sdk::Sdk::new_mock());
    let mgr = Arc::new(PlatformWalletManager::new(sdk, Arc::clone(&p), h));

    let err = mgr
        .load_from_persistor()
        .await
        .expect_err("transient backend failure must surface");
    match err {
        PlatformWalletError::PersisterLoad(inner) => {
            assert!(
                inner.is_transient(),
                "transient classification must survive propagation"
            );
            assert_eq!(inner.kind(), Some(PersistenceErrorKind::Transient));
        }
        other => panic!("expected PersisterLoad, got {other:?}"),
    }
}

/// RT-PersisterLoad-Permanent: a fatal persister load failure propagates as
/// a typed `PersisterLoad` error classified non-transient, so callers do
/// not retry a permanent failure.
#[tokio::test]
async fn rt_persister_load_permanent_error_is_typed_and_not_retryable() {
    let p = Arc::new(FailingLoadPersister { transient: false });
    let h = Arc::new(RecordingHandler::default());
    let sdk = Arc::new(dash_sdk::Sdk::new_mock());
    let mgr = Arc::new(PlatformWalletManager::new(sdk, Arc::clone(&p), h));

    let err = mgr
        .load_from_persistor()
        .await
        .expect_err("fatal backend failure must surface");
    match err {
        PlatformWalletError::PersisterLoad(inner) => {
            assert!(
                !inner.is_transient(),
                "fatal failure must not read as retryable"
            );
            assert_eq!(inner.kind(), Some(PersistenceErrorKind::Fatal));
        }
        other => panic!("expected PersisterLoad, got {other:?}"),
    }
}

/// RT-EmptyFirstRun: a genuinely fresh install — the persister returns
/// `ClientStartState::default()` with no rows at all — loads cleanly as
/// `LoadOutcome::Loaded` with empty loaded/skipped and fires zero skip
/// handlers. This is the condition every first launch hits.
#[tokio::test]
async fn rt_empty_first_run_is_clean_with_no_handler_calls() {
    // `FixedLoadPersister::new()` without `set()` returns the default
    // (empty) ClientStartState from `load()`.
    let p = Arc::new(FixedLoadPersister::new());
    let h = Arc::new(RecordingHandler::default());

    let mgr = manager(Arc::clone(&p), Arc::clone(&h)).await;
    let outcome = mgr
        .load_from_persistor()
        .await
        .expect("an empty store must load cleanly");

    assert!(
        matches!(outcome, LoadOutcome::Loaded { .. }),
        "an empty store is a full (Loaded) outcome, not partial/none-usable"
    );
    assert!(
        outcome.loaded().is_empty(),
        "nothing to load on a fresh install"
    );
    assert!(
        outcome.skipped().is_empty(),
        "nothing to skip on a fresh install"
    );
    assert!(mgr.wallet_ids().await.is_empty(), "no wallets registered");
    assert!(
        h.skipped.lock().unwrap().is_empty(),
        "no skip handler fires on a fresh install"
    );
}

/// RT-Concurrent: two concurrent `load_from_persistor` passes against the
/// same manager and multi-wallet snapshot. Both must complete without
/// error, every wallet must register exactly once (the idempotency check
/// releases its read lock before the write-lock insert — a check-then-act
/// window), and the outcomes must be consistent: each wallet is freshly
/// loaded by exactly one pass and reported `AlreadyRegistered` by the
/// other (never a corruption skip, never a hard error).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rt_concurrent_loads_register_each_wallet_exactly_once() {
    let p = Arc::new(FixedLoadPersister::new());
    let h = Arc::new(RecordingHandler::default());
    let (id1, s1) = slice([0xC1; 64]);
    let (id2, s2) = slice([0xC2; 64]);
    let mut st = ClientStartState::default();
    st.wallets.insert(id1, s1);
    st.wallets.insert(id2, s2);
    p.set(st);

    let mgr = manager(Arc::clone(&p), Arc::clone(&h)).await;
    let m1 = Arc::clone(&mgr);
    let m2 = Arc::clone(&mgr);
    let t1 = tokio::spawn(async move { m1.load_from_persistor().await });
    let t2 = tokio::spawn(async move { m2.load_from_persistor().await });
    let o1 = t1.await.unwrap().expect("first concurrent load Ok");
    let o2 = t2.await.unwrap().expect("second concurrent load Ok");

    // Each wallet registered exactly once, in both maps.
    assert!(mgr.get_wallet(&id1).await.is_some());
    assert!(mgr.get_wallet(&id2).await.is_some());
    assert_eq!(
        mgr.wallet_ids().await.len(),
        2,
        "each wallet registered exactly once despite the concurrent passes"
    );

    // Every wallet is freshly loaded by exactly one of the two passes.
    let mut loaded_union: std::collections::BTreeSet<WalletId> =
        o1.loaded().iter().copied().collect();
    for id in o2.loaded() {
        assert!(
            loaded_union.insert(*id),
            "a wallet must be freshly loaded by at most one pass"
        );
    }
    assert!(
        loaded_union.contains(&id1) && loaded_union.contains(&id2),
        "both wallets loaded across the two passes"
    );

    // Any overlap surfaces as an AlreadyRegistered skip, never corruption.
    for outcome in [&o1, &o2] {
        for (_, reason) in outcome.skipped() {
            assert_eq!(
                *reason,
                SkipReason::AlreadyRegistered,
                "a concurrent overlap is an AlreadyRegistered skip, not a corrupt row"
            );
        }
    }
}

/// Persister for TC-3692-011. Rebuilds — fresh on each `load()`, since the
/// identity-manager snapshot is intentionally not `Clone` — a
/// `ClientStartState` carrying one wallet-owned identity whose
/// `Identity.public_keys` already holds a CRITICAL / AUTHENTICATION /
/// ECDSA_SECP256K1 key, the shape the FFI/iOS path produces via
/// `build_wallet_identity_bucket` (never through an `IdentityKeysChangeSet`).
struct IdentityKeyedLoadPersister {
    seed: [u8; 64],
    identity_id: Identifier,
}

impl PlatformWalletPersistence for IdentityKeyedLoadPersister {
    fn store(&self, _: WalletId, _: PlatformWalletChangeSet) -> Result<(), PersistenceError> {
        Ok(())
    }
    fn flush(&self, _: WalletId) -> Result<(), PersistenceError> {
        Ok(())
    }
    fn load(&self) -> Result<ClientStartState, PersistenceError> {
        use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
        use dpp::identity::identity_public_key::{IdentityPublicKey, Purpose};
        use dpp::identity::v0::IdentityV0;
        use dpp::identity::{Identity, KeyType, SecurityLevel};

        let (id, mut s) = slice(self.seed);

        let key = IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: 0,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::CRITICAL,
            contract_bounds: None,
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: dpp::platform_value::BinaryData::new(vec![2u8; 33]),
            disabled_at: None,
        });
        let mut public_keys = BTreeMap::new();
        public_keys.insert(0, key);
        let identity = Identity::V0(IdentityV0 {
            id: self.identity_id,
            public_keys,
            balance: 0,
            revision: 0,
        });

        let mut inner = BTreeMap::new();
        inner.insert(0u32, ManagedIdentity::new(identity, 0));
        let mut ims = IdentityManagerStartState::default();
        ims.wallet_identities.insert(id, inner);
        s.identity_manager = ims;

        let mut st = ClientStartState::default();
        st.wallets.insert(id, s);
        Ok(st)
    }
}

/// TC-3692-011 [CRITICAL]: the restored identity's signing key is
/// selectable via the exact `get_first_public_key_matching` predicate the
/// real signing path uses (`contract.rs`, `dpns.rs`, ...) immediately
/// after `load_from_persistor`, with zero sync. The FFI/iOS path populates
/// `Identity.public_keys` at construction, so dropping
/// `ClientWalletStartState.identity_keys` must not change this — the key
/// must sit in the exact map slot the lookup reads, with no reliance on
/// `apply_identity_key_entry` ever running on the load path.
#[tokio::test]
async fn rt_signing_key_selectable_immediately_after_load() {
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dpp::identity::identity_public_key::Purpose;
    use dpp::identity::{KeyType, SecurityLevel};

    let seed = [0x5A; 64];
    let identity_id = Identifier::from([0x11; 32]);
    let p = Arc::new(IdentityKeyedLoadPersister { seed, identity_id });
    let h = Arc::new(RecordingHandler::default());
    let sdk = Arc::new(dash_sdk::Sdk::new_mock());
    let mgr = Arc::new(PlatformWalletManager::new(sdk, Arc::clone(&p), h));

    let outcome = mgr.load_from_persistor().await.expect("Ok");
    assert!(
        outcome.skipped().is_empty(),
        "identity row must not be skipped"
    );
    let id = outcome
        .loaded()
        .first()
        .copied()
        .expect("exactly one wallet loaded");

    // Immediately — no SPV sync, no identity refresh — drive the exact
    // predicate a real signing flow uses to pick its authentication key.
    let wallet = mgr.get_wallet(&id).await.expect("wallet registered");
    let wm = wallet.wallet_manager().read().await;
    let info = wm.get_wallet_info(&id).expect("wallet info present");
    let managed = info
        .identity_manager
        .identity(&identity_id)
        .expect("identity restored into the manager");
    let selected = managed
        .identity
        .get_first_public_key_matching(
            Purpose::AUTHENTICATION,
            [SecurityLevel::CRITICAL].into(),
            [KeyType::ECDSA_SECP256K1].into(),
            false,
        )
        .expect("CRITICAL/AUTH/ECDSA signing key must be selectable post-load, zero sync");
    assert_eq!(selected.id(), 0, "the exact installed key is selected");
}
