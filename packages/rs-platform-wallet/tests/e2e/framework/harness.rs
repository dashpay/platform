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
use std::sync::{Arc, Mutex as StdMutex, Once};
use std::time::Duration;

use platform_wallet::wallet::persister::NoPlatformPersistence;
use platform_wallet::{PlatformEventHandler, PlatformWalletManager, SpvRuntime};
use rs_sdk_trusted_context_provider::TrustedHttpContextProvider;
use tokio::sync::OnceCell;
use tokio_util::sync::CancellationToken;

use super::bank::BankWallet;
use super::bank_identity::{self, BankIdentity};
use super::cleanup;
use super::config::{BankCoreGateSource, Config};
use super::registry::PersistentTestWalletRegistry;
use super::sdk;
use super::spv;
use super::wait;
use super::wait_hub::WaitEventHub;
use super::workdir;
use super::FrameworkResult;

/// Deadline for the SPV mn-list to reach `Synced` during framework
/// init. Internally raised to `COLD_CACHE_TIMEOUT_FLOOR` (600s) by
/// [`spv::wait_for_mn_list_synced`] so cold testnet caches still fit.
const SPV_READY_TIMEOUT: Duration = Duration::from_secs(180);

/// Threshold (duffs) used by the bank Core funding gate. The gate
/// waits for the bank's confirmed Core balance to reach at least this
/// value — any non-zero observation proves the SPV compact-filter scan
/// has walked far enough to see the bank's pre-funded UTXOs (Marvin's
/// QA-001). The gate's *timeout* lives on [`Config::bank_core_gate_timeout`]
/// and defaults to 900s; this constant is just the "any funding visible"
/// floor.
const BANK_CORE_GATE_MIN_DUFFS: u64 = 1;

/// Process-shared singleton populated on first
/// [`E2eContext::init`].
static CTX: OnceCell<E2eContext> = OnceCell::const_new();

/// Holds an [`Arc<SpvRuntime>`] for the in-flight `Self::build` call.
///
/// `OnceCell::get_or_try_init` discards the partial value when the
/// init future returns `Err` or panics — but the [`SpvRuntime`]
/// spawned via [`SpvRuntime::spawn_in_background`] keeps a self-clone
/// of the `Arc` alive on the tokio runtime, so the dash-spv data-dir
/// lockfile under `<workdir>/spv-data/.lock` survives the failure.
/// The next `init()` retry would then spawn a fresh runtime against
/// the same on-disk path, hit "Data directory locked", and emit a
/// 600s `wait_for_mn_list_synced` timeout — Marvin's QA-002, "one
/// panic poisons the whole serial suite".
///
/// This stash + the panic hook installed by [`E2eContext::build`] +
/// the retry-time cancel below break that cascade:
///
/// 1. After [`spv::start_spv`] succeeds, `build` writes its
///    `Arc<SpvRuntime>` here.
/// 2. If `build` returns `Err` or panics, the value stays put.
/// 3. The panic hook (sync) calls
///    [`SpvRuntime::cancel_background`] so the spawned `run()` task
///    starts its async teardown — drops `DiskStorageManager` →
///    drops `LockFile` → removes the on-disk lockfile.
/// 4. The next `init()` retry takes the `Arc` out, calls
///    `stop().await` (idempotent with the cancel above), and only
///    then proceeds to spawn a fresh runtime — guaranteeing the
///    lockfile is released before the new `DiskStorageManager::new`
///    runs.
/// 5. On success, `build` clears this slot so subsequent test-body
///    panics (which never re-enter `build`) don't re-trigger the
///    hook against a still-running SPV.
///
/// Mirrors the `SPV_CANCEL` pattern in DET's `backend-e2e/framework/
/// harness.rs` (`/home/ubuntu/git/dash-evo-tool/...`).
static IN_FLIGHT_SPV: StdMutex<Option<Arc<SpvRuntime>>> = StdMutex::new(None);

