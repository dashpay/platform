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
use std::sync::atomic::AtomicUsize;
use std::sync::{Arc, Mutex as StdMutex, Once};
use std::time::{Duration, Instant};

use platform_wallet::wallet::persister::NoPlatformPersistence;
use platform_wallet::{PlatformEventHandler, PlatformWalletManager, SpvRuntime};
use rs_sdk_trusted_context_provider::TrustedHttpContextProvider;
use tokio::sync::OnceCell;
use tokio_util::sync::CancellationToken;

use dpp::fee::Credits;

use super::bank::{BankWallet, CrossCheckResult};
use super::bank_identity::{self, BankIdentity};
use super::bank_plan;
use super::bank_rebalance;
use super::cleanup;
use super::config::{self, BankCoreGateSource, Config, ContextProviderKind};
use super::context_provider::CompositeContextProvider;
use super::identity_sync::IdentitySync;
use super::registry::{EntryStatus, PersistentTestWalletRegistry};
use super::sdk;
use super::spv;
use super::wait;
use super::wait_hub::WaitEventHub;
use super::workdir;
use super::{FrameworkError, FrameworkResult};

/// Deadline for the SPV mn-list to reach `Synced` during framework
/// init. Internally raised to `COLD_CACHE_TIMEOUT_FLOOR` (600s) by
/// [`spv::wait_for_mn_list_synced`] so cold testnet caches still fit.
const SPV_READY_TIMEOUT: Duration = Duration::from_secs(180);

/// Threshold (duffs) used by the bank Core funding gate. The gate
/// waits for the bank's confirmed Core balance to reach at least this
/// value — any non-zero observation proves the SPV compact-filter scan
/// has walked far enough to see the bank's pre-funded UTXOs (Marvin's
/// QA-001). The gate's *timeout* lives on [`Config::bank_core_gate_timeout`]
/// and defaults to 180s; this constant is just the "any funding visible"
/// floor.
const BANK_CORE_GATE_MIN_DUFFS: u64 = 1;

/// Tolerance (credits) for the bank Platform balance cross-check between
/// the harness wallet cache and an independent DAPI fetch (QA-V28-410).
/// Strict equality flagged sub-tDASH drift as MISMATCH, suppressing the
/// OK log even when the harness was healthy. 1 tDASH (1e8 credits) is
/// well above observed DAPI replica drift but small enough that any real
/// accounting bug still trips the MISMATCH branch.
const BANK_CROSS_CHECK_TOLERANCE_CREDITS: i64 = 100_000_000;

