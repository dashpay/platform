//! TC-CODE-001 — `load_from_persistor` must refuse to silently drop
//! platform-address state when the persister reports its `wallets`
//! rehydration is unimplemented.
//!
//! Persister contract (pre-#3692): `load()` returns
//! `wallets={}, platform_addresses={...}` because
//! `LOAD_UNIMPLEMENTED = &["ClientStartState::wallets"]`. The consumer
//! used to loop over the empty `wallets` map and drop the
//! `platform_addresses` slices at function scope. The fix forces the
//! caller to take the per-wallet `register_wallet` re-fetch path.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use platform_wallet::changeset::{
    ClientStartState, PersistenceError, PlatformAddressSyncStartState, PlatformWalletChangeSet,
    PlatformWalletPersistence,
};
use platform_wallet::error::PlatformWalletError;
use platform_wallet::events::{EventHandler, PlatformEventHandler};
use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet::PlatformWalletManager;

/// Persister whose `load()` payload is configurable per test — lets
/// `load_from_persistor` see the exact `(wallets, platform_addresses)`
/// shape we want.
struct CannedLoadPersister {
    payload: Mutex<Option<ClientStartState>>,
}

impl CannedLoadPersister {
    fn new(payload: ClientStartState) -> Self {
        Self {
            payload: Mutex::new(Some(payload)),
        }
    }
}

impl PlatformWalletPersistence for CannedLoadPersister {
    fn store(
        &self,
        _wallet_id: WalletId,
        _changeset: PlatformWalletChangeSet,
    ) -> Result<(), PersistenceError> {
        Ok(())
    }

    fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
        Ok(())
    }

    fn load(&self) -> Result<ClientStartState, PersistenceError> {
        // Hand out the canned payload exactly once — `load_from_persistor`
        // only calls `load()` once per invocation.
        Ok(self.payload.lock().unwrap().take().unwrap_or_default())
    }
}

struct NoopEventHandler;
impl EventHandler for NoopEventHandler {}
impl PlatformEventHandler for NoopEventHandler {}

fn mock_sdk() -> Arc<dash_sdk::Sdk> {
    Arc::new(
        dash_sdk::SdkBuilder::new_mock()
            .build()
            .expect("mock sdk should build"),
    )
}

fn build_manager(
    persister: Arc<CannedLoadPersister>,
) -> Arc<PlatformWalletManager<CannedLoadPersister>> {
    let sdk = mock_sdk();
    let handler: Arc<dyn PlatformEventHandler> = Arc::new(NoopEventHandler);
    Arc::new(PlatformWalletManager::new(sdk, persister, handler))
}

/// TC-CODE-001-a — Persister returned `wallets={}` but
/// `platform_addresses={W1, W2}` → manager must return
/// `PersistorMissingWalletRehydration` rather than silently dropping
/// the slices.
#[tokio::test]
async fn tc_code_001_a_refuses_silent_drop_of_orphan_platform_addresses() {
    let w1: WalletId = [1u8; 32];
    let w2: WalletId = [2u8; 32];

    let mut platform_addresses: BTreeMap<WalletId, PlatformAddressSyncStartState> = BTreeMap::new();
    platform_addresses.insert(w1, PlatformAddressSyncStartState::default());
    platform_addresses.insert(w2, PlatformAddressSyncStartState::default());

    let payload = ClientStartState {
        platform_addresses,
        wallets: BTreeMap::new(),
        #[cfg(feature = "shielded")]
        shielded: Default::default(),
    };

    let persister = Arc::new(CannedLoadPersister::new(payload));
    let manager = build_manager(Arc::clone(&persister));

    let err = manager
        .load_from_persistor()
        .await
        .expect_err("load_from_persistor must reject orphan platform_addresses");

    match err {
        PlatformWalletError::PersistorMissingWalletRehydration {
            unimplemented,
            orphan_addresses_count,
        } => {
            assert_eq!(
                orphan_addresses_count, 2,
                "should report both orphan slices"
            );
            assert!(
                unimplemented.iter().any(|s| s.contains("wallets")),
                "unimplemented list should mention wallets, got {:?}",
                unimplemented
            );
        }
        other => panic!("expected PersistorMissingWalletRehydration, got {other:?}"),
    }
}

/// TC-CODE-001-a (negative variant) — Empty persister payload (the
/// `NoPlatformPersistence` shape) must still succeed; the gate only
/// trips when `platform_addresses` is the orphan party.
#[tokio::test]
async fn tc_code_001_a_empty_payload_succeeds() {
    let persister = Arc::new(CannedLoadPersister::new(ClientStartState::default()));
    let manager = build_manager(Arc::clone(&persister));

    manager
        .load_from_persistor()
        .await
        .expect("empty payload must succeed — same shape as NoPlatformPersistence");
}
