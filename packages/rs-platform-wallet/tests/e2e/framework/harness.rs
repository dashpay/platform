//! Process-shared `E2eContext` initialised once per test run via
//! [`tokio::sync::OnceCell`]. Single entry point: [`E2eContext::init`]
//! wires config → workdir slot → SDK (with
//! [`TrustedHttpContextProvider`]) → manager → bank → registry →
//! startup sweep.
//!
//! SPV runtime is started during `Self::build` so monitored-address
//! / Layer-1 contracts have something live to observe. The SDK keeps
//! the trusted HTTP context provider for now — tests that need
//! SPV-backed proof verification can swap to `SpvContextProvider`.

use std::fs::File;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use platform_wallet::wallet::persister::NoPlatformPersistence;
use platform_wallet::{PlatformEventHandler, PlatformWalletManager, SpvRuntime};
use tokio::sync::OnceCell;
use tokio_util::sync::CancellationToken;

use super::bank::BankWallet;
use super::bank_identity::{self, BankIdentity};
use super::cleanup;
use super::config::Config;
use super::registry::PersistentTestWalletRegistry;
use super::sdk;
use super::spv;
use super::wait_hub::WaitEventHub;
use super::workdir;
use super::FrameworkResult;

/// Deadline for the SPV mn-list to reach `Synced` during framework
/// init. Internally raised to `COLD_CACHE_TIMEOUT_FLOOR` (600s) by
/// [`spv::wait_for_mn_list_synced`] so cold testnet caches still fit.
const SPV_READY_TIMEOUT: Duration = Duration::from_secs(180);

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
    /// SPV runtime started by [`Self::build`]. The SDK still uses
    /// the trusted HTTP context provider; this handle is exposed via
    /// [`Self::spv`] for tests that need to observe SPV state
    /// directly. Held as `Option` so individual setups can opt out
    /// without breaking the type — current default is `Some`.
    pub spv_runtime: Option<Arc<SpvRuntime>>,
    pub bank: BankWallet,
    /// Identity-credit sweep destination — registered or loaded once
    /// per process (see [`super::bank_identity`]).
    pub bank_identity: BankIdentity,
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

    /// Bank identity — destination of identity-credit sweeps.
    pub fn bank_identity(&self) -> &BankIdentity {
        &self.bank_identity
    }

    /// Persistent test-wallet registry — every `setup` registers,
    /// every `teardown` removes its entry.
    pub fn registry(&self) -> &PersistentTestWalletRegistry {
        &self.registry
    }

    /// Live SPV runtime started by [`Self::build`].
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

        // Start SPV before the bank loads so any L1 funding /
        // monitored-address contract assertions have a live mn-list
        // to observe. SDK keeps `TrustedHttpContextProvider` —
        // tests that need SPV-quorum-backed proof verification can
        // switch via `sdk.set_context_provider(SpvContextProvider::new(...))`
        // (it's `ArcSwap`-backed, safe to call after construction).
        // Address-list seeding pins SPV peers to the same DAPI hosts
        // the SDK is talking to (port-swapped to the P2P port), so
        // tests don't drift between two independent peer pools.
        let spv_runtime = spv::start_spv(&manager, &config, &workdir, sdk.address_list()).await?;
        spv::wait_for_mn_list_synced(&spv_runtime, SPV_READY_TIMEOUT).await?;
        let spv_runtime: Option<Arc<SpvRuntime>> = Some(spv_runtime);

        // Panics on under-funded balance — see `BankWallet::load`.
        let bank = BankWallet::load(&manager, &config).await?;

        // Surface the bank's Core (Layer-1) balance and primary
        // receive address at init with a visual marker so it's easy
        // to spot in test output. Most tests don't need duffs — a
        // zero balance is not fatal — but CR-/ID-007-class cases
        // require the address to be pre-funded with testnet duffs
        // before they can run end-to-end. Logged once per process so
        // funding the bank is a single-line copy-paste task. Errors
        // fetching the address are demoted to a warning so framework
        // init isn't gated on Core paths that most tests bypass
        // entirely.
        match bank.primary_core_receive_address().await {
            Ok(addr) => tracing::info!(
                target: "platform_wallet::e2e::bank",
                bank_core_addr = %addr,
                bank_core_balance = bank.core_balance_confirmed(),
                "═══ BANK CORE ADDRESS (fund here for CR-* / ID-007 tests) ═══"
            ),
            Err(err) => tracing::warn!(
                target: "platform_wallet::e2e::bank",
                error = %err,
                bank_core_balance = bank.core_balance_confirmed(),
                "Bank Core address derivation failed; pre-flight log incomplete"
            ),
        }

        // Resolve / register the bank identity BEFORE the orphan
        // sweep so [`cleanup::sweep_orphans`] has a valid sweep
        // destination on its very first invocation.
        let bank_identity = bank_identity::resolve_bank_identity(
            &manager,
            &bank,
            &workdir,
            config.bank_identity_id.as_deref(),
            bank.network(),
        )
        .await?;

        let registry = PersistentTestWalletRegistry::open(workdir.join("test_wallets.json"))?;

        // Best-effort startup sweep; failures don't abort init.
        let network = bank.network();
        match cleanup::sweep_orphans(&manager, &bank, &bank_identity, &registry, network).await {
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
            bank_identity,
            registry,
            cancel_token,
            wait_hub,
        })
    }
}
