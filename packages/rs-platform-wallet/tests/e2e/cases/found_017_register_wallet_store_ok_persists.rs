//! Found-017 positive guard — `register_wallet` still succeeds when the
//! persister `store` returns `Ok`.
//!
//! **Spec**: `tests/e2e/TEST_SPEC.md` (§ Found bugs → Found-017).
//! **Companion**: `found_017_register_wallet_store_error_lost` pins the
//!   failure path (store `Err` → registration must fail + roll back).
//!
//! The Found-017 fix makes the registration-round `store` load-bearing:
//! on `Err` it rolls back the in-memory insert and returns `Err`. This
//! test is the symmetric guard — it asserts the fix did *not*
//! over-rotate into rolling back (or failing) the *success* path. With a
//! persister whose `store` returns `Ok`, `create_wallet_from_seed_bytes`
//! must return `Ok` and the wallet must be present in `wallet_ids()`.
//!
//! Deterministic, no live network, no concurrency — same harness as the
//! failure-path pin, only the persister `store` outcome differs.

use std::sync::Arc;

use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::Network;
use platform_wallet::changeset::{
    ClientStartState, PersistenceError, PlatformWalletChangeSet, PlatformWalletPersistence,
};
use platform_wallet::events::{EventHandler, PlatformEventHandler};
use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet::PlatformWalletManager;

/// Test-only persister whose `store` always succeeds. `load` returns an
/// empty start state so the post-registration `load_persisted()` path is
/// quiet and the only thing under observation is the success path of the
/// registration-round `store`.
struct StoreOkPersister;

impl PlatformWalletPersistence for StoreOkPersister {
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
        Ok(ClientStartState::default())
    }
}

/// No-op event handler. Every `EventHandler` / `PlatformEventHandler`
/// method has a default body, so this empty impl satisfies the
/// `PlatformWalletManager::new` `app_handlers` parameter.
struct NoopEventHandler;
impl EventHandler for NoopEventHandler {}
impl PlatformEventHandler for NoopEventHandler {}

/// Deterministic seed for the test wallet. Value is irrelevant — the
/// guard is on registration ordering, not key material.
const TEST_SEED: [u8; 64] = [7u8; 64];

/// Positive regression guard for Found-017.
///
/// With a persister whose `store` succeeds, `create_wallet_from_seed_bytes`
/// must return `Ok` and the wallet must be present in `wallet_ids()`. This
/// fails if the Found-017 fail-closed fix regresses into rolling back (or
/// erroring) the success path.
// Multi-thread required: `PlatformWalletManager::shutdown` joins its
// coordinator OS threads through `ThreadRegistry::shutdown`, which drives
// each worker via `Handle::block_on` and panics on a current-thread runtime.
#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 4)]
async fn found_017_register_wallet_store_ok_persists() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    let sdk = Arc::new(dash_sdk::Sdk::new_mock());
    let persister = Arc::new(StoreOkPersister);
    let handler: Arc<dyn PlatformEventHandler> = Arc::new(NoopEventHandler);
    let manager = PlatformWalletManager::new(Arc::clone(&sdk), persister, vec![handler]);

    let result = manager
        .create_wallet_from_seed_bytes(
            Network::Mainnet,
            &TEST_SEED,
            WalletAccountCreationOptions::Default,
            // Pin the SPV scan window so birth-height resolution does not
            // touch the (absent) SPV runtime; keeps the path fully local.
            Some(0),
        )
        .await;

    let registered_ids = manager.wallet_ids().await;
    let wallet_present = !registered_ids.is_empty();

    let shutdown = manager.shutdown().await;
    assert!(
        shutdown.all_clean(),
        "manager teardown must leave no live worker or orphan: {shutdown:?}"
    );

    assert!(
        result.is_ok() && wallet_present,
        "Found-017 positive guard: with a persister whose `store` succeeds, \
         `create_wallet_from_seed_bytes` must return Ok AND the wallet must \
         be present in the live registry. Observed: \
         create_wallet_from_seed_bytes returned {} and wallet_ids() = {} \
         entr{} (wallet_present = {}). A failure here means the Found-017 \
         fail-closed fix over-rotated and now rolls back / errors the \
         success path. See TEST_SPEC.md Found-017.",
        if result.is_ok() { "Ok(_)" } else { "Err(_)" },
        registered_ids.len(),
        if registered_ids.len() == 1 {
            "y"
        } else {
            "ies"
        },
        wallet_present,
    );
}