/// Returns `true` when the *second* independent fetch confirms the first —
/// i.e. both independent readings are well above the harness cache, meaning
/// a genuine replica-lag (#3611) scenario rather than a stale-node phantom.
///
/// The decision rule: `second_independent − harness_credits > tolerance`.
/// `first_independent` is accepted as a parameter for logging symmetry but is
/// not part of the decision — only the *confirmation* (second) read matters.
///
/// Extracted as a pure, synchronous function so unit tests can cover every
/// branch without an async runtime or a live DAPI node.
fn independent_balance_confirmed(
    _first_independent: Credits,
    second_independent: Credits,
    harness_credits: Credits,
    tolerance: i64,
) -> bool {
    let confirm_drift = second_independent as i64 - harness_credits as i64;
    confirm_drift > tolerance
}

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
    /// Bank identity — transient mid-run sink (drained back to the
    /// bank Platform address at suite start; used as the buffer for
    /// the core-refill chain). Registered or loaded once per process
    /// (see [`super::bank_identity`] and [`super::bank_rebalance`]).
    pub bank_identity: BankIdentity,
    pub registry: PersistentTestWalletRegistry,
    /// Framework-wide shutdown signal for background tasks. Not
    /// tripped by individual test panics — a single failing test
    /// must not cancel SPV / wait helpers for sibling tasks.
    pub cancel_token: CancellationToken,
    /// Installed as the harness's `PlatformEventHandler`; test
    /// wallets clone the `Arc` so `wait_for_balance` wakes on real
    /// events instead of fixed polling.
    pub wait_hub: Arc<WaitEventHub>,
    /// Constructor-injected observer of dash-spv
    /// `SyncEvent::ManagerError`s scoped to the masternode manager.
    /// [`spv::wait_for_mn_list_synced`] subscribes a fresh receiver
    /// per call so mn-list hard-stalls surface immediately instead of
    /// burning the cold-cache floor.
    pub mn_list_observer: Arc<spv::MnListErrorObserver>,
    /// Independent DAPI cross-check of the bank's Platform balance,
    /// captured once per init AFTER the startup sweep and
    /// `sync_and_refresh_floor` (QA-V26-005 / QA-V26-013). Both
    /// `harness_credits` and `independent_credits` reflect post-sweep
    /// state — the same balance that `assert_floor` evaluates. On fetch
    /// error `independent_credits = 0` with a `warn` logged.
    pub bank_balance_cross_check: Option<CrossCheckResult>,
    /// Periodic identity-state auto-sync. Calls
    /// [`refresh_identity`](platform_wallet::wallet::identity::IdentityWallet::refresh_identity)
    /// on every cached `(wallet, identity)` pair so
    /// `Identity::balance`, `Identity::revision`, and
    /// `Identity::public_keys` track chain reality during a test run.
    /// Cadence is taken from [`Config::identity_sync_interval`].
    ///
    /// Held in `StdMutex<Option<_>>` so the end-of-suite
    /// `SetupGuard::Drop` hook can `take()` + `stop().await` via
    /// [`Self::shutdown_identity_sync`]. Stopping the loop after the
    /// final `sweep_orphans` lets the run-loop's cancellation branch
    /// fire and surfaces the "loop exiting" debug log in traces —
    /// without that hook the loop was previously reaped at process
    /// exit and the shutdown breadcrumb was lost. (#353)
    pub identity_sync: StdMutex<Option<IdentitySync>>,
    /// Live count of outstanding [`super::SetupGuard`] instances.
    /// Incremented in [`super::setup`] and decremented in
    /// [`super::SetupGuard`]'s `Drop`. The guard whose decrement
    /// observes a previous value of `1` is the last in-flight test —
    /// it fires the end-of-suite [`cleanup::sweep_orphans`] pass so
    /// dust + retained-`Failed` entries surfaced by per-test Drop
    /// sweeps get one final retry without waiting for the next process
    /// startup. (V27-004)
    pub active_guards: AtomicUsize,
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

    /// Bank identity — transient mid-run sink (see
    /// [`super::bank_rebalance`] for the design contract).
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

    /// Constructor-injected mn-list `ManagerError` observer. Pass to
    /// [`spv::wait_for_mn_list_synced`] to surface dash-spv hard-stalls
    /// without the full cold-cache wait.
    pub fn mn_list_observer(&self) -> &Arc<spv::MnListErrorObserver> {
        &self.mn_list_observer
    }

    /// Framework-shutdown signal; background helpers can `select!`
    /// on it for graceful shutdown.
    pub fn cancel_token(&self) -> &CancellationToken {
        &self.cancel_token
    }

    pub fn wait_hub(&self) -> &Arc<WaitEventHub> {
        &self.wait_hub
    }

    /// Cancel the framework cancel token and wait for the identity-
    /// state auto-sync to settle. Intended for tests / orchestrators
    /// that want a deterministic shutdown signal (no-op when the loop
    /// has already been stopped, or was never started).
    pub async fn shutdown_identity_sync(&self) {
        self.cancel_token.cancel();
        let task = self
            .identity_sync
            .lock()
            .expect("identity_sync mutex poisoned")
            .take();
        if let Some(task) = task {
            task.stop().await;
        }
    }

    /// `true` when the bank's Platform balance met the token-suite floor
    /// (~88.8B credits) at init time. Token tests check this at startup and
    /// skip cleanly when `false` (QA-V26-003).
    pub fn bank_floor_satisfied(&self) -> bool {
        self.bank.bank_floor_satisfied()
    }

    /// Single source of truth for the token-suite bank-floor skip.
    ///
    /// Returns `true` when `case` should `return` early because the bank's
    /// Platform balance is below the token-suite floor. A drained live bank is
    /// a legitimate reason to not run (so this is a skip, not a hard failure),
    /// but a skip that reports PASS is indistinguishable from a real pass — so
    /// this emits a loud `WARN` plus a greppable `E2E-SKIP` stderr marker. A
    /// run where the whole token suite was skipped therefore shows `N` warnings
    /// and `N` `E2E-SKIP` lines instead of a silent all-green.
    ///
    /// Centralizing the policy here means the skip-vs-fail decision can be
    /// upgraded suite-wide in one edit (e.g. to a hard fail in CI, gated on an
    /// env var) without touching all 17 token cases.
    pub fn skip_if_bank_floor_unmet(&self, case: &str) -> bool {
        if self.bank_floor_satisfied() {
            return false;
        }
        let refill = self
            .bank()
            .primary_receive_address()
            .to_bech32m_string(self.bank().network());
        tracing::warn!(
            target: "platform_wallet::e2e::harness",
            case,
            refill_address = %refill,
            "E2E-SKIP: {case} did NOT run — bank Platform balance below the ~88.8B token-suite floor; \
             this is a SKIP reporting PASS, not a verified pass. Refill {refill} to exercise the token suite."
        );
        eprintln!(
            "E2E-SKIP: {case} did NOT run (bank Platform balance below ~88.8B floor); refill {refill} to run the token suite"
        );
        true
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
        sdk.refresh_protocol_version()
            .await
            .map_err(|err| FrameworkError::Sdk(format!("refresh protocol version: {err}")))?;

        // Register the withdrawals system contract on the context
        // provider's known-contracts cache. The shielded-withdrawal (SH-019,
        // Type 19) proof verifier resolves `withdrawals_contract::ID` to
        // build the expected withdrawal document; without it the verifier
        // errors `UnknownContract("withdrawals contract not available for
        // shielded withdrawal verification")`. Mirrors the token-contract
        // registration in `tokens.rs`. Fail-soft: a load error WARNs and
        // leaves SH-019 to surface the deployment gap rather than aborting
        // the whole suite.
        match dpp::system_data_contracts::load_system_data_contract(
            dpp::system_data_contracts::SystemDataContract::Withdrawals,
            dpp::version::PlatformVersion::latest(),
        ) {
            Ok(withdrawals) => context_provider.add_known_contract(withdrawals),
            Err(err) => tracing::warn!(
                target: "platform_wallet::e2e::harness",
                error = %err,
                "could not load the withdrawals system contract; shielded-withdrawal \
                 (SH-019) proof verification may fail with UnknownContract"
            ),
        }

        // Persister discards changesets (testnet re-sync is fast).
        // App handlers: the shared [`WaitEventHub`] so test helpers
        // await on real events instead of fixed polling, plus the
        // [`MnListErrorObserver`] so `wait_for_mn_list_synced` can
        // surface dash-spv `ManagerError`s without a post-construction
        // handler-registration escape hatch.
        let persister: Arc<NoPlatformPersistence> = Arc::new(NoPlatformPersistence);
        let wait_hub = Arc::new(WaitEventHub::new());
        let mn_list_observer = Arc::new(spv::MnListErrorObserver::new());

        let manager = Arc::new(PlatformWalletManager::new(
            Arc::clone(&sdk),
            persister,
            vec![
                Arc::clone(&wait_hub) as Arc<dyn PlatformEventHandler>,
                Arc::clone(&mn_list_observer) as Arc<dyn PlatformEventHandler>,
            ],
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
        //
        // Operator escape hatch: `PLATFORM_WALLET_E2E_DISABLE_SPV=1`
        // skips the spawn entirely so testnet ChainLock-cycle windows
        // (rust-dashcore #470) don't block the whole suite. Core-
        // dependent tests fail under this flag — see the warn below.
        // `context_provider=spv` resolves quorum keys from the SPV
        // runtime, so disabling SPV would leave the SDK on the cache-only
        // trusted provider whose quorum path can't answer — every
        // proof-verified query would fail. Surface that loudly; the run
        // can still exercise non-proof paths but the operator should pick
        // `context_provider=http` if they need SPV off.
        if config.disable_spv && config.context_provider == ContextProviderKind::Spv {
            tracing::warn!(
                target: "platform_wallet::e2e::harness",
                disable_spv = config::vars::DISABLE_SPV,
                context_provider = config::vars::CONTEXT_PROVIDER,
                "PLATFORM_WALLET_E2E_DISABLE_SPV with context_provider=spv: no \
                 SPV runtime will be started, so quorum-backed proof verification \
                 has no source — proof-verified queries WILL fail. Set \
                 context_provider=http (with a reachable quorums host) if you need \
                 SPV disabled."
            );
        }

        let spv_runtime: Option<Arc<SpvRuntime>> = if config.disable_spv {
            tracing::warn!(
                target: "platform_wallet::e2e::harness",
                var = config::vars::DISABLE_SPV,
                "PLATFORM_WALLET_E2E_DISABLE_SPV is set: skipping SPV runtime \
                 spawn and mn-list-sync gate. Core-dependent tests (CR-003 \
                 funded-asset-lock path, ID-007 Core-balance gates, anything \
                 that walks Core blocks) WILL fail; Platform-only flows still \
                 run. Use this only when testnet ChainLock cycles are blocking \
                 progress."
            );
            None
        } else {
            let spv_runtime =
                spv::start_spv(&manager, &config, &workdir, sdk.address_list()).await?;
            // Park the runtime in `IN_FLIGHT_SPV` BEFORE the next
            // fallible step so any panic / Err inside the rest of `build`
            // hands the runtime to the panic hook + retry path described
            // on `IN_FLIGHT_SPV`. Cleared on success at the bottom of
            // `build`. Drops the previous slot value (should be `None`
            // already because we took it above; defensive).
            *IN_FLIGHT_SPV.lock().expect("IN_FLIGHT_SPV poisoned") = Some(Arc::clone(&spv_runtime));
            spv::wait_for_mn_list_synced(&spv_runtime, &mn_list_observer, SPV_READY_TIMEOUT)
                .await?;

            // SPV-mode proof verification: swap the SDK's HTTP quorums
            // backend for `CompositeContextProvider` now that the mn-list
            // is synced. Quorum keys come from SPV (no hosted quorums host
            // — porter publishes none, QA-001); contracts / token configs
            // still come from the shared `TrustedHttpContextProvider`
            // cache, so `add_known_contract` (QA-900) is unaffected.
            // `set_context_provider` is `ArcSwap`-backed, safe post-build.
            if config.context_provider == ContextProviderKind::Spv {
                sdk.set_context_provider(CompositeContextProvider::new(
                    Arc::clone(&spv_runtime),
                    Arc::clone(&context_provider),
                ));
                tracing::info!(
                    target: "platform_wallet::e2e::harness",
                    "context_provider=spv: swapped SDK proof-verification \
                     backend to CompositeContextProvider (quorums via SPV, \
                     contracts via TrustedHttpContextProvider cache)"
                );
            }

            Some(spv_runtime)
        };

        let mut bank = BankWallet::load(&manager, &config).await?;

        // Bank Core (Layer-1) funding gate. Marvin's QA-001 — first
        // cold-cache run on testnet walks ~1.47M compact filters from
        // genesis (~15 min); without the gate, the harness samples
        // `core_balance_confirmed` while the scan is still ~52 s in
        // and any CR-* / ID-007 case using `send_core_to` fails on a
        // false-zero balance. The gate is *default-on* (180s timeout)
        // so fresh-workdir runs don't race the scan; opt out via
        // `PLATFORM_WALLET_E2E_BANK_CORE_GATE=0` for Platform-only
        // suites that don't need Core duffs.
        //
        // Failure is demoted to a warn rather than a hard abort so
        // tests that don't need bank Core funding still run; the ones
        // that do panic at `send_core_to` with the operator-actionable
        // "top up at <addr>" message (see `BankWallet::send_core_to`).
        //
        // When `DISABLE_SPV` is set the gate is auto-skipped: it polls
        // the SPV-fed `core_balance_confirmed`, which would never
        // advance without a running SPV runtime — letting the gate run
        // would just burn the full timeout for nothing.
        let effective_gate_timeout = if config.disable_spv {
            if config.bank_core_gate_timeout.is_some() {
                tracing::warn!(
                    target: "platform_wallet::e2e::bank",
                    var = config::vars::DISABLE_SPV,
                    "auto-disabling bank_core_gate because SPV is disabled (gate \
                     polls SPV-fed Core balance and would burn its full timeout \
                     for nothing)"
                );
            }
            None
        } else {
            config.bank_core_gate_timeout
        };
        match effective_gate_timeout {
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

        // Baseline the confirmed Core balance before the bootstrap self-fund
        // spends from it; the fund-planner snapshot below polls back to it.
        let pre_bootstrap_core_duff = bank.core_balance_confirmed();

        // Resolve / register the bank identity BEFORE the orphan
        // sweep so [`cleanup::sweep_orphans`] has a valid sweep
        // destination on its very first invocation.
        let bank_identity = bank_identity::resolve_bank_identity(
            &manager,
            &bank,
            &workdir,
            config.bank_identity_id.as_deref(),
            bank.network(),
            config.disable_spv,
        )
        .await?;

        // Make sure the bank identity carries a TRANSFER-purpose key
        // before we ask the drain helper (which broadcasts an
        // `IdentityCreditTransferToAddresses` transition gated on
        // `Purpose::TRANSFER`) to talk to it. Identities bootstrapped
        // before the bank-flow refactor only had AUTHENTICATION keys,
        // so the drain WARN'd and skipped on every run; this helper
        // adds the missing key once and short-circuits thereafter.
        // Best-effort: failures are logged inside the helper.
        match bank_rebalance::provision_transfer_key_if_missing(&bank, &bank_identity).await {
            Ok(Some(key_id)) => tracing::info!(
                target: "platform_wallet::e2e::harness",
                key_id,
                "bank identity provisioned with TRANSFER key for drain helper"
            ),
            Ok(None) => {}
            Err(err) => tracing::warn!(
                target: "platform_wallet::e2e::harness",
                error = %err,
                "bank identity TRANSFER-key provision encountered an error; continuing"
            ),
        }

        // The bank-identity drain (E3') is no longer a standalone step:
        // it is the first `Move` the fund planner emits below, so it runs
        // after the orphan sweep (maximising Platform surplus before the
        // planner sizes the leaf deficits).

        let registry = PersistentTestWalletRegistry::open(workdir.join("test_wallets.json"))?;

        // Capture pre-sweep registry stats so `assert_floor` can name them
        // in its panic message if the bank is still under-funded after sweep.
        let pre_sweep_orphans = registry.list_orphans();
        let pre_sweep_total = pre_sweep_orphans.len();
        let pre_sweep_failed = pre_sweep_orphans
            .iter()
            .filter(|(_, e)| e.status == EntryStatus::Failed)
            .count();

        // Best-effort startup sweep. Runs BEFORE the floor check so orphan
        // funds can flow back to the bank before we assert it's funded
        // (QA-V26-007). Failures don't abort init.
        let network = bank.network();
        let sweep_recovered =
            match cleanup::sweep_orphans(&manager, &bank, &bank_identity, &registry, network).await
            {
                Ok(0) => 0_usize,
                Ok(n) => {
                    tracing::info!(
                        target: "platform_wallet::e2e::harness",
                        count = n,
                        "startup sweep recovered orphan wallets from prior runs"
                    );
                    n
                }
                Err(err) => {
                    tracing::warn!(
                        target: "platform_wallet::e2e::harness",
                        error = %err,
                        "startup sweep encountered errors; continuing"
                    );
                    0
                }
            };

        // Re-read the bank's balance after the sweep so the floor check
        // counts any credits just swept back. `sync_and_refresh_floor`
        // also updates `bank_floor_satisfied` so the token-suite gate
        // reflects the post-sweep state rather than the load-time snapshot
        // (QA-V26-007). If still under-funded after sweep, panic with a
        // message that names sweep stats so operators know what ran.
        if let Err(err) = bank.sync_and_refresh_floor().await {
            tracing::warn!(
                target: "platform_wallet::e2e::harness",
                error = %err,
                "post-sweep bank resync failed; floor check uses pre-sweep balance"
            );
        }

        // Independent DAPI cross-check of the bank's Platform balance
        // (QA-V26-005 / QA-V26-013). Fires AFTER sync_and_refresh_floor so
        // `harness_credits` reflects the post-sweep wallet cache — the same
        // balance that assert_floor will evaluate. Firing pre-sweep (old
        // location) used a stale load-time snapshot; the cross-check would
        // agree with DAPI for well-funded banks (no mismatch → OK-only line)
        // making it appear absent when filtered for the MISMATCH keyword
        // (QA-V26-013). Never aborts init — warn is enough.
        //
        // #3611 recovery: when the independent reading is significantly
        // HIGHER than the wallet cache (positive drift), the startup sync
        // landed on a lagging DAPI replica.  We retry sync BALANCE_SYNC_RETRIES
        // times and, if still diverged, adopt the proof-verified independent
        // balance via `bank.accept_independent_platform_balance()` so the
        // floor gate, fund planner, and assert_floor() all see the real balance
        // instead of a false 0.  See bank.rs for the `effective_platform_credits`
        // abstraction that makes the adoption transparent to all call sites.
        let bank_balance_cross_check = {
            let network = bank.network();
            let result = bank.cross_check_balance(&sdk).await;
            let addr_bech32 = result.address.to_bech32m_string(network);
            let addr_hex = match &result.address {
                dpp::address_funds::PlatformAddress::P2pkh(hash) => hex::encode(hash),
                dpp::address_funds::PlatformAddress::P2sh(hash) => hex::encode(hash),
            };
            let nonce = result.nonce.unwrap_or(0);
            let signed_drift = result.independent_credits as i64 - result.harness_credits as i64;
            let abs_drift = signed_drift.unsigned_abs() as i64;
            if abs_drift <= BANK_CROSS_CHECK_TOLERANCE_CREDITS {
                tracing::info!(
                    target: "platform_wallet::e2e::bank",
                    harness_credits = result.harness_credits,
                    independent_credits = result.independent_credits,
                    drift = abs_drift,
                    tolerance = BANK_CROSS_CHECK_TOLERANCE_CREDITS,
                    addr_bech32 = %addr_bech32,
                    addr_hash160 = %addr_hex,
                    nonce,
                    "═══ BANK PLATFORM BALANCE CROSS-CHECK OK (QA-V26-005) ═══"
                );
            } else if signed_drift > BANK_CROSS_CHECK_TOLERANCE_CREDITS {
                // Positive drift: independent >> harness — almost certainly a
                // lagging DAPI replica at startup (#3611).  Retry sync to try
                // to land on a fresher node before falling back to adoption.
                tracing::warn!(
                    target: "platform_wallet::e2e::bank",
                    harness_credits = result.harness_credits,
                    independent_credits = result.independent_credits,
                    positive_drift = signed_drift,
                    tolerance = BANK_CROSS_CHECK_TOLERANCE_CREDITS,
                    retries = super::bank::BALANCE_SYNC_RETRIES,
                    addr_bech32 = %addr_bech32,
                    addr_hash160 = %addr_hex,
                    nonce,
                    "DAPI replica lag suspected (#3611): independent fetch shows \
                     more credits than harness wallet cache. Retrying startup sync."
                );
                let mut converged = false;
                for attempt in 1..=super::bank::BALANCE_SYNC_RETRIES {
                    tokio::time::sleep(super::bank::BALANCE_SYNC_RETRY_SLEEP).await;
                    match bank.sync_and_refresh_floor().await {
                        Ok(()) => {
                            let refreshed = bank.total_credits().await;
                            let remaining = result.independent_credits as i64 - refreshed as i64;
                            tracing::info!(
                                target: "platform_wallet::e2e::bank",
                                attempt,
                                refreshed_harness_credits = refreshed,
                                independent_credits = result.independent_credits,
                                remaining_drift = remaining,
                                "replica-lag recovery: sync retry {attempt} complete"
                            );
                            if remaining <= BANK_CROSS_CHECK_TOLERANCE_CREDITS {
                                tracing::info!(
                                    target: "platform_wallet::e2e::bank",
                                    attempt,
                                    refreshed_harness_credits = refreshed,
                                    "replica-lag recovered after {attempt} retry(ies) \
                                     — wallet cache now agrees with independent fetch"
                                );
                                converged = true;
                                break;
                            }
                        }
                        Err(err) => {
                            tracing::warn!(
                                target: "platform_wallet::e2e::bank",
                                attempt,
                                error = %err,
                                "replica-lag recovery: sync retry {attempt} failed"
                            );
                        }
                    }
                }
                if !converged {
                    // Guard against the inverse of #3611: the harness correctly
                    // reads 0 (bank is genuinely empty), but one DAPI node
                    // returned stale pre-spend state for the independent fetch.
                    // A second independent query will likely land on a different
                    // node; if it doesn't confirm the large balance, reject the
                    // adoption and let the planner see 0 — it will attempt E5
                    // Core→Platform if Core is funded, or surface a clear
                    // InsufficientFunds so the operator gets an actionable error.
                    tokio::time::sleep(super::bank::BALANCE_SYNC_RETRY_SLEEP).await;
                    let confirmation = bank.cross_check_balance(&sdk).await;
                    let confirm_drift = confirmation.independent_credits as i64
                        - confirmation.harness_credits as i64;

                    if independent_balance_confirmed(
                        result.independent_credits,
                        confirmation.independent_credits,
                        confirmation.harness_credits,
                        BANK_CROSS_CHECK_TOLERANCE_CREDITS,
                    ) {
                        // Second fetch also shows a large balance: two independent
                        // queries agree → genuine replica lag (#3611).  Adopt as
                        // before.
                        tracing::warn!(
                            target: "platform_wallet::e2e::bank",
                            harness_credits = result.harness_credits,
                            independent_credits = result.independent_credits,
                            positive_drift = signed_drift,
                            second_independent = confirmation.independent_credits,
                            retries = super::bank::BALANCE_SYNC_RETRIES,
                            "replica-lag (#3611): wallet cache still diverged after all retries. \
                             Second independent fetch confirms the large balance (dual-verified). \
                             Adopting proof-verified independent balance for floor gate, fund \
                             planner, and assert_floor()."
                        );
                        bank.accept_independent_platform_balance(result.independent_credits);
                        // Fix the spend path: adopted_platform_floor heals the gate/planner
                        // but `auto_select_inputs` reads address_credit_balance directly from
                        // the wallet manager map, not from `adopted_floor`. Inject the
                        // dual-verified balance into the spend cache so fund_address can spend
                        // these credits (mirrors provider.rs:621 and fund_from_asset_lock.rs:429).
                        bank.inject_verified_balance_into_spend_cache(result.independent_credits)
                            .await;
                    } else {
                        // Second fetch does NOT confirm the large balance.
                        // The first independent read was stale (phantom).  Do NOT
                        // adopt.  The planner will see platform = wallet-cache = 0
                        // and attempt E5 (Core→Platform asset-lock) if Core has
                        // enough duffs, or surface InsufficientFunds so the
                        // operator gets a clear message.
                        tracing::warn!(
                            target: "platform_wallet::e2e::bank",
                            harness_credits = result.harness_credits,
                            first_independent = result.independent_credits,
                            second_independent = confirmation.independent_credits,
                            confirm_drift,
                            tolerance = BANK_CROSS_CHECK_TOLERANCE_CREDITS,
                            "phantom balance REJECTED: second independent fetch (drift={confirm_drift}) \
                             does not confirm the first read ({first} credits). The first DAPI node \
                             returned stale pre-spend state. Proceeding with harness balance — the \
                             fund planner will attempt E5 Core→Platform bootstrap if Core is funded, \
                             or surface an operator-actionable InsufficientFunds error.",
                            first = result.independent_credits,
                        );
                        // No adopt call — adopted_platform_floor stays at 0.
                    }
                }
            } else {
                // Negative drift: harness >> independent — possible accounting
                // bug or a different replica serving the independent fetch.
                // Log and continue; do NOT adopt (harness overestimates = safe).
                tracing::warn!(
                    target: "platform_wallet::e2e::bank",
                    harness_credits = result.harness_credits,
                    independent_credits = result.independent_credits,
                    negative_drift = -signed_drift,
                    tolerance = BANK_CROSS_CHECK_TOLERANCE_CREDITS,
                    addr_bech32 = %addr_bech32,
                    addr_hash160 = %addr_hex,
                    nonce,
                    "bank Platform balance MISMATCH: harness cache > independent \
                     DAPI fetch (negative drift). Possible accounting bug or \
                     independent fetch hit a different lagging replica. Harness \
                     balance remains authoritative — investigate if tests fail."
                );
            }
            Some(result)
        };

        // Smart fund planner. Replaces the old straight-line
        // drain → assert_floor → refill → assert_core block with one
        // cost-ordered pass over the four account types (PLATFORM,
        // IDENTITY, SHIELDED, CORE):
        //   1. snapshot live balances,
        //   2. `plan()` — pure, deficit-gated, cheapest-edge-first
        //      (fast L2 < shield < one-time Core→Platform asset-lock ≪
        //      Platform→Core withdrawal),
        //   3. `execute()` — dispatches each Move to the bank_rebalance
        //      primitives in §3.4 order (drain → bootstrap → top-up →
        //      shield → withdrawal),
        //   4. `assert_all_floors()` — unified gate, subsumes the prior
        //      `assert_floor` (Platform panic) + `assert_core_funded_for_one_pass`
        //      (Core error).
        // Idempotent: a re-run with balances already at min emits an
        // empty plan (only the self-gating drain).
        //
        // `snapshot_balances` sizes E5 from `bank.core_balance_confirmed()`,
        // the lock-free `WalletBalance` atomic written only by the async
        // wallet_task as it drains the self-fund's WalletEvents. That atomic
        // is NON-MONOTONIC across the drain: it can still show the stale
        // pre-spend total, dip toward ~0 as the spent input is removed, then
        // settle at the post-spend total. A plain "reached target once" poll
        // clears on the stale-high value and the snapshot then reads the dip.
        // So poll the SAME atomic the snapshot reads and break only once it is
        // both at/above `converge_min` AND unchanged since the prior read.
        // SPV-disabled runs never self-fund and can't advance the atomic, skip.
        if !config.disable_spv {
            let converge_min = pre_bootstrap_core_duff
                .saturating_sub(bank_identity::MAX_BOOTSTRAP_CORE_OUTLAY_DUFF);
            let timeout = config
                .bank_core_gate_timeout
                .unwrap_or(config::DEFAULT_BANK_CORE_GATE_TIMEOUT);
            let start = Instant::now();
            let deadline = start + timeout;
            let mut iteration = 0u64;
            let mut prev: Option<u64> = None;
            loop {
                let observed = bank.core_balance_confirmed();
                iteration += 1;
                let settled = observed >= converge_min && prev == Some(observed);
                tracing::info!(
                    target: "platform_wallet::e2e::harness",
                    observed,
                    converge_min,
                    pre_bootstrap_core_duff,
                    iteration,
                    settled,
                    elapsed = ?start.elapsed(),
                    "fund-planner Core-balance convergence poll (reads the same \
                     core_balance_confirmed() atomic the planner snapshot uses)"
                );
                if settled {
                    break;
                }
                let remaining = deadline.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    tracing::warn!(
                        target: "platform_wallet::e2e::harness",
                        observed,
                        converge_min,
                        pre_bootstrap_core_duff,
                        iteration,
                        "confirmed Core balance did not settle at/above target before the \
                         fund-planner snapshot; E5 may be sized from a stale/transient value"
                    );
                    break;
                }
                prev = Some(observed);
                tokio::time::sleep(std::cmp::min(remaining, wait::BACKSTOP_WAKE_INTERVAL)).await;
            }
        }
        let balances = bank_plan::snapshot_balances(&bank, &bank_identity).await;
        let mins = bank_plan::mins_from_config(&config);
        match bank_plan::plan(balances, mins) {
            Ok(plan) => {
                tracing::info!(
                    target: "platform_wallet::e2e::harness",
                    ?balances,
                    ?mins,
                    moves = plan.len(),
                    "fund planner produced plan"
                );
                bank_plan::execute(&plan, &bank, &bank_identity, &config).await?;
            }
            Err(insufficiency) => {
                // Single operator-actionable failure: per-type have/need/short
                // + the two fixed top-up addresses. No partial-subset run.
                return Err(bank_plan::insufficiency_to_error(&insufficiency, &bank).await);
            }
        }

        // Re-read balances after execution so the floor gate evaluates
        // post-plan state.
        if let Err(err) = bank.sync_and_refresh_floor().await {
            tracing::warn!(
                target: "platform_wallet::e2e::harness",
                error = %err,
                "post-plan bank resync failed; floor check uses pre-plan balance"
            );
        }
        bank_plan::assert_all_floors(
            &bank,
            &bank_identity,
            &config,
            sweep_recovered,
            pre_sweep_total,
            pre_sweep_failed,
        )
        .await?;

        // Successful build — ownership of the runtime now lives on
        // the returned `E2eContext`. Clear `IN_FLIGHT_SPV` so the
        // panic hook becomes a no-op for individual *test-body*
        // panics, which must NOT cancel the shared SPV runtime that
        // surviving tests still depend on.
        *IN_FLIGHT_SPV.lock().expect("IN_FLIGHT_SPV poisoned") = None;

        // Spawn the identity-state auto-sync. Test-harness only — the
        // production wallet has no equivalent loop; until that lands
        // (feature request filed with the wallet team), this keeps
        // `Identity::balance`, `Identity::revision`, and
        // `Identity::public_keys` aligned with chain reality across
        // every test in the suite. Uses the framework cancel token so
        // a future graceful-shutdown path can fire it across all
        // background helpers in one shot.
        let identity_sync = IdentitySync::start(
            Arc::clone(&manager),
            cancel_token.clone(),
            config.identity_sync_interval,
        );
        tracing::info!(
            target: "platform_wallet::e2e::identity_sync",
            interval_secs = config.identity_sync_interval.as_secs(),
            "identity-state auto-sync started (refreshes balance/revision/public_keys per tick)"
        );

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
            mn_list_observer,
            bank_balance_cross_check,
            identity_sync: StdMutex::new(Some(identity_sync)),
            active_guards: AtomicUsize::new(0),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The classic bug scenario: harness reads 0 (bank genuinely empty),
    /// first independent fetch hits a stale node and returns the pre-spend
    /// balance (~425B).  A second fetch lands on a fresh node and returns 0.
    /// `independent_balance_confirmed` must return `false` → NO adoption.
    #[test]
    fn phantom_balance_rejected_when_second_fetch_returns_zero() {
        assert!(!independent_balance_confirmed(
            425_092_011_601,
            0,
            0,
            100_000_000
        ));
    }

    /// Genuine replica-lag (#3611): harness reads 0 (stale), BOTH independent
    /// fetches see a large balance (~425B).  Confirmation drift = 425B >> 100M
    /// tolerance → `independent_balance_confirmed` returns `true` → adopt.
    #[test]
    fn genuine_lag_accepted_when_second_fetch_confirms() {
        assert!(independent_balance_confirmed(
            425_092_011_601,
            425_000_000_000,
            0,
            100_000_000
        ));
    }

    /// Second fetch returns a small non-zero value (50M) that is within the
    /// 100M tolerance.  Still treated as "not confirmed" → reject.
    #[test]
    fn near_zero_second_fetch_rejected() {
        assert!(!independent_balance_confirmed(
            425_092_011_601,
            50_000_000,
            0,
            100_000_000
        ));
    }

    /// Edge: second fetch drift exactly equals tolerance → NOT confirmed
    /// (the guard uses strictly-greater-than).
    #[test]
    fn second_fetch_at_exact_tolerance_not_confirmed() {
        // confirm_drift = 100M − 0 = 100M; 100M > 100M is false → reject.
        assert!(!independent_balance_confirmed(
            425_092_011_601,
            100_000_000,
            0,
            100_000_000
        ));
    }

    /// Edge: both reads zero, harness zero → trivially not confirmed (no drift).
    #[test]
    fn all_zero_not_confirmed() {
        assert!(!independent_balance_confirmed(0, 0, 0, 100_000_000));
    }
}
