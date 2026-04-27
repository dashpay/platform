//! Process-shared `E2eContext` lazily initialised once per test run.
//!
//! The harness sets up the bank wallet, SDK, persistent registry,
//! and panic hook in one place so every test case under `cases/`
//! can reuse them. A per-process singleton via
//! [`tokio::sync::OnceCell`] amortises the cost across the suite.
//!
//! [`E2eContext::init`] is the single entry point. It wires (in
//! order):
//!
//! 1. [`Config::from_env`] — env vars + `.env`.
//! 2. [`workdir::pick_available_workdir`] — `flock`-locked slot.
//! 3. [`panic_hook::install`] — cancels background tasks on panic.
//! 4. [`sdk::build_sdk`] — `Sdk` with
//!    [`TrustedHttpContextProvider`] installed at construction
//!    time (testnet/mainnet endpoints baked in; devnet / custom via
//!    `PLATFORM_WALLET_E2E_TRUSTED_CONTEXT_URL`).
//! 5. [`PlatformWalletManager::new`] — manager backed by
//!    [`NoPlatformPersistence`].
//! 6. [`BankWallet::load`] — panics on under-funded balance.
//! 7. [`PersistentTestWalletRegistry::open`] +
//!    [`cleanup::sweep_orphans`].
//!
//! # SPV-based context provider — currently disabled
//!
//! The SPV start + readiness wait + live-swap to
//! [`SpvContextProvider`] are intentionally commented out (see
//! `Self::build`). The SPV cold-start path is unstable on testnet
//! today; the harness uses the deterministic
//! [`TrustedHttpContextProvider`] instead so e2e runs are fast and
//! reliable. To re-enable when SPV stabilises (Task #15), uncomment
//! the SPV blocks in `Self::build` and swap the SDK's context
//! provider via `Sdk::set_context_provider` after mn-list sync.
//!
//! The returned `&'static E2eContext` lives for the lifetime of the
//! process — `tokio_shared_rt` keeps the runtime alive across tests
//! so a single init pass amortises across the whole suite.

use std::fs::File;
use std::path::PathBuf;
use std::sync::Arc;

// `SpvRuntime` is referenced by the optional `spv_runtime` field
// kept for re-enablement of the SPV-based context provider (Task
// #15). The corresponding helpers (`spv::start_spv`,
// `wait_for_mn_list_synced`, `SpvContextProvider`) are still
// compilable but disabled — see `Self::build`.
use platform_wallet::wallet::persister::NoPlatformPersistence;
use platform_wallet::{PlatformEventHandler, PlatformWalletManager, SpvRuntime};
use tokio::sync::OnceCell;
use tokio_util::sync::CancellationToken;

use super::bank::BankWallet;
use super::cleanup;
use super::config::Config;
use super::panic_hook;
use super::registry::PersistentTestWalletRegistry;
use super::sdk;
use super::wait_hub::WaitEventHub;
use super::workdir;
use super::FrameworkResult;

/// Process-shared singleton. Initialised on first call to
/// [`E2eContext::init`]; subsequent calls return the same handle.
static CTX: OnceCell<E2eContext> = OnceCell::const_new();

/// Process-shared context for the e2e suite.
///
/// Tests acquire a `&'static E2eContext` via [`super::setup`] /
/// [`E2eContext::init`]. Direct construction is not part of the
/// public surface — the lazy init enforces the "one bank + one SPV
/// runtime per process" invariant.
pub struct E2eContext {
    /// Resolved configuration loaded from env vars + `.env`.
    pub config: Config,
    /// Slot-locked workdir base path.
    pub workdir: PathBuf,
    /// `flock`-held lock file kept open for the context's lifetime
    /// so concurrent test processes pick a different slot. Stored
    /// even though it's never read explicitly — dropping it would
    /// release the lock.
    workdir_lock: File,
    /// Constructed `dash_sdk::Sdk` shared between bank, test
    /// wallets, and SPV.
    pub sdk: Arc<dash_sdk::Sdk>,
    /// `PlatformWalletManager` shared across bank + test wallets.
    pub manager: Arc<PlatformWalletManager<NoPlatformPersistence>>,
    /// `SpvRuntime` — currently `None` while the SPV-based context
    /// provider is deferred (Task #15). The harness uses
    /// [`TrustedHttpContextProvider`] instead. Re-enabling SPV
    /// (uncomment the SPV blocks in `Self::build`) populates this
    /// with a started runtime; the field shape is kept so future
    /// Core-feature tests don't change signatures when SPV returns.
    pub spv_runtime: Option<Arc<SpvRuntime>>,
    /// Pre-funded bank wallet.
    pub bank: BankWallet,
    /// Persistent test-wallet registry.
    pub registry: PersistentTestWalletRegistry,
    /// Cancellation token tripped by the panic hook so SPV /
    /// background tasks shut down cleanly.
    pub cancel_token: CancellationToken,
    /// Process-shared event hub installed as the harness's
    /// `PlatformEventHandler`. Test wallets clone this `Arc` so
    /// `wait_for_balance` can wake on real chain / wallet events
    /// instead of polling the SDK on a fixed interval.
    pub wait_hub: Arc<WaitEventHub>,
}

