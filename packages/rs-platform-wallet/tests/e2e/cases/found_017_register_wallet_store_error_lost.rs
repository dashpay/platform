//! Found-017 — `register_wallet` registers a wallet in memory even when
//! the persister `store` returns `Err` — the wallet vanishes on next launch.
//!
//! **Spec**: `tests/e2e/TEST_SPEC.md` (§ Found bugs → Found-017).
//! **Defect site**: `src/manager/wallet_lifecycle.rs:276-282` (the
//!   registration-round `persister.store(...)` whose `Err` is logged and
//!   then ignored) followed by the unconditional `self.wallets.insert(...)`
//!   at `src/manager/wallet_lifecycle.rs:347-349`.
//!
//! **Pinned status**: RED-BY-DESIGN — deterministic, no live network, no
//! concurrency. The bug is reproducible by injecting a persister whose
//! `store` fails; failure of this test IS the proof of the defect.
//!
//! ## Bug shape
//!
//! `register_wallet` snapshots the registration changeset (wallet
//! metadata + per-account xpubs + per-pool address snapshots) and hands
//! it to the persister:
//!
//! ```ignore
//! if let Err(e) = self.persister.store(wallet_id, registration_changeset) {
//!     tracing::error!(/* ... */ "failed to persist wallet registration changeset");
//! }
//! ```
//!
//! On error it logs and falls through. A few lines later the wallet is
//! inserted into the live registry unconditionally:
//!
//! ```ignore
//! let mut wallets = self.wallets.write().await;
//! wallets.insert(wallet_id, Arc::clone(&platform_wallet));
//! ```
//!
//! The registration round is the *only* record from which a wallet can
//! be rebuilt watch-only on next launch (`Wallet::new_watch_only` +
//! address-table population). Without it the persister has no trace of
//! the wallet. The user-visible effect: "I imported my wallet, used it,
//! restarted the app, and the wallet is gone" — a successful-looking
//! import that does not survive a restart.
//!
//! ## Why it is silent
//!
//! Note the asymmetry inside the same function. The two later
//! persistence-adjacent failure paths — `load_persisted()` and
//! `initialize_from_persisted()` — *do* roll back
//! (`wm.remove_wallet(&wallet_id)`) and return `Err`. Only the
//! registration `store` at line 276 is treated as best-effort: logged,
//! never surfaced to the caller, never rolled back. `tracing::error!`
//! into a log sink is not a signal the caller can act on, and the
//! function still returns `Ok(Arc<PlatformWallet>)` — the caller has
//! every reason to believe the import is durable.
//!
//! ## Correct behaviour
//!
//! The registration-round `store` is load-bearing, not best-effort.
//! On a persister error the registration must fail atomically:
//! roll back the in-memory state (no entry in `self.wallets`, no entry
//! in the inner `WalletManager`) and return `Err` so the caller knows
//! the wallet will not survive a restart — exactly the shape the two
//! later failure paths in the same function already use.
//!
//! ## What this test pins
//!
//! Build a `PlatformWalletManager` wired to a [`StoreFailsPersister`]
//! whose `store` returns `Err` while `load` / `flush` succeed (so the
//! *already-correct* `load_persisted` rollback path is **not** what
//! trips — this isolates the Found-017 defect specifically). Call
//! `create_wallet_from_seed_bytes`, then assert the correct contract:
//!
//! - the call returns `Err`, **and**
//! - the wallet is absent from the public `wallet_ids()` registry.
//!
//! ## Test lifecycle
//!
//! **Today (bug present)**: `store` fails, the error is logged and
//! swallowed, the wallet is inserted into `self.wallets`, and
//! `create_wallet_from_seed_bytes` returns `Ok`. Both assertions fire —
//! the call did not fail and the wallet is present despite having no
//! persisted record. **RED**.
//!
//! **After fix**: the registration `store` error aborts registration,
//! the in-memory state is rolled back, and the call returns `Err`. The
//! wallet is absent from `wallet_ids()`. **GREEN**. This test must be
//! updated alongside the fix only if the error *type* changes; the
//! `is_err()` + absent-from-registry shape is the durable contract.
//!
//! ## Why no live network
//!
//! The defect lives entirely in `register_wallet`'s synchronous
//! store-then-commit ordering. `Sdk::new_mock()` (the dev-dependency
//! `mocks` feature) supplies an SDK without a live backend; the only
//! network-touching call on the path — best-effort identity discovery
//! at the tail of `register_wallet` — has its `Err` logged and ignored
//! (the mock DAPI client returns `MockExpectationNotFound` rather than
//! panicking), so it neither blocks nor masks the assertion.

use std::sync::Arc;

use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::Network;
use platform_wallet::changeset::{
    ClientStartState, PersistenceError, PlatformWalletChangeSet, PlatformWalletPersistence,
};
use platform_wallet::events::{EventHandler, PlatformEventHandler};
use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet::PlatformWalletManager;

