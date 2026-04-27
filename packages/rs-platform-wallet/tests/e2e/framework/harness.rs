//! Process-shared `E2eContext` lazily initialised once per test run.
//!
//! The harness sets up the bank wallet, SDK, SPV runtime, persistent
//! registry, and panic hook in one place so every test case under
//! `cases/` can reuse them. SDK / SPV initialisation is genuinely
//! expensive (~30–60s on cold start); a per-process singleton via
//! [`tokio::sync::OnceCell`] amortises the cost.
//!
//! [`E2eContext::init`] is the single entry point. It wires (in
//! order):
//!
//! 1. [`Config::from_env`] — env vars + `.env`.
//! 2. [`workdir::pick_available_workdir`] — `flock`-locked slot.
//! 3. [`panic_hook::install`] — cancels SPV on init / test panic.
//! 4. [`sdk::build_sdk`] — `Sdk` with [`NoopContextProvider`].
//! 5. [`PlatformWalletManager::new`] — manager backed by
//!    [`NoPlatformPersistence`].
//! 6. [`spv::start_spv`] + [`spv::wait_for_mn_list_synced`].
//! 7. [`Sdk::set_context_provider`] — swap in
//!    [`SpvContextProvider`].
//! 8. [`BankWallet::load`] — panics on under-funded balance.
//! 9. [`PersistentTestWalletRegistry::open`] +
//!    [`cleanup::sweep_orphans`].
//!
//! The returned `&'static E2eContext` lives for the lifetime of the
//! process — `tokio_shared_rt` keeps the runtime alive across tests
//! so a single init pass amortises across the whole suite.

use std::fs::File;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use platform_wallet::events::EventHandler;
use platform_wallet::wallet::persister::NoPlatformPersistence;
use platform_wallet::{PlatformEventHandler, PlatformWalletManager, SpvRuntime};
use tokio::sync::OnceCell;
use tokio_util::sync::CancellationToken;

use super::bank::BankWallet;
use super::cleanup;
use super::config::Config;
use super::context_provider::SpvContextProvider;
use super::panic_hook;
use super::registry::PersistentTestWalletRegistry;
use super::sdk;
use super::spv;
use super::workdir;
use super::{FrameworkError, FrameworkResult};

/// Default timeout for `spv::wait_for_mn_list_synced` during init.
/// Cold start on testnet typically takes 30–90s; 180s gives slow CI
/// networks headroom without hanging forever.
const SPV_READY_TIMEOUT: Duration = Duration::from_secs(180);

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
    /// `SpvRuntime` started during init.
    pub spv_runtime: Arc<SpvRuntime>,
    /// Pre-funded bank wallet.
    pub bank: BankWallet,
    /// Persistent test-wallet registry.
    pub registry: PersistentTestWalletRegistry,
    /// Cancellation token tripped by the panic hook so SPV /
    /// background tasks shut down cleanly.
    pub cancel_token: CancellationToken,
}

impl E2eContext {
    /// Lazily build (or reuse) the process-shared context.
    ///
    /// On first call this performs the full init sequence (see
    /// module docs). Concurrent first-callers serialise inside
    /// [`OnceCell::get_or_try_init`] — only one builds the context,
    /// the rest wait for the same handle.
    ///
    /// **Multi-threaded tokio runtime required** — the SPV-backed
    /// [`SpvContextProvider`] uses
    /// [`tokio::task::block_in_place`] to bridge the synchronous
    /// `ContextProvider` trait to its async API.
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

    /// Borrow the SPV runtime. Future test cases that exercise
    /// Core-feature flows reach through here.
    pub fn spv(&self) -> &Arc<SpvRuntime> {
        &self.spv_runtime
    }

    /// Cancellation token that the panic hook trips. Background
    /// helpers can `select!` on it for graceful shutdown.
    pub fn cancel_token(&self) -> &CancellationToken {
        &self.cancel_token
    }

    /// Build the singleton. Separated from `init` so the
    /// `OnceCell::get_or_try_init` body stays small.
    async fn build() -> FrameworkResult<E2eContext> {
        let config = Config::from_env()?;

        let (workdir, workdir_lock) = workdir::pick_available_workdir(&config.workdir_base)?;

        let cancel_token = CancellationToken::new();
        panic_hook::install(cancel_token.clone());

        let sdk = sdk::build_sdk(&config)?;

        // Persister + event handler: tests use no-op variants. The
        // persister discards changesets (per-suite re-sync is fast
        // on testnet). The event handler is a noop bridge that
        // satisfies the trait without doing anything — bank /
        // tests don't need event callbacks.
        let persister: Arc<NoPlatformPersistence> = Arc::new(NoPlatformPersistence);
        let event_handler: Arc<dyn PlatformEventHandler> = Arc::new(NoopEventHandler);

        let manager = Arc::new(PlatformWalletManager::new(
            Arc::clone(&sdk),
            persister,
            event_handler,
        ));

        // Start SPV before constructing the bank — the bank's load
        // path runs a sync, and the SDK's proof verification will
        // need the SpvContextProvider to answer quorum keys.
        let spv_runtime = spv::start_spv(&manager, &config).await?;
        spv::wait_for_mn_list_synced(&spv_runtime, SPV_READY_TIMEOUT).await?;

        // Live-swap the SDK's context provider to the SPV-backed
        // variant. `dash_sdk::Sdk::set_context_provider` is backed
        // by `ArcSwap`, so this is safe to call after construction.
        sdk.set_context_provider(SpvContextProvider::new(Arc::clone(&spv_runtime)));

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
        })
    }
}

/// No-op `PlatformEventHandler` used by the test harness.
///
/// The bank / test wallets don't subscribe to SPV events for any
/// behavioural decision — sync calls are explicit. We still need a
/// handler to satisfy the `PlatformWalletManager::new` signature.
#[derive(Debug)]
struct NoopEventHandler;

impl EventHandler for NoopEventHandler {}
impl PlatformEventHandler for NoopEventHandler {}