impl E2eContext {
    /// Lazily build (or reuse) the process-shared context.
    ///
    /// On first call this performs the full init sequence (see
    /// module docs). Concurrent first-callers serialise inside
    /// [`OnceCell::get_or_try_init`] — only one builds the context,
    /// the rest wait for the same handle.
    pub async fn init() -> FrameworkResult<&'static Self> {
        CTX.get_or_try_init(Self::build).await
    }

    /// Borrow the underlying SDK. Convenience accessor used by the
    /// public test API.
    pub fn sdk(&self) -> &Arc<dash_sdk::Sdk> {
        &self.sdk
    }

    /// Borrow the manager — needed by `wallet_factory::TestWallet`
    /// and `cleanup::{sweep_orphans, teardown_one}`.
    pub fn manager(&self) -> &Arc<PlatformWalletManager<NoPlatformPersistence>> {
        &self.manager
    }

    /// Borrow the bank wallet — funding source for every test.
    pub fn bank(&self) -> &BankWallet {
        &self.bank
    }

    /// Borrow the registry — every `setup` registers itself here
    /// before handing control to the test body, every `teardown`
    /// removes its entry on success.
    pub fn registry(&self) -> &PersistentTestWalletRegistry {
        &self.registry
    }

    /// Borrow the SPV runtime, if any. Currently `None` — the
    /// harness uses [`TrustedHttpContextProvider`] instead of an
    /// SPV-backed context provider (Task #15). Future Core-feature
    /// tests that re-enable SPV will see `Some` here.
    pub fn spv(&self) -> Option<&Arc<SpvRuntime>> {
        self.spv_runtime.as_ref()
    }

    /// Cancellation token that the panic hook trips. Background
    /// helpers can `select!` on it for graceful shutdown.
    pub fn cancel_token(&self) -> &CancellationToken {
        &self.cancel_token
    }

    /// Borrow the process-shared event hub. Test wallets clone the
    /// `Arc` at construction time; helpers like
    /// [`super::wait::wait_for_balance`] await on the hub's `Notify`
    /// to wake on real SPV / wallet / platform-address-sync events.
    pub fn wait_hub(&self) -> &Arc<WaitEventHub> {
        &self.wait_hub
    }

    /// Build the singleton. Separated from `init` so the
    /// `OnceCell::get_or_try_init` body stays small.
    async fn build() -> FrameworkResult<E2eContext> {
        let config = Config::from_env()?;

        let (workdir, workdir_lock) = workdir::pick_available_workdir(&config.workdir_base)?;

        let cancel_token = CancellationToken::new();
        panic_hook::install(cancel_token.clone());

        let sdk = sdk::build_sdk(&config)?;

        // Persister + event handler. The persister discards
        // changesets (per-suite re-sync is fast on testnet). The
        // event handler is the shared [`WaitEventHub`] — installed
        // here so test helpers can `await` on real chain / wallet
        // events instead of polling the SDK on a fixed interval.
        let persister: Arc<NoPlatformPersistence> = Arc::new(NoPlatformPersistence);
        let wait_hub = Arc::new(WaitEventHub::new());
        let event_handler: Arc<dyn PlatformEventHandler> = Arc::clone(&wait_hub) as _;

        let manager = Arc::new(PlatformWalletManager::new(
            Arc::clone(&sdk),
            persister,
            event_handler,
        ));

        // SPV deferred — using `TrustedHttpContextProvider` while
        // SPV stabilizes (Task #15). The provider was already
        // installed at SDK construction in `sdk::build_sdk`. To
        // re-enable the SPV-backed provider, uncomment the block
        // below and the `SPV_READY_TIMEOUT` constant + `spv` /
        // `context_provider` imports at the top of this file.
        //
        // ```rust,ignore
        // const SPV_READY_TIMEOUT: Duration = Duration::from_secs(180);
        // use super::context_provider::SpvContextProvider;
        // use super::spv;
        //
        // // Start SPV before constructing the bank — the bank's
        // // load path runs a sync, and the SDK's proof
        // // verification will need the SpvContextProvider to
        // // answer quorum keys.
        // let spv_runtime = spv::start_spv(&manager, &config).await?;
        // spv::wait_for_mn_list_synced(&spv_runtime, SPV_READY_TIMEOUT).await?;
        //
        // // Live-swap the SDK's context provider to the
        // // SPV-backed variant. `Sdk::set_context_provider` is
        // // backed by `ArcSwap`, so this is safe to call after
        // // construction.
        // sdk.set_context_provider(SpvContextProvider::new(
        //     Arc::clone(&spv_runtime),
        // ));
        // ```
        let spv_runtime: Option<Arc<SpvRuntime>> = None;

        // Bank load panics on under-funded balance with an
        // actionable message — see `bank::BankWallet::load`.
        let bank = BankWallet::load(&manager, &config).await?;

        let registry = PersistentTestWalletRegistry::open(workdir.join("test_wallets.json"))?;

        // Run startup sweep best-effort. Failures are logged but
        // don't abort init — individual test runs can still proceed
        // and a stuck orphan retries on the next process launch.
        let network = bank.network();
        match cleanup::sweep_orphans(&manager, &bank, &registry, network).await {
            Ok(0) => {}
            Ok(n) => tracing::info!(
                target: "platform_wallet::e2e::harness",
                count = n,
                "startup sweep recovered orphan wallets from prior runs"
            ),
            Err(err) => tracing::warn!(
                target: "platform_wallet::e2e::harness",
                error = %err,
                "startup sweep encountered errors; continuing"
            ),
        }

        Ok(E2eContext {
            config,
            workdir,
            workdir_lock,
            sdk,
            manager,
            spv_runtime,
            bank,
            registry,
            cancel_token,
            wait_hub,
        })
    }
}