/// Test-only persister whose `store` always fails.
///
/// `load` and `flush` succeed deliberately: `register_wallet` calls
/// `load_persisted()` *after* the registration `store`, and that path
/// has its own (already-correct) rollback. Letting `load` return
/// `Ok(ClientStartState::default())` keeps that path quiet so the
/// failure this test observes can only be the Found-017 defect — the
/// swallowed registration-`store` error — and nothing else.
struct StoreFailsPersister;

impl PlatformWalletPersistence for StoreFailsPersister {
    fn store(
        &self,
        _wallet_id: WalletId,
        _changeset: PlatformWalletChangeSet,
    ) -> Result<(), PersistenceError> {
        Err(PersistenceError::backend(
            "Found-017 injected store failure: the registration round must \
             not be treated as best-effort"
                .to_string(),
        ))
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
/// `PlatformWalletManager::new` `app_handlers` parameter without doing
/// anything — the registration path under test emits no events this
/// test needs to observe.
struct NoopEventHandler;
impl EventHandler for NoopEventHandler {}
impl PlatformEventHandler for NoopEventHandler {}

/// Deterministic seed for the test wallet. Value is irrelevant — the
/// defect is in registration ordering, not key material.
const TEST_SEED: [u8; 64] = [7u8; 64];

/// Regression guard for Found-017.
///
/// The fix is landed: on a registration-round `persister.store` error,
/// `register_wallet` (`src/manager/wallet_lifecycle.rs`) rolls back the
/// in-memory state via `remove_wallet` and returns
/// `Err(WalletCreation(..))`. This test actively guards it — if a
/// future change reintroduces log-and-swallow on the registration
/// `store`, `create_wallet_from_seed_bytes` would return `Ok` with the
/// wallet still in `wallet_ids()` and this test fails.
///
/// Deterministic — no live network, no concurrency (`Sdk::new_mock`,
/// `StoreFailsPersister`). See TEST_SPEC.md Found-017.
// Multi-thread required: `PlatformWalletManager::shutdown` joins its
// coordinator OS threads through `ThreadRegistry::shutdown`, which drives
// each worker via `Handle::block_on` and panics on a current-thread runtime.
#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 4)]
async fn found_017_register_wallet_store_error_lost() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    // ── 1. Manager wired to a persister whose `store` always fails ──────
    //
    // `Sdk::new_mock()` defaults to `Network::Mainnet`; the test wallet
    // is created on the same network so `WalletMetadataEntry { network }`
    // is consistent and nothing fails for an unrelated reason.
    let sdk = Arc::new(dash_sdk::Sdk::new_mock());
    let persister = Arc::new(StoreFailsPersister);
    let handler: Arc<dyn PlatformEventHandler> = Arc::new(NoopEventHandler);
    let manager = PlatformWalletManager::new(Arc::clone(&sdk), persister, vec![handler]);

    // ── 2. Register a fresh wallet ──────────────────────────────────────
    //
    // `create_wallet_from_seed_bytes` -> `register_wallet`. The
    // registration-round `persister.store(...)` returns `Err`. Under the
    // CORRECT contract this aborts registration with an `Err` and rolls
    // back the in-memory state.
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

    // ── 3. Assert the correct atomic-failure contract ───────────────────
    //
    // Capture the registry state regardless of the call's outcome so the
    // failure message can report exactly what happened.
    let registered_ids = manager.wallet_ids().await;
    let wallet_present = !registered_ids.is_empty();

    // Best-effort shutdown of the spawned event-adapter task. Not part of
    // the assertion; keeps the shared runtime clean for sibling tests.
    let shutdown = manager.shutdown().await;
    assert!(
        shutdown.all_clean(),
        "manager teardown must leave no live worker or orphan: {shutdown:?}"
    );

    assert!(
        result.is_err() && !wallet_present,
        "Found-017 regression: registering a wallet while the persister \
         `store` fails must fail atomically — `create_wallet_from_seed_bytes` \
         must return Err AND the wallet must be absent from the live \
         registry. Observed: create_wallet_from_seed_bytes returned {} and \
         wallet_ids() = {} entr{} (wallet_present = {}). \
         The landed contract: register_wallet \
         (src/manager/wallet_lifecycle.rs) treats the registration-round \
         `persister.store` as load-bearing — on error it rolls back the \
         in-memory state via `remove_wallet` and returns \
         Err(WalletCreation(..)), matching the load_persisted / \
         initialize_from_persisted failure paths in the same function. A \
         failure here means that rollback was removed, so a wallet with no \
         persisted record is left in the live registry and vanishes on the \
         next launch — silent data loss. See TEST_SPEC.md Found-017.",
        if result.is_err() { "Err(_)" } else { "Ok(_)" },
        registered_ids.len(),
        if registered_ids.len() == 1 {
            "y"
        } else {
            "ies"
        },
        wallet_present,
    );
}
