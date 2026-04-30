//! Process-shared `E2eContext` initialised once per test run via
//! [`tokio::sync::OnceCell`]. Single entry point: [`E2eContext::init`]
//! wires config → workdir slot → SDK (with
//! [`TrustedHttpContextProvider`]) → manager → bank → registry →
//! startup sweep.
//!
//! SPV-based context provider currently disabled; re-enable by
//! uncommenting the SPV blocks in `Self::build` (Task #15).

use std::fs::File;
use std::path::PathBuf;
use std::sync::Arc;

// `SpvRuntime` is held in an `Option` for SPV re-enablement
// (Task #15); the corresponding helpers stay compilable.
use platform_wallet::wallet::persister::NoPlatformPersistence;
use platform_wallet::{PlatformEventHandler, PlatformWalletManager, SpvRuntime};
use tokio::sync::OnceCell;
use tokio_util::sync::CancellationToken;

use super::bank::BankWallet;
use super::cleanup;
use super::config::Config;
use super::registry::PersistentTestWalletRegistry;
use super::sdk;
use super::wait_hub::WaitEventHub;
use super::workdir;
use super::FrameworkResult;

/// Process-shared singleton populated on first
/// [`E2eContext::init`].
static CTX: OnceCell<E2eContext> = OnceCell::const_new();

/// Process-shared context. Tests obtain a `&'static E2eContext`
/// via [`super::setup`]; lazy init enforces the
/// "one bank + one SPV runtime per process" invariant.
pub struct E2eContext {
    pub config: Config,
    pub workdir: PathBuf,
    /// `flock`-held lock kept open for the context's lifetime so
    /// concurrent processes pick a different slot. Dropping it
    /// releases the lock.
    workdir_lock: File,
    pub sdk: Arc<dash_sdk::Sdk>,
    pub manager: Arc<PlatformWalletManager<NoPlatformPersistence>>,
    /// `None` while the SPV-based context provider is deferred
    /// (Task #15); shape kept stable for future re-enablement.
    pub spv_runtime: Option<Arc<SpvRuntime>>,
    pub bank: BankWallet,
    pub registry: PersistentTestWalletRegistry,
    /// Framework-wide shutdown signal for background tasks. Not
    /// tripped by individual test panics — a single failing test
    /// must not cancel SPV / wait helpers for sibling tests.
    pub cancel_token: CancellationToken,
    /// Installed as the harness's `PlatformEventHandler`; test
    /// wallets clone the `Arc` so `wait_for_balance` wakes on real
    /// events instead of fixed polling.
    pub wait_hub: Arc<WaitEventHub>,
}

impl E2eContext {
    /// Lazily build (or reuse) the process-shared context.
    /// Concurrent callers serialise inside `OnceCell` — exactly one
    /// build runs.
    pub async fn init() -> FrameworkResult<&'static Self> {
        CTX.get_or_try_init(Self::build).await
    }

    pub fn sdk(&self) -> &Arc<dash_sdk::Sdk> {
        &self.sdk
    }

    pub fn manager(&self) -> &Arc<PlatformWalletManager<NoPlatformPersistence>> {
        &self.manager
    }

    /// Pre-funded bank wallet — the funding source for tests.
    pub fn bank(&self) -> &BankWallet {
        &self.bank
    }

    /// Persistent test-wallet registry — every `setup` registers,
    /// every `teardown` removes its entry.
    pub fn registry(&self) -> &PersistentTestWalletRegistry {
        &self.registry
    }

    /// `None` while the SPV-based context provider is deferred
    /// (Task #15).
    pub fn spv(&self) -> Option<&Arc<SpvRuntime>> {
        self.spv_runtime.as_ref()
    }

    /// Framework-shutdown signal; background helpers can `select!`
    /// on it for graceful shutdown.
    pub fn cancel_token(&self) -> &CancellationToken {
        &self.cancel_token
    }

    pub fn wait_hub(&self) -> &Arc<WaitEventHub> {
        &self.wait_hub
    }

    async fn build() -> FrameworkResult<E2eContext> {
        let config = Config::from_env()?;

        let (workdir, workdir_lock) = workdir::pick_available_workdir(&config.workdir_base)?;

        let cancel_token = CancellationToken::new();

        let sdk = sdk::build_sdk(&config)?;

        // Persister discards changesets (testnet re-sync is fast).
        // Event handler is the shared [`WaitEventHub`] so test
        // helpers can await on real events instead of fixed polling.
        let persister: Arc<NoPlatformPersistence> = Arc::new(NoPlatformPersistence);
        let wait_hub = Arc::new(WaitEventHub::new());
        let event_handler: Arc<dyn PlatformEventHandler> = Arc::clone(&wait_hub) as _;

        let manager = Arc::new(PlatformWalletManager::new(
            Arc::clone(&sdk),
            persister,
            event_handler,
        ));

        // SPV deferred (Task #15) — `TrustedHttpContextProvider`
        // is wired at SDK construction in `sdk::build_sdk`. To
        // re-enable the SPV-backed provider, uncomment below and
        // restore the `spv` / `context_provider` imports.
        //
        // ```rust,ignore
        // const SPV_READY_TIMEOUT: Duration = Duration::from_secs(180);
        // use super::context_provider::SpvContextProvider;
        // use super::spv;
        // // Start SPV before the bank's sync; SDK proof
        // // verification needs SpvContextProvider for quorum keys.
        // // Pass the SDK's live address list so SPV peers stay in
        // // lock-step with the DAPI endpoints the SDK is actually
        // // talking to (port-swapped to the effective P2P port).
        // let spv_runtime = spv::start_spv(&manager, &config, sdk.address_list()).await?;
        // spv::wait_for_mn_list_synced(&spv_runtime, SPV_READY_TIMEOUT).await?;
        // // `set_context_provider` is `ArcSwap`-backed, safe to
        // // call after construction.
        // sdk.set_context_provider(SpvContextProvider::new(
        //     Arc::clone(&spv_runtime),
        // ));
        // ```
        let spv_runtime: Option<Arc<SpvRuntime>> = None;

        // Panics on under-funded balance — see `BankWallet::load`.
        let bank = BankWallet::load(&manager, &config).await?;

        let registry = PersistentTestWalletRegistry::open(workdir.join("test_wallets.json"))?;

        // Best-effort startup sweep; failures don't abort init.
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
