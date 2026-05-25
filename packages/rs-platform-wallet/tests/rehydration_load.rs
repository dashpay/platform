//! Item E — `load_from_persistor` (seedless / watch-only) end-to-end
//! through a real `PlatformWalletManager`.
//!
//! Scope after the seedless rework: load reconstructs every persisted
//! wallet **watch-only** from its keyless account manifest. Wrong-seed
//! detection has moved to the first-sign path (covered in
//! `rs-platform-wallet-ffi/tests/sign_wrong_seed_gate.rs`). Per-row
//! decode failures surface as
//! [`SkipReason::CorruptPersistedRow`] without aborting the batch.
//!
//! RT cases here:
//! - RT-WO: round-trip — watch-only wallet is registered after reload.
//! - RT-Corrupt: a row with an empty manifest is skipped with
//!   `MissingManifest`, the other row loads, a `WalletSkippedOnLoad`
//!   event fires, `load` returns `Ok`.
//! - RT-Z: no key/seed material in any `LoadOutcome` / `SkipReason`
//!   surface (the structural-only contract).

use std::sync::{Arc, Mutex};

use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::wallet::Wallet;
use platform_wallet::changeset::{
    AccountRegistrationEntry, ClientStartState, ClientWalletStartState, CoreChangeSet,
    PersistenceError, PlatformWalletChangeSet, PlatformWalletPersistence,
};
use platform_wallet::events::{EventHandler, PlatformEvent, PlatformEventHandler};
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
                            core_state: w.core_state.clone(),
                            identity_manager: Default::default(),
                            unused_asset_locks: Default::default(),
                        },
                    );
                }
                Ok(out)
            }
        }
    }
}

/// Event handler recording every `PlatformEvent`.
#[derive(Default)]
struct RecordingHandler {
    events: Mutex<Vec<PlatformEvent>>,
}
impl EventHandler for RecordingHandler {}
impl PlatformEventHandler for RecordingHandler {
    fn on_platform_event(&self, event: &PlatformEvent) {
        self.events.lock().unwrap().push(event.clone());
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
            core_state: CoreChangeSet::default(),
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

    assert_eq!(outcome.loaded, vec![id]);
    assert!(outcome.skipped.is_empty());
    assert!(
        mgr.get_wallet(&id).await.is_some(),
        "watch-only restored wallet must be registered"
    );
    assert_eq!(mgr.wallet_ids().await, vec![id]);
}

/// RT-Corrupt: a corrupt row (empty manifest) is skipped with
/// `MissingManifest`; the other row loads cleanly; the load returns
/// `Ok`; exactly one `WalletSkippedOnLoad` event fires for the skipped
/// row.
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
        core_state: CoreChangeSet::default(),
        identity_manager: Default::default(),
        unused_asset_locks: Default::default(),
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

    // Exactly one WalletSkippedOnLoad event for B.
    {
        let events = h.events.lock().unwrap();
        assert_eq!(events.len(), 1);
        match &events[0] {
            PlatformEvent::WalletSkippedOnLoad { wallet_id, reason } => {
                assert_eq!(*wallet_id, id_b);
                assert!(matches!(
                    reason,
                    SkipReason::CorruptPersistedRow {
                        kind: CorruptKind::MissingManifest
                    }
                ));
            }
        }
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
        core_state: CoreChangeSet::default(),
        identity_manager: Default::default(),
        unused_asset_locks: Default::default(),
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
