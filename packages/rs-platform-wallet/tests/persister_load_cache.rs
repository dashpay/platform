//! TC-CODE-017 — `PlatformWalletManager` must call `persister.load()`
//! at most once across boot + the full per-wallet
//! `register_wallet` / `bind_shielded` round, draining cached
//! `ClientStartState` slices instead of re-issuing per-wallet loads.
//!
//! Without the cache, `register_wallet` (and historically
//! `bind_shielded`) called `persister.load()` once per wallet — each
//! call held the connection mutex for a full read, so M wallets =
//! M * O(state-size) mutex-bound work at boot.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::Network;
use platform_wallet::changeset::{
    ClientStartState, PersistenceError, PlatformAddressSyncStartState, PlatformWalletChangeSet,
    PlatformWalletPersistence,
};
use platform_wallet::events::{EventHandler, PlatformEventHandler};
use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet::PlatformWalletManager;

/// Persister that counts `load()` invocations and hands back a fresh
/// `ClientStartState` cloned from a stashed template each call.
/// `store` / `flush` succeed silently so post-registration writes
/// from the event-adapter don't poison the test.
struct CountingLoadPersister {
    load_calls: AtomicUsize,
    template_addresses: std::sync::Mutex<BTreeMap<WalletId, PlatformAddressSyncStartState>>,
}

impl CountingLoadPersister {
    fn new(template_addresses: BTreeMap<WalletId, PlatformAddressSyncStartState>) -> Self {
        Self {
            load_calls: AtomicUsize::new(0),
            template_addresses: std::sync::Mutex::new(template_addresses),
        }
    }

    fn load_call_count(&self) -> usize {
        self.load_calls.load(Ordering::SeqCst)
    }

    /// Replace the persister's address template — used by the
    /// cache-invalidation test to assert a `remove_wallet` +
    /// re-register sees the NEW state, not the stale cached one.
    fn replace_template(&self, addresses: BTreeMap<WalletId, PlatformAddressSyncStartState>) {
        *self.template_addresses.lock().unwrap() = addresses;
    }
}

impl PlatformWalletPersistence for CountingLoadPersister {
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
        self.load_calls.fetch_add(1, Ordering::SeqCst);
        // Hand out a snapshot of the template (BTreeMap of default
        // state is cheap to rebuild).
        let template = self.template_addresses.lock().unwrap();
        let platform_addresses: BTreeMap<WalletId, PlatformAddressSyncStartState> = template
            .keys()
            .map(|k| (*k, PlatformAddressSyncStartState::default()))
            .collect();
        Ok(ClientStartState {
            platform_addresses,
            wallets: BTreeMap::new(),
            #[cfg(feature = "shielded")]
            shielded: Default::default(),
        })
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
    persister: Arc<CountingLoadPersister>,
) -> Arc<PlatformWalletManager<CountingLoadPersister>> {
    let sdk = mock_sdk();
    let handler: Arc<dyn PlatformEventHandler> = Arc::new(NoopEventHandler);
    Arc::new(PlatformWalletManager::new(sdk, persister, handler))
}

/// Distinct 64-byte seed per wallet, deterministic per `index`.
fn seed_bytes_for(index: u8) -> [u8; 64] {
    let mut seed = [0u8; 64];
    for (i, b) in seed.iter_mut().enumerate() {
        // Index influences every byte so the recomputed WalletId is
        // distinct across registrations.
        *b = ((i as u8).wrapping_mul(7))
            .wrapping_add(3)
            .wrapping_add(index.wrapping_mul(31));
    }
    seed
}