/// One-shot guard for installing the panic hook described on
/// [`IN_FLIGHT_SPV`]. The hook stays installed for the lifetime of
/// the test binary — chaining the previous hook so default panic
/// printing still fires.
static PANIC_HOOK_INSTALLED: Once = Once::new();

/// Best-effort post-cancel grace period for the spawned `run()` task
/// to advance through its async teardown (drop `DiskStorageManager`
/// → drop `LockFile` → remove `<spv-data>/.lock`) before the retry
/// proceeds to spawn a fresh runtime against the same path. The
/// retry already follows up with `stop().await` which serialises on
/// the runtime's internal client write-lock, so this sleep is purely
/// a fairness hint — it lets the spawned task be scheduled on the
/// shared tokio runtime instead of starving it. Matches DET's 500 ms.
const SPV_CANCEL_GRACE: Duration = Duration::from_millis(500);

/// Install [`PANIC_HOOK_INSTALLED`]'s panic hook. Idempotent.
///
/// On any panic, fires every in-flight SPV runtime's
/// [`SpvRuntime::cancel_background`] so the spawned `run()` task
/// starts its async teardown immediately. Cleared by `build` on
/// success so individual *test-body* panics don't disturb the
/// shared SPV runtime — the hook is only meaningful while
/// [`IN_FLIGHT_SPV`] is `Some`, which is exactly the window between
/// "SPV spawned" and "ownership handed to `E2eContext`".
fn ensure_panic_hook() {
    PANIC_HOOK_INSTALLED.call_once(|| {
        let prev_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            if let Some(spv) = IN_FLIGHT_SPV
                .lock()
                .inspect_err(|e| {
                    eprintln!("platform_wallet::e2e: IN_FLIGHT_SPV poisoned in panic hook: {e}");
                })
                .ok()
                .and_then(|g| g.clone())
            {
                tracing::warn!(
                    target: "platform_wallet::e2e::harness",
                    "panic during E2eContext::build — cancelling in-flight SPV \
                     runtime to release dash-spv data-dir lock so the next \
                     init() retry can re-acquire it"
                );
                spv.cancel_background();
            }
            prev_hook(info);
        }));
    });
}

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
    /// Shared handle to the SDK's [`TrustedHttpContextProvider`].
    /// Tests that deploy contracts at runtime must call
    /// [`TrustedHttpContextProvider::add_known_contract`] (and
    /// `add_known_token_configuration` for token slots) on this
    /// handle so the SDK's proof verifier can resolve the contract
    /// — otherwise the next state transition referencing the new
    /// contract surfaces `DriveProofError(UnknownContract)`. The
    /// inner caches are `Arc<Mutex<...>>`, so the SDK's clone of
    /// the provider sees mutations made through this handle. (QA-900)
    pub context_provider: Arc<TrustedHttpContextProvider>,
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

    /// Shared `Arc` over the SDK's [`TrustedHttpContextProvider`].
    /// Use [`TrustedHttpContextProvider::add_known_contract`] to
    /// register a freshly-deployed contract before any state
    /// transition that references it; see the field-level docs on
    /// [`Self::context_provider`]. (QA-900)
    pub fn context_provider(&self) -> &Arc<TrustedHttpContextProvider> {
        &self.context_provider
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
        // Install the panic hook before doing anything that can
        // panic — it's a no-op on subsequent calls. See
        // [`IN_FLIGHT_SPV`] for the full lifecycle rationale.
        ensure_panic_hook();

        // If a previous `build` call returned `Err` (or panicked), an
        // `Arc<SpvRuntime>` may still be parked in `IN_FLIGHT_SPV`
        // with the spawned `run()` task holding dash-spv's data-dir
        // lockfile. Take it out and `stop().await` so the lockfile is
        // fully released before this attempt's `start_spv` runs —
        // otherwise the new `DiskStorageManager::new` races the
        // orphan and surfaces "Data directory locked" warnings. The
        // panic-hook path also fired `cancel_background()`; calling
        // `stop()` here is idempotent against that, and additionally
        // serialises on the runtime's internal client write-lock so
        // we observe a clean lockfile state before proceeding.
        let orphan = IN_FLIGHT_SPV.lock().expect("IN_FLIGHT_SPV poisoned").take();
        if let Some(spv) = orphan {
            tracing::warn!(
                target: "platform_wallet::e2e::harness",
                "previous E2eContext::build left an SPV runtime in flight; \
                 awaiting graceful stop before retry"
            );
            // Give the panic-hook-fired `cancel_background` a moment
            // to advance the spawned task to its async teardown
            // before we contend on the same internal write-lock —
            // strictly a scheduler-fairness hint, the `stop().await`
            // below provides the actual ordering guarantee.
            tokio::time::sleep(SPV_CANCEL_GRACE).await;
            if let Err(e) = spv.stop().await {
                tracing::warn!(
                    target: "platform_wallet::e2e::harness",
                    error = %e,
                    "orphan SPV stop returned an error; continuing — the \
                     storage-side lockfile drop happens regardless of this \
                     result"
                );
            }
        }

        let config = Config::from_env()?;

        let (workdir, workdir_lock) = workdir::pick_available_workdir(&config.workdir_base)?;

        let cancel_token = CancellationToken::new();

        let (sdk, context_provider) = sdk::build_sdk(&config)?;

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
        // Park the runtime in `IN_FLIGHT_SPV` BEFORE the next
        // fallible step so any panic / Err inside the rest of `build`
        // hands the runtime to the panic hook + retry path described
        // on `IN_FLIGHT_SPV`. Cleared on success at the bottom of
        // `build`. Drops the previous slot value (should be `None`
        // already because we took it above; defensive).
        *IN_FLIGHT_SPV.lock().expect("IN_FLIGHT_SPV poisoned") = Some(Arc::clone(&spv_runtime));
        spv::wait_for_mn_list_synced(&spv_runtime, SPV_READY_TIMEOUT).await?;
        let spv_runtime: Option<Arc<SpvRuntime>> = Some(spv_runtime);

        // Panics on under-funded balance — see `BankWallet::load`.
        let bank = BankWallet::load(&manager, &config).await?;

        // Bank Core (Layer-1) funding gate. Marvin's QA-001 — first
        // cold-cache run on testnet walks ~1.47M compact filters from
        // genesis (~15 min); without the gate, the harness samples
        // `core_balance_confirmed` while the scan is still ~52 s in
        // and any CR-* / ID-007 case using `send_core_to` fails on a
        // false-zero balance. The gate is *default-on* (900s timeout)
        // so fresh-workdir runs don't race the scan; opt out via
        // `PLATFORM_WALLET_E2E_BANK_CORE_GATE=0` for Platform-only
        // suites that don't need Core duffs.
        //
        // Failure is demoted to a warn rather than a hard abort so
        // tests that don't need bank Core funding still run; the ones
        // that do panic at `send_core_to` with the operator-actionable
        // "top up at <addr>" message (see `BankWallet::send_core_to`).
        match config.bank_core_gate_timeout {
            Some(timeout) => {
                let source = match config.bank_core_gate_source {
                    BankCoreGateSource::Default => "default",
                    BankCoreGateSource::EnvTimeout => "env(PLATFORM_WALLET_E2E_BANK_CORE_GATE)",
                    BankCoreGateSource::EnvInvalidFallback => "env-invalid-fallback",
                    // Disabled is unreachable in this arm; kept for exhaustiveness.
                    BankCoreGateSource::EnvDisabled => "env-disabled",
                };
                tracing::info!(
                    target: "platform_wallet::e2e::bank",
                    timeout_secs = timeout.as_secs(),
                    min_duffs = BANK_CORE_GATE_MIN_DUFFS,
                    source = source,
                    "bank_core_gate active (waits for any non-zero confirmed \
                     Core balance so tests don't race a cold-cache compact-\
                     filter scan; first cold-cache run can take ~15 min while \
                     SPV walks filters from genesis, subsequent runs reuse \
                     the on-disk cache)"
                );
                match wait::wait_for_bank_funded(
                    &bank,
                    spv_runtime.as_deref(),
                    BANK_CORE_GATE_MIN_DUFFS,
                    timeout,
                )
                .await
                {
                    Ok(observed) => tracing::info!(
                        target: "platform_wallet::e2e::bank",
                        observed,
                        min_duffs = BANK_CORE_GATE_MIN_DUFFS,
                        "bank Core funding gate cleared"
                    ),
                    Err(err) => tracing::warn!(
                        target: "platform_wallet::e2e::bank",
                        error = %err,
                        "bank Core funding gate timed out; tests requiring \
                         bank Core funding will surface BankCoreUnderfunded with \
                         the operator-actionable top-up address"
                    ),
                }
            }
            None => tracing::info!(
                target: "platform_wallet::e2e::bank",
                source = "env(PLATFORM_WALLET_E2E_BANK_CORE_GATE)",
                "bank_core_gate disabled by env opt-out; tests requiring \
                 bank Core funding will surface BankCoreUnderfunded with \
                 the operator-actionable top-up address if SPV hasn't \
                 caught up yet"
            ),
        }

        // Surface the bank's Core (Layer-1) balance and primary
        // receive address at init with a visual marker so it's easy
        // to spot in test output. Logged AFTER the gate above so the
        // banner reflects the post-scan balance — Marvin's QA-001
        // (a pre-gate banner shows `core_balance_balance=0` while
        // SPV is mid-scan, which sends operators chasing a phantom
        // funding problem). Errors fetching the address are demoted
        // to a warning so framework init isn't gated on Core paths
        // that most tests bypass entirely.
        // QA-003: surface the bank's `birth_height` next to the
        // address + balance so operators can tell "wallet starts
        // above your funding tx" from "your tx hasn't confirmed yet".
        // When `core_balance == 0` and `birth_height > 0`, SPV's
        // compact-filter scan window starts past genesis, so any
        // funding tx confirmed at a lower block is invisible until
        // re-broadcast at a height ≥ `birth_height`. The bank
        // currently passes `Some(0)` to bypass this entirely (see
        // `BankWallet::load`); the warn is defence-in-depth in case
        // that ever regresses.
        let bank_birth_height = bank.birth_height().await;
        let bank_core_balance = bank.core_balance_confirmed();
        match bank.primary_core_receive_address().await {
            Ok(addr) => tracing::info!(
                target: "platform_wallet::e2e::bank",
                bank_core_addr = %addr,
                bank_core_balance,
                birth_height = bank_birth_height,
                "═══ BANK CORE ADDRESS (fund here for CR-* / ID-007 tests) ═══"
            ),
            Err(err) => tracing::warn!(
                target: "platform_wallet::e2e::bank",
                error = %err,
                bank_core_balance,
                birth_height = bank_birth_height,
                "Bank Core address derivation failed; pre-flight log incomplete"
            ),
        }
        if bank_core_balance == 0 && bank_birth_height > 0 {
            tracing::warn!(
                target: "platform_wallet::e2e::bank",
                birth_height = bank_birth_height,
                "Bank Core balance is zero with birth_height > 0 — SPV's filter \
                 scan starts at this block; any funding tx confirmed below it \
                 is invisible until re-broadcast at a height ≥ birth_height"
            );
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

        // Successful build — ownership of the runtime now lives on
        // the returned `E2eContext`. Clear `IN_FLIGHT_SPV` so the
        // panic hook becomes a no-op for individual *test-body*
        // panics, which must NOT cancel the shared SPV runtime that
        // surviving tests still depend on.
        *IN_FLIGHT_SPV.lock().expect("IN_FLIGHT_SPV poisoned") = None;

        Ok(E2eContext {
            config,
            workdir,
            workdir_lock,
            sdk,
            context_provider,
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