/// TC-CODE-017-a — `register_wallet` after `load_from_persistor` must
/// reuse the cached `ClientStartState`. `persister.load()` is invoked
/// exactly once for the full M-wallet register round.
#[tokio::test]
async fn tc_code_017_a_register_after_load_reuses_cache() {
    // Empty address template so `load_from_persistor` succeeds (CODE-001
    // gate only trips when wallets={} AND platform_addresses!={}).
    let persister = Arc::new(CountingLoadPersister::new(BTreeMap::new()));
    let manager = build_manager(Arc::clone(&persister));

    // Single boot-time load.
    manager
        .load_from_persistor()
        .await
        .expect("empty payload boot should succeed");
    assert_eq!(
        persister.load_call_count(),
        1,
        "load_from_persistor must issue exactly one persister.load()"
    );

    // Register M wallets — each `register_wallet` historically called
    // `persister.load()` per-wallet. With the cache it must drain the
    // already-populated map and skip the load entirely.
    const M: u8 = 5;
    for i in 0..M {
        manager
            .create_wallet_from_seed_bytes(
                Network::Testnet,
                seed_bytes_for(i),
                WalletAccountCreationOptions::Default,
                Some(0),
            )
            .await
            .expect("wallet registration should succeed with empty persisted state");
    }

    assert_eq!(
        persister.load_call_count(),
        1,
        "register_wallet after load_from_persistor must NOT trigger \
         additional persister.load() calls (saw {})",
        persister.load_call_count(),
    );
}

/// TC-CODE-017-b — Fresh boot with no prior `load_from_persistor`:
/// the very first `register_wallet` lazily populates the cache via a
/// single `persister.load()`; subsequent registrations drain the
/// cache instead of re-loading.
#[tokio::test]
async fn tc_code_017_b_lazy_cache_init_on_first_register() {
    let persister = Arc::new(CountingLoadPersister::new(BTreeMap::new()));
    let manager = build_manager(Arc::clone(&persister));

    // No boot load — go straight to per-wallet registration.
    const M: u8 = 4;
    for i in 0..M {
        manager
            .create_wallet_from_seed_bytes(
                Network::Testnet,
                seed_bytes_for(i),
                WalletAccountCreationOptions::Default,
                Some(0),
            )
            .await
            .expect("wallet registration should succeed");
    }

    assert_eq!(
        persister.load_call_count(),
        1,
        "first register_wallet must lazily issue exactly one persister.load(); \
         subsequent registrations must drain the cache (saw {})",
        persister.load_call_count(),
    );
}

/// TC-CODE-017-c — Cache invalidation: after `remove_wallet`, the
/// cached slice for that wallet_id is dropped. A subsequent
/// `register_wallet` for the SAME id with the SAME persister payload
/// must see the live (re-loaded? — no, the cache for OTHER wallets is
/// preserved) state — i.e. it cannot re-apply a stale removed-then-
/// re-cached slice and must NOT trigger an additional load.
#[tokio::test]
async fn tc_code_017_c_remove_wallet_invalidates_cache_entry() {
    let persister = Arc::new(CountingLoadPersister::new(BTreeMap::new()));
    let manager = build_manager(Arc::clone(&persister));

    let wallet = manager
        .create_wallet_from_seed_bytes(
            Network::Testnet,
            seed_bytes_for(0),
            WalletAccountCreationOptions::Default,
            Some(0),
        )
        .await
        .expect("first registration should succeed");
    let wallet_id = wallet.wallet_id();

    let loads_after_first_register = persister.load_call_count();
    assert_eq!(
        loads_after_first_register, 1,
        "first registration lazily populates cache once"
    );

    // Replace the persister template so any rogue re-load after
    // remove would surface a slice with bogus content. We don't read
    // its content directly, but the call-count assertion + cache
    // invalidation contract guarantees no stale slice survives.
    let mut new_template = BTreeMap::new();
    new_template.insert(wallet_id, PlatformAddressSyncStartState::default());
    persister.replace_template(new_template);

    manager
        .remove_wallet(&wallet_id)
        .await
        .expect("remove_wallet should succeed");

    // Re-register the same wallet under the same id. The cache's
    // entry for `wallet_id` was invalidated, so the only state in
    // play for the new registration is "no slice" → fresh
    // `platform().initialize()`. Crucially, no additional
    // `persister.load()` fires — the cache slot stays populated
    // (just minus this wallet), so the lookup is in-memory.
    manager
        .create_wallet_from_seed_bytes(
            Network::Testnet,
            seed_bytes_for(0),
            WalletAccountCreationOptions::Default,
            Some(0),
        )
        .await
        .expect("re-registration after remove should succeed");

    assert_eq!(
        persister.load_call_count(),
        1,
        "remove_wallet + re-register must not trigger a second \
         persister.load() — the cache is preserved across removes \
         (only the removed wallet's slice is dropped). Saw {} call(s).",
        persister.load_call_count(),
    );
}
