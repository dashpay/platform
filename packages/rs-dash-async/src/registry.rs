//! Shared lifecycle engine for background workers (`ThreadRegistry`).
//!
//! Centralizes the dangerous 80% of a background worker's lifecycle —
//! the generation-match exit epilogue, the
//! reap-or-park of a restarted worker's prior thread, and the orphan
//! drain — into one tested place, while deliberately leaving the
//! domain-specific 20% (the "is a pass in flight?" drain barrier) to the
//! consumer as a [`DrainHook`].
//!
//! Two worker kinds are supported:
//! - [`start_thread`](ThreadRegistry::start_thread) — a dedicated OS
//!   thread, for loops that `block_on` `!Send` futures internally (the
//!   `!Send` value never crosses the spawn boundary; the body itself is
//!   `Send`).
//! - [`start_task`](ThreadRegistry::start_task) — a tokio task, for
//!   `Send` futures.
//!
//! # Safety invariants
//!
//! - **A timed-out or dropped quiesce never detaches a live thread.**
//!   Every join path takes `&self`; the live join handle stays owned by
//!   the slot and is never moved into a cancellable future's frame. A
//!   dropped/timed-out [`quiesce`](ThreadRegistry::quiesce) therefore
//!   cannot drop-and-detach the handle — on timeout (or on an external
//!   drop) the handle is deterministically re-parked into the orphan
//!   list, and the slot reports [`WorkerStatus::Timeout`], never a clean
//!   `NotRunning`.
//! - **A store wipe cannot race a parked prior-generation thread.**
//!   Orphans live in the registry and
//!   [`any_alive_for`](ThreadRegistry::any_alive_for) is the key-scoped
//!   liveness gate spanning a key's live slot **and** its parked orphans
//!   (with [`any_alive`](ThreadRegistry::any_alive) the registry-wide
//!   variant). A store-wiping path scoped to one worker consults the
//!   key-scoped gate, so a parked still-live thread blocks the wipe of its
//!   own worker's store without an unrelated worker blocking it.

use std::collections::BTreeMap;
use std::future::Future;
use std::num::NonZeroUsize;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use futures::future::FutureExt;
use tokio::runtime::RuntimeFlavor;
use tokio_util::sync::CancellationToken;

// ---------------------------------------------------------------------
// Key & weight
// ---------------------------------------------------------------------

/// Worker identity. A wallet supplies a fixed enum; rs-dapi a generated
/// id. Blanket-implemented — consumers just derive the listed bounds on
/// their own key type.
pub trait RegistryKey: Copy + Ord + Eq + std::fmt::Debug + Send + Sync + 'static {}
impl<T: Copy + Ord + Eq + std::fmt::Debug + Send + Sync + 'static> RegistryKey for T {}

/// Teardown order. Lower weights drain first; equal weights drain
/// concurrently within a tier. Default `0`.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub struct ShutdownWeight(pub i32);

// ---------------------------------------------------------------------
// Status
// ---------------------------------------------------------------------

/// Terminal status of one worker as classified by the registry at the end
/// of [`quiesce`](ThreadRegistry::quiesce) or the orphan reap.
///
/// Consumers may re-export this directly on their public surface; the
/// variants distinguish clean exits (`Ok`, `NotRunning`) from every
/// non-clean outcome a host UAF-safety check must observe — see
/// [`is_clean`](Self::is_clean).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorkerStatus {
    /// The loop exited and its thread/task joined cleanly.
    Ok,
    /// A tokio task ended for a non-panic, non-clean reason (cancelled /
    /// aborted at the runtime level). Carries a reason when available.
    /// Only the `Task` kind can produce this; an OS thread never does.
    Stopped(Option<String>),
    /// The thread/task panicked; carries the best-effort panic message.
    Panicked(String),
    /// The managed join exceeded this worker's `join_budget`. The live
    /// handle was re-parked into the orphan list — UAF-safe, non-clean.
    Timeout,
    /// A parked orphan was still alive after the reap grace — UAF-safe,
    /// non-clean.
    Detached,
    /// No thread/task was running to join — never started, or already
    /// joined by a prior teardown.
    NotRunning,
    /// Infrastructural join failure that is neither a timeout nor a
    /// panic (unreachable in normal operation).
    Error(String),
}

impl WorkerStatus {
    /// `true` only for a fully clean outcome: joined normally (`Ok`) or
    /// never ran (`NotRunning`).
    pub fn is_clean(&self) -> bool {
        matches!(self, Self::Ok | Self::NotRunning)
    }
}

/// Aggregate result of [`ThreadRegistry::shutdown`].
#[derive(Clone, Debug, PartialEq, Eq)]
#[must_use = "inspect all_clean() before freeing host callback context / dropping the runtime: a non-clean status flags a still-live worker or orphan"]
pub struct ShutdownReport<K: RegistryKey> {
    /// Per-worker terminal status, keyed by worker id.
    pub per_worker: BTreeMap<K, WorkerStatus>,
    /// Number of parked orphans still alive at the reap deadline.
    pub detached: usize,
    /// Aggregate terminal status from the orphan reap. Reaped orphans are
    /// not keyed in `per_worker`, so without this a panicked/errored orphan
    /// that finished within the reap grace (`detached == 0`) would silently
    /// pass `all_clean()`. First non-clean classification wins; `Ok` when
    /// every reaped orphan was clean (or none were parked).
    pub orphan_status: WorkerStatus,
}

impl<K: RegistryKey> ShutdownReport<K> {
    /// `true` only when every per-worker status is clean, every reaped
    /// orphan was clean, and no orphan survived the reap.
    pub fn all_clean(&self) -> bool {
        self.detached == 0
            && self.orphan_status.is_clean()
            && self.per_worker.values().all(WorkerStatus::is_clean)
    }
}

// ---------------------------------------------------------------------
// Per-worker registration options
// ---------------------------------------------------------------------

/// Async drain hook the registry awaits **before** cancelling a worker,
/// in weight order. The domain barrier (raise a `quiescing` gate, wait
/// out an in-flight pass) lives here, supplied by the consumer — the
/// registry never owns domain semantics.
///
/// The captured state must be `Send + Sync`; a `!Send` capture does not
/// compile as a `DrainHook`. The fence is anchored to `E0277` (unsatisfied
/// `Send` bound) so the test cannot pass vacuously on some unrelated
/// compile error:
///
/// ```compile_fail,E0277
/// use std::rc::Rc;
/// use std::sync::Arc;
/// use dash_async::DrainHook;
/// let rc = Rc::new(42u32); // !Send
/// let _hook: DrainHook =
///     Arc::new(move || { let r = Rc::clone(&rc); Box::pin(async move { let _ = &r; }) });
/// ```
pub type DrainHook = Arc<dyn Fn() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

/// Default managed-join budget when a [`WorkerConfig`] does not override
/// it. Pinned so an accidental change surfaces in tests.
pub const DEFAULT_JOIN_BUDGET: Duration = Duration::from_secs(30);

/// Default orphan reap backstop (start-time reap and shutdown grace).
pub const DEFAULT_REAP_BACKSTOP: Duration = Duration::from_secs(1);

/// Threshold above which a per-worker drain hook is logged at WARN rather
/// than DEBUG by [`ThreadRegistry::quiesce`]. Not a hard timeout — the
/// caller still bounds the whole teardown — just a heuristic surface
/// for a hung drain. Sized as 1/3 of the default join budget so a drain
/// approaching the worker's join-budget ceiling is loud, while a normal
/// few-millisecond drain stays quiet.
pub const DRAIN_HOOK_WARN_THRESHOLD: Duration = Duration::from_secs(10);

/// Per-worker registration options.
pub struct WorkerConfig {
    /// Teardown tier; lower drains first, equal weights concurrently.
    pub weight: ShutdownWeight,
    /// Optional drain barrier awaited before cancellation.
    pub drain: Option<DrainHook>,
    /// Managed-join timeout for this worker.
    pub join_budget: Duration,
    /// OS-thread stack size ([`start_thread`](ThreadRegistry::start_thread)
    /// only; ignored by [`start_task`](ThreadRegistry::start_task)). `None`
    /// uses the platform default. Raise it for a loop whose body recurses
    /// deeply — e.g. GroveDB proof verification, which overflows the default
    /// stack and faults with SIGBUS on the guard page.
    pub stack_size: Option<NonZeroUsize>,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            weight: ShutdownWeight::default(),
            drain: None,
            join_budget: DEFAULT_JOIN_BUDGET,
            stack_size: None,
        }
    }
}

impl std::fmt::Debug for WorkerConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // `drain` is a boxed closure with no useful `Debug`; render its
        // presence instead.
        f.debug_struct("WorkerConfig")
            .field("weight", &self.weight)
            .field("drain", &self.drain.is_some())
            .field("join_budget", &self.join_budget)
            .field("stack_size", &self.stack_size)
            .finish()
    }
}

// ---------------------------------------------------------------------
// Internal handle + slot state
// ---------------------------------------------------------------------

/// A live worker's join handle. Kept owned by its slot so a cancellable
/// caller can never move it into a future frame and detach it on drop.
enum WorkerHandle {
    OsThread(std::thread::JoinHandle<()>),
    Task(tokio::task::JoinHandle<()>),
}

impl WorkerHandle {
    fn is_finished(&self) -> bool {
        match self {
            WorkerHandle::OsThread(h) => h.is_finished(),
            WorkerHandle::Task(h) => h.is_finished(),
        }
    }

    /// Classify a **finished** handle. Kind-dispatched (R3): an OS thread
    /// yields only `Ok` / `Panicked`; a task can also yield `Stopped`
    /// (cancelled / aborted at the runtime level).
    fn classify(self) -> WorkerStatus {
        match self {
            WorkerHandle::OsThread(j) => match j.join() {
                Ok(()) => WorkerStatus::Ok,
                Err(payload) => WorkerStatus::Panicked(panic_message(payload)),
            },
            WorkerHandle::Task(j) => match j.now_or_never() {
                Some(Ok(())) => WorkerStatus::Ok,
                Some(Err(e)) if e.is_panic() => {
                    WorkerStatus::Panicked(panic_message(e.into_panic()))
                }
                Some(Err(e)) => WorkerStatus::Stopped(Some(e.to_string())),
                // Only ever called on a finished handle, so a finished
                // task is always ready; this arm is defensive.
                None => WorkerStatus::Error("task handle not ready at join".to_string()),
            },
        }
    }
}

/// Best-effort extraction of a panic message (`&str` / `String` cases).
fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "<non-string panic>".to_string()
    }
}

/// One key's slot. The entry is created on first start and never removed,
/// so `generation` stays monotonic across the key's whole lifetime — a
/// parked prior-generation thread can therefore always tell that its
/// generation is stale. `cancel.is_some()` is the running indicator;
/// `handle` is the join handle, reaped by the next start or by quiesce.
struct SlotState {
    generation: u64,
    cancel: Option<CancellationToken>,
    handle: Option<WorkerHandle>,
    weight: ShutdownWeight,
    drain: Option<DrainHook>,
    join_budget: Duration,
}

// Manual `Default` (not `#[derive(Default)]`): the derived impl would
// initialise `join_budget` to `Duration::ZERO`, but the dormant slot must
// carry [`DEFAULT_JOIN_BUDGET`] so a key created on first-touch (via
// `BTreeMap::entry().or_default()`) is still join-bounded.
impl Default for SlotState {
    fn default() -> Self {
        Self {
            generation: 0,
            cancel: None,
            handle: None,
            weight: ShutdownWeight::default(),
            drain: None,
            join_budget: DEFAULT_JOIN_BUDGET,
        }
    }
}

impl SlotState {
    /// Rotate the slot onto a new generation: take the prior handle,
    /// install a fresh cancellation token, bump generation, and write
    /// `cfg`'s teardown config. Returns `(prior_handle, new_token,
    /// new_generation)`.
    fn prepare(&mut self, cfg: WorkerConfig) -> (Option<WorkerHandle>, CancellationToken, u64) {
        let prior = self.handle.take();
        let token = CancellationToken::new();
        self.cancel = Some(token.clone());
        self.generation += 1;
        let my_gen = self.generation;
        self.weight = cfg.weight;
        self.drain = cfg.drain;
        self.join_budget = cfg.join_budget;
        (prior, token, my_gen)
    }
}

// ---------------------------------------------------------------------
// The registry
// ---------------------------------------------------------------------

/// Shared lifecycle engine for background workers. See the module docs.
///
/// Parked orphans carry their originating key so a store-wiping path for
/// one worker can gate on [`any_alive_for`](Self::any_alive_for) without
/// being blocked by an unrelated worker still legitimately running.
pub struct ThreadRegistry<K: RegistryKey> {
    slots: Mutex<BTreeMap<K, SlotState>>,
    orphans: Mutex<Vec<(K, WorkerHandle)>>,
    reap_backstop: Duration,
    /// One-way teardown latch. [`shutdown`](Self::shutdown) sets it under
    /// the slot lock before snapshotting tiers; `start_thread`/`start_task`
    /// honour it under the same lock and refuse to register a new worker
    /// once teardown has begun, so a start racing shutdown can never leave
    /// an un-joined worker behind.
    closing: AtomicBool,
    /// Per-key clearing latch — refcounted live-holder count per key
    /// whose owner is mid clear-then-wipe (e.g. shielded
    /// `clear_shielded`). `start_thread`/`start_task` refuse a (re)start
    /// for any key with `count > 0`, so a fresh worker cannot slip past
    /// the "no new pass" barrier and re-persist into the store the
    /// clear is about to wipe. Resettable (mirror of
    /// [`closing`](Self::closing) but per-key and scoped to a
    /// [`ClearingGuard`]); the guard's `Drop` decrements the count and
    /// removes the entry only when the count reaches zero — so two
    /// concurrent / nested holders for the same key both keep the latch
    /// raised until the LAST guard drops.
    clearing: Mutex<BTreeMap<K, NonZeroUsize>>,
    /// Test seam: when set, the next OS-thread spawn returns an injected
    /// `io::Error` instead of really spawning, so the spawn-failure
    /// rollback path can be exercised deterministically.
    #[cfg(test)]
    force_spawn_failure: AtomicBool,
}

impl<K: RegistryKey> std::fmt::Debug for ThreadRegistry<K> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ThreadRegistry")
            .field("live_slots", &self.lock_slots().len())
            .field("orphans", &self.lock_orphans().len())
            .field("reap_backstop", &self.reap_backstop)
            .field("closing", &self.closing.load(Ordering::Acquire))
            .field("clearing", &self.lock_clearing().len())
            .finish()
    }
}

/// Process-wide latch for the panic=abort startup warn. Hoisted out of the
/// constructor so the in-crate regression test can assert it tripped without
/// peeking at function-local state.
#[cfg(panic = "abort")]
static PANIC_ABORT_WARNED: std::sync::Once = std::sync::Once::new();

impl<K: RegistryKey> ThreadRegistry<K> {
    /// New registry with the default reap backstop ([`DEFAULT_REAP_BACKSTOP`]).
    pub fn new() -> Arc<Self> {
        Self::with_reap_backstop(DEFAULT_REAP_BACKSTOP)
    }

    /// New registry with an explicit orphan reap backstop (the wallet
    /// uses 1s — the same grace separates "finishing" from "wedged").
    ///
    /// Under `panic = "abort"` builds (e.g. iOS release profiles) this
    /// constructor emits a single startup-time `tracing::warn!` so
    /// operators can audit the risk that an `EpilogueGuard` panic during
    /// teardown aborts the process before `Drop` can release the
    /// orphan-liveness gate. The warn is fired at most once per process via
    /// [`std::sync::Once`].
    pub fn with_reap_backstop(backstop: Duration) -> Arc<Self> {
        // Stable Rust has no runtime API to query the active panic strategy,
        // so the gate is compile-time. iOS release builds intentionally pick
        // abort — this is observability, not a hard error.
        #[cfg(panic = "abort")]
        PANIC_ABORT_WARNED.call_once(|| {
            tracing::warn!(
                "dash-async registry built with panic=abort: an EpilogueGuard \
                 panic during teardown aborts the process instead of unwinding, \
                 so the orphan-liveness gate may stay held — see the EpilogueGuard \
                 doc caveat. iOS release builds choose abort intentionally; non-iOS \
                 targets should prefer panic=unwind."
            );
        });
        Arc::new(Self {
            slots: Mutex::new(BTreeMap::new()),
            orphans: Mutex::new(Vec::new()),
            reap_backstop: backstop,
            closing: AtomicBool::new(false),
            clearing: Mutex::new(BTreeMap::new()),
            #[cfg(test)]
            force_spawn_failure: AtomicBool::new(false),
        })
    }

    /// Start an OS-thread worker for `!Send` loops. `body` runs on a
    /// fresh `std::thread` and may build and `block_on` `!Send` futures
    /// internally — the `!Send` value never crosses the spawn boundary
    /// (`body` itself is `Send`). Starting a key that already has a live
    /// worker is a no-op; a key whose prior thread has not been reaped is
    /// reaped-or-parked first (the restart-reap path). After
    /// [`shutdown`](Self::shutdown) has begun the call is also a no-op (the
    /// one-way closing latch).
    ///
    /// **Requires a multi-thread runtime**: the worker drives its loop
    /// via `Handle::block_on` and needs the shared timer/IO driver.
    ///
    /// **Blocks the calling thread on restart-reap**: when restarting a
    /// key whose prior OS thread is still finishing, this call SPINS
    /// SYNCHRONOUSLY for up to `WorkerConfig::reap_backstop` (default
    /// `DEFAULT_REAP_BACKSTOP` = 1 s) waiting for the prior to exit. Do
    /// not call it directly from an async context — drive it via
    /// `tokio::task::spawn_blocking` or a dedicated host thread.
    ///
    /// # Panics
    ///
    /// Panics if called outside a multi-thread Tokio runtime (see
    /// [`shutdown`](Self::shutdown)). It does **not** panic on thread-spawn
    /// failure: a failed spawn (e.g. the OS thread-count limit) is rolled
    /// back — the prior handle is re-installed rather than detached and the
    /// slot returns to not-running — and the call simply does not start a
    /// worker.
    pub fn start_thread<F>(self: &Arc<Self>, key: K, cfg: WorkerConfig, body: F)
    where
        F: FnOnce(CancellationToken) + Send + 'static,
    {
        Self::assert_multi_thread("start_thread");
        let prior_tid = {
            let mut slots = self.lock_slots();
            // One-way teardown latch: refuse new workers once shutdown has
            // begun, under the same lock shutdown snapshots tiers with.
            if self.closing.load(Ordering::Acquire) {
                return;
            }
            // Per-key clearing latch: refuse a (re)start while this key's
            // owner is mid clear-then-wipe, so a fresh worker cannot slip
            // past the "no new pass" barrier and re-persist into the store
            // the clear is about to wipe.
            if self.lock_clearing().contains_key(&key) {
                return;
            }
            let slot = slots.entry(key).or_default();
            if slot.cancel.is_some() {
                return;
            }
            // Snapshot the slot's pre-start config so a spawn failure can roll
            // the slot back to exactly its prior state: a re-installed prior
            // worker must keep its OWN teardown config, not inherit the failed
            // start's weight/drain/join_budget. Generation is rolled back too —
            // the bump is only ever observed under this lock and a failed start
            // spawns no thread to reference it, so the rollback is net-zero and
            // the externally-visible generation stays monotonic. The drain hook
            // is taken here so `prepare` below can install `cfg.drain` cleanly;
            // a spawn failure restores it via `slot.drain = prev_drain`.
            let prev_generation = slot.generation;
            let prev_weight = slot.weight;
            let prev_join_budget = slot.join_budget;
            let prev_drain = slot.drain.take();
            // `stack_size` is spawn-time only (not persisted on the slot):
            // read it out before `prepare` consumes `cfg`.
            let stack_size = cfg.stack_size;
            // Rotate the slot atomically: take prior handle, install fresh
            // cancellation token, bump generation, and write this start's
            // teardown config — all under THIS slot lock so a prior thread's
            // epilogue observes the post-swap generation.
            let (prior, token, my_gen) = slot.prepare(cfg);

            let reg = Arc::clone(self);
            let body_token = token;
            // Build the epilogue drop-guard INSIDE the worker closure, not
            // here: on a spawn failure the closure is dropped while we still
            // hold the slot lock, and a guard constructed out here would run
            // `run_epilogue` (which re-locks `slots`) on that drop and
            // deadlock. Constructing it inside means it only exists once the
            // thread is actually running. A panicking `body` then still
            // clears this generation's running flag via the guard's Drop
            // (under `panic = "unwind"`), and the panic keeps unwinding so
            // the join handle still classifies as `Panicked`.
            match self.spawn_os_thread(key, stack_size, move || {
                let _epilogue = EpilogueGuard { reg, key, my_gen };
                body(body_token);
            }) {
                Ok(join) => {
                    // Store the new handle, then park the prior into orphans —
                    // both still under THIS slot lock, so `shutdown`'s
                    // under-lock tier snapshot can never see the new slot
                    // without also seeing the prior accounted (R1: store handle
                    // -> park prior -> drop guard -> THEN bounded reap below).
                    // See `park_prior_locked` for the lock-order rationale; the
                    // bounded join stays out of the lock in `reap_parked_prior`.
                    slot.handle = Some(WorkerHandle::OsThread(join));
                    self.park_prior_locked(key, prior)
                }
                Err(e) => {
                    // Spawn failed (e.g. EAGAIN at the OS thread ceiling). Roll
                    // the slot back to exactly its pre-start state: clear the
                    // running flag, re-install the prior handle (never
                    // detached), and restore the prior teardown config +
                    // generation so nothing of the failed start lingers. The
                    // re-installed prior keeps its own weight/drain/join_budget
                    // for a later quiesce/shutdown, and generation returns to
                    // its pre-bump value (the bump was never observed outside
                    // this lock and spawned no thread). Nothing was parked, so
                    // there is no prior to reap below.
                    tracing::error!(
                        ?key,
                        error = %e,
                        "failed to spawn registry worker thread; rolling back \
                         start (prior handle re-installed, not detached)"
                    );
                    slot.cancel = None;
                    slot.handle = prior;
                    slot.generation = prev_generation;
                    slot.weight = prev_weight;
                    slot.drain = prev_drain;
                    slot.join_budget = prev_join_budget;
                    None
                }
            }
        };

        // The prior thread was cancellation-signalled by a preceding
        // cancel(); with the slot lock released its epilogue completes
        // promptly and the join lands in milliseconds — `reap_parked_prior`
        // then removes it from orphans and joins it. The backstop fires only
        // on a genuine wedge, in which case the still-live handle is left
        // parked (not dropped) so teardown can account for it.
        self.reap_parked_prior(key, prior_tid);
    }

    /// Start a tokio-task worker for `Send` futures. Same restart-reap
    /// semantics as [`start_thread`](Self::start_thread); does not require
    /// a multi-thread runtime.
    ///
    /// # Panics
    ///
    /// Panics if called outside a Tokio runtime context (`tokio::spawn`'s
    /// own precondition). After [`shutdown`](Self::shutdown) has begun the
    /// call is a no-op (the one-way closing latch).
    // TODO(rs-dapi-adoption): a task-only consumer can register on a
    // current_thread runtime yet trip `shutdown`'s multi-thread assert late.
    pub fn start_task<F, Fut>(self: &Arc<Self>, key: K, cfg: WorkerConfig, body: F)
    where
        F: FnOnce(CancellationToken) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        {
            let mut slots = self.lock_slots();
            // One-way teardown latch — see `start_thread`.
            if self.closing.load(Ordering::Acquire) {
                return;
            }
            // Per-key clearing latch — see `start_thread`.
            if self.lock_clearing().contains_key(&key) {
                return;
            }
            let slot = slots.entry(key).or_default();
            if slot.cancel.is_some() {
                return;
            }
            // No spawn-failure rollback here: `tokio::spawn` panics rather
            // than failing, so there is no Err arm to snapshot for.
            let (prior, token, my_gen) = slot.prepare(cfg);

            let reg = Arc::clone(self);
            let body_token = token;
            // Drop-guard epilogue, same rationale as `start_thread`: a task
            // whose future panics still clears its running flag via the
            // guard's Drop during unwind.
            let join = tokio::spawn(async move {
                let _epilogue = EpilogueGuard { reg, key, my_gen };
                body(body_token).await;
            });
            slot.handle = Some(WorkerHandle::Task(join));
            // Park the prior UNDER this slot lock, same rationale as
            // `start_thread`: it keeps `shutdown`'s under-lock tier snapshot
            // from ever missing the prior. A task cannot be joined
            // synchronously, so there is no bounded reap here — a live prior
            // is parked for the async orphan reap (`reap_orphans` /
            // `shutdown`) and a finished one is dropped. The returned thread
            // id is unused: a task prior has none, and a (mixed-usage)
            // OS-thread prior is likewise left to the async reap rather than
            // spun on synchronously from this (possibly async) caller.
            let _ = self.park_prior_locked(key, prior);
        }
    }

    /// Register an externally-spawned, externally-cancelled OS-thread
    /// worker for managed join / status only.
    ///
    /// Unlike [`start_thread`](Self::start_thread), the registry does
    /// **not** create or own a cancellation token: the caller drives
    /// cancellation through its own mechanism and hands the registry only
    /// the [`JoinHandle`](std::thread::JoinHandle), so
    /// [`shutdown`](Self::shutdown) / [`quiesce`](Self::quiesce) can join it
    /// and classify its terminal [`WorkerStatus`]. Because no token is
    /// installed, the slot's running flag stays clear —
    /// [`is_running`](Self::is_running) reports `false` for a handle-only
    /// worker, so consult the caller's own liveness signal instead;
    /// [`any_alive_for`](Self::any_alive_for) still reflects the handle.
    ///
    /// Restart-reap matches `start_thread`: a prior un-reaped handle under
    /// `key` is parked as an orphan (and its OS thread bounded-joined)
    /// before the new handle is installed, so a stop → start that
    /// overwrites the slot never detaches the still-draining prior thread.
    ///
    /// If teardown has begun ([`shutdown`](Self::shutdown)'s `closing`
    /// latch) or the key's clearing latch is raised, the handle is parked
    /// as an orphan rather than installed — the orphan reap still joins it,
    /// so it is never dropped-and-detached.
    ///
    /// **Blocks the calling thread on restart-reap**: like `start_thread`,
    /// this spins synchronously for up to the reap backstop when a prior OS
    /// thread is still finishing. Do not call it from an async task
    /// directly — drive it from a dedicated host thread.
    pub fn register_thread(
        self: &Arc<Self>,
        key: K,
        cfg: WorkerConfig,
        handle: std::thread::JoinHandle<()>,
    ) {
        let prior_tid = {
            let mut slots = self.lock_slots();
            // Teardown / clear latch: never install past a closing or
            // clearing barrier. The handle is already spawned, so parking it
            // as an orphan (rather than dropping it) keeps the join UAF-safe
            // — `shutdown`'s orphan reap still accounts for it.
            if self.closing.load(Ordering::Acquire) || self.lock_clearing().contains_key(&key) {
                // The caller already spawned a live, self-cancelled loop but
                // the registry is mid-teardown / mid-clear, so its slot is
                // barred. Parking keeps the join UAF-safe, but the worker is
                // now an uncancellable-by-the-registry live thread the caller
                // should have gated out — loud so the race is auditable, not
                // silent.
                tracing::error!(
                    ?key,
                    closing = self.closing.load(Ordering::Acquire),
                    clearing = self.lock_clearing().contains_key(&key),
                    "register_thread parked a live worker as an orphan because the \
                     registry was closing or clearing; the caller should have gated \
                     its start on is_closing()/is_clearing() before spawning"
                );
                self.lock_orphans()
                    .push((key, WorkerHandle::OsThread(handle)));
                return;
            }
            let slot = slots.entry(key).or_default();
            // Rotate the slot: take the prior handle, bump generation, write
            // this registration's teardown config, install the new handle —
            // all under THIS slot lock so a concurrent `quiesce`/`shutdown`
            // snapshot never sees the new handle without the prior accounted.
            // `cancel` is deliberately left untouched (`None`): the caller
            // owns cancellation.
            let prior = slot.handle.take();
            slot.generation += 1;
            slot.weight = cfg.weight;
            slot.drain = cfg.drain;
            slot.join_budget = cfg.join_budget;
            slot.handle = Some(WorkerHandle::OsThread(handle));
            self.park_prior_locked(key, prior)
        };

        // Bounded-join the parked prior with the slot lock released, same as
        // `start_thread`: the caller cancelled it before restarting, so its
        // epilogue lands in milliseconds; a genuine wedge past the backstop
        // is left parked for teardown rather than detached.
        self.reap_parked_prior(key, prior_tid);
    }

    /// Whether a worker is currently registered and running for `key`.
    pub fn is_running(&self, key: K) -> bool {
        self.lock_slots()
            .get(&key)
            .map(|s| s.cancel.is_some())
            .unwrap_or(false)
    }

    /// Signal-only cancellation of one worker.
    pub fn cancel(&self, key: K) {
        if let Some(slot) = self.lock_slots().get_mut(&key) {
            if let Some(token) = slot.cancel.take() {
                token.cancel();
            }
        }
    }

    /// Signal-only cancellation of every registered worker.
    pub fn cancel_all(&self) {
        for slot in self.lock_slots().values_mut() {
            if let Some(token) = slot.cancel.take() {
                token.cancel();
            }
        }
    }

    /// Mark `key` as mid clear-then-wipe and refuse any
    /// `start_thread`/`start_task` for it until the returned
    /// [`ClearingGuard`] drops. Per-key (other keys are unaffected) and
    /// resettable (subsequent clears can reacquire). The caller is
    /// expected to hold the guard across the full quiesce → liveness →
    /// wipe sequence, so a racing `(re)start` for the same key cannot
    /// install a fresh worker that re-persists into the store mid-clear.
    ///
    /// **Refcounted**: two concurrent / nested holders for the same key
    /// both keep the latch raised until the LAST guard drops. Each call
    /// increments a per-key counter; `Drop` decrements it and removes the
    /// entry only when the count reaches zero. This composes safely with
    /// re-entrant or concurrent clears on the same key (e.g. a host
    /// driving two FFI `shielded_clear` invocations through a read-locked
    /// handle).
    ///
    /// Returns a guard, not a `begin/end` pair, so the latch is released
    /// on every drop path — including panic unwinding — and a caller
    /// cannot leak it.
    pub fn hold_clearing(self: &Arc<Self>, key: K) -> ClearingGuard<K> {
        let mut clearing = self.lock_clearing();
        let next = match clearing.get(&key) {
            Some(n) => n
                .checked_add(1)
                .expect("ClearingGuard count overflowed usize::MAX"),
            None => NonZeroUsize::new(1).expect("1 is non-zero"),
        };
        clearing.insert(key, next);
        drop(clearing);
        ClearingGuard {
            reg: Arc::clone(self),
            key,
        }
    }

    /// Whether `key` is currently held under a [`ClearingGuard`] (count
    /// ≥ 1). Exposed so a coordinator can observe the latch BEFORE
    /// side-effects that would otherwise leak into the clear flow (e.g.
    /// lowering a continuously-held quiescing gate) even when its
    /// `start_thread`/`start_task` would be refused.
    pub fn is_clearing(&self, key: K) -> bool {
        self.lock_clearing().contains_key(&key)
    }

    /// Whether [`shutdown`](Self::shutdown) has latched the registry closed.
    ///
    /// The latch is one-way: once teardown begins it never reopens. A
    /// consumer that spawns and cancels its workers outside the registry
    /// (handing over only a join handle via
    /// [`register_thread`](Self::register_thread)) must gate
    /// its own `start` on this so it does not spawn a fresh, uncancelled
    /// loop that teardown has already stopped waiting for.
    pub fn is_closing(&self) -> bool {
        self.closing.load(Ordering::Acquire)
    }

    /// Await this worker's drain hook, cancel it, then join within its
    /// budget. The live handle is owned by the slot and is **never** moved
    /// into this future's frame, so a dropped/timed-out call cannot detach
    /// it; on the managed timeout — or if this future is dropped
    /// mid-poll — the handle is re-parked into the orphan list.
    pub async fn quiesce(&self, key: K) -> WorkerStatus {
        // Snapshot the drain hook + budget + generation, and bail early if
        // nothing is registered for this key. The generation is the anchor
        // for the supersede guard below.
        //
        // The inhabited check is raw (`cancel.is_some() || handle.is_some()`)
        // rather than `slot_alive()`: a finished-but-unreaped handle must
        // still be classified into its terminal status here, but
        // `slot_alive()` treats `handle.is_finished()` as "not alive" and
        // would short-circuit to `NotRunning` — incorrectly dropping the
        // result on the floor.
        let (drain, budget, my_gen) = {
            let slots = self.lock_slots();
            match slots.get(&key) {
                Some(s) if s.cancel.is_some() || s.handle.is_some() => {
                    (s.drain.clone(), s.join_budget, s.generation)
                }
                _ => return WorkerStatus::NotRunning,
            }
        };

        // R2: gate-before-cancel — drain hook fully awaited before the
        // cancel signal fires. Timed for observability; no hard timeout
        // here (the caller bounds teardown).
        if let Some(drain) = drain {
            let drain_started = Instant::now();
            drain().await;
            let drain_elapsed = drain_started.elapsed();
            if drain_elapsed >= DRAIN_HOOK_WARN_THRESHOLD {
                tracing::warn!(
                    ?key,
                    elapsed_ms = drain_elapsed.as_millis() as u64,
                    threshold_ms = DRAIN_HOOK_WARN_THRESHOLD.as_millis() as u64,
                    "registry drain hook took longer than the warn threshold; \
                     a slow drain hook delays the per-worker join budget"
                );
            } else {
                tracing::debug!(
                    ?key,
                    elapsed_ms = drain_elapsed.as_millis() as u64,
                    "registry drain hook completed",
                );
            }
        }

        // Signal-only cancel — but only if this is still the generation we
        // snapshotted. A concurrent restart (which can proceed the instant
        // we take `cancel` below) bumps the generation; taking the new
        // token here would silently un-track the fresh worker.
        if let Some(slot) = self.lock_slots().get_mut(&key) {
            if slot.generation == my_gen {
                if let Some(token) = slot.cancel.take() {
                    token.cancel();
                }
            }
        }

        // Poll-join within budget. The re-park guard moves the slot's
        // still-live handle into orphans if this future is dropped before
        // the loop finishes — the handle is never owned by this frame. Both
        // the guard and the loop are generation-scoped, so a concurrent
        // same-key restart's live handle is never parked or classified by
        // the quiesce that cancelled the *prior* generation.
        let _repark = Repark {
            reg: self,
            key,
            my_gen,
        };
        let deadline = Instant::now() + budget;
        loop {
            enum Step {
                Classify(WorkerHandle),
                Park(WorkerHandle),
                NotRunning,
                Superseded,
                Wait,
            }
            let step = {
                let mut slots = self.lock_slots();
                match slots.get_mut(&key) {
                    None => Step::NotRunning,
                    // A restart replaced the generation we were draining:
                    // the handle now in the slot belongs to a newer, live
                    // worker the restart owns. Leave it untouched.
                    Some(slot) if slot.generation != my_gen => Step::Superseded,
                    Some(slot) => match slot.handle.take_if(|h| h.is_finished()) {
                        Some(h) => Step::Classify(h),
                        None if slot.handle.is_none() => Step::NotRunning,
                        None if Instant::now() >= deadline => {
                            Step::Park(slot.handle.take().expect("handle present"))
                        }
                        None => Step::Wait,
                    },
                }
            };
            match step {
                Step::Classify(h) => return h.classify(),
                Step::Park(h) => {
                    self.lock_orphans().push((key, h));
                    return WorkerStatus::Timeout;
                }
                Step::NotRunning | Step::Superseded => return WorkerStatus::NotRunning,
                Step::Wait => tokio::time::sleep(Duration::from_millis(5)).await,
            }
        }
    }

    /// Is any registered worker **or** parked orphan still alive across
    /// the whole registry?
    pub fn any_alive(&self) -> bool {
        {
            let slots = self.lock_slots();
            for slot in slots.values() {
                if slot_alive(slot) {
                    return true;
                }
            }
        }
        self.lock_orphans().iter().any(|(_, h)| !h.is_finished())
    }

    /// Is the worker for `key` — its live slot **or** any orphan parked
    /// under that key — still alive? A store-wiping path scoped to one
    /// worker must gate on this (rather than the registry-wide
    /// [`any_alive`](Self::any_alive)) so an unrelated worker that is
    /// legitimately running does not block the wipe.
    pub fn any_alive_for(&self, key: K) -> bool {
        if let Some(slot) = self.lock_slots().get(&key) {
            if slot_alive(slot) {
                return true;
            }
        }
        self.lock_orphans()
            .iter()
            .any(|(k, h)| *k == key && !h.is_finished())
    }

    /// Reap parked orphans with a short grace; survivors are re-parked and
    /// reported as [`WorkerStatus::Detached`] (idempotent retry).
    pub async fn reap_orphans(&self, grace: Duration) -> WorkerStatus {
        self.reap_orphans_impl(grace).await.0
    }

    /// Weight-ordered teardown: ascending tier by tier, each worker's
    /// (drain-hook -> cancel -> join) run concurrently within a tier;
    /// orphan reap runs last. **Requires a multi-thread runtime.**
    ///
    /// Latches the registry closed first (under the slot lock, before the
    /// tier snapshot), so any `start_thread`/`start_task` racing teardown is
    /// either already in the snapshot or refused outright — shutdown is a
    /// one-way door and never leaves a worker un-joined. Idempotent.
    ///
    /// # Panics
    ///
    /// Panics if called outside a multi-thread Tokio runtime: an OS-thread
    /// worker drives its loop via `Handle::block_on` and needs the shared
    /// timer/IO driver, so a `current_thread` runtime would deadlock the
    /// join.
    pub async fn shutdown(&self) -> ShutdownReport<K> {
        // TODO(rs-dapi-adoption): see `start_task` — this assert is the late
        // panic point for a task-only consumer on a current_thread runtime.
        Self::assert_multi_thread("shutdown");

        // Snapshot keys grouped by weight. A `BTreeMap` iterates tiers in
        // ascending weight order, giving the lower-first drain. Latch the
        // registry closed under the same lock and before the snapshot so a
        // racing start is serialized: it either landed before this lock (and
        // is in the snapshot) or sees `closing` and bails.
        let tiers: BTreeMap<ShutdownWeight, Vec<K>> = {
            let slots = self.lock_slots();
            self.closing.store(true, Ordering::Release);
            let mut tiers: BTreeMap<ShutdownWeight, Vec<K>> = BTreeMap::new();
            for (key, slot) in slots.iter() {
                tiers.entry(slot.weight).or_default().push(*key);
            }
            tiers
        };

        let mut per_worker = BTreeMap::new();
        for (_weight, keys) in tiers {
            // Drain every worker in this tier concurrently: each
            // quiesce() drives its own drain-hook -> cancel -> join, and
            // `join_all` polls them on one task so their drain hooks
            // interleave (equal-weight concurrency).
            let drained = keys
                .into_iter()
                .map(|key| async move { (key, self.quiesce(key).await) });
            for (key, status) in futures::future::join_all(drained).await {
                per_worker.insert(key, status);
            }
        }

        // Account for parked orphans last. The terminal status is folded
        // into the report so a panicked/errored reaped orphan that finished
        // within the grace (`detached == 0`) still flips `all_clean()`.
        let (mut orphan_status, mut detached) = self.reap_orphans_impl(self.reap_backstop).await;

        // Late parkers: a `register_thread` that raced this teardown sees the
        // `closing` latch and parks its handle into orphans AFTER the reap
        // above snapshotted the list. Re-drain until the orphan list is stable
        // so such a straggler cannot slip through and let `all_clean()`
        // false-pass. Bounded: `closing` is one-way so no new worker can start,
        // and each pass either drains a finite backlog or re-parks genuine
        // survivors (`detached > 0`) — which we fold in and stop, leaving them
        // for an idempotent retry rather than spinning on a wedged thread.
        while detached == 0 && !self.lock_orphans().is_empty() {
            let (late_status, late_detached) = self.reap_orphans_impl(self.reap_backstop).await;
            if orphan_status.is_clean() && !late_status.is_clean() {
                orphan_status = late_status;
            }
            detached += late_detached;
        }
        ShutdownReport {
            per_worker,
            detached,
            orphan_status,
        }
    }

    // -----------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------

    fn lock_slots(&self) -> std::sync::MutexGuard<'_, BTreeMap<K, SlotState>> {
        self.slots.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn lock_orphans(&self) -> std::sync::MutexGuard<'_, Vec<(K, WorkerHandle)>> {
        self.orphans.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn lock_clearing(&self) -> std::sync::MutexGuard<'_, BTreeMap<K, NonZeroUsize>> {
        self.clearing.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn assert_multi_thread(ctx: &str) {
        assert!(
            matches!(
                tokio::runtime::Handle::current().runtime_flavor(),
                RuntimeFlavor::MultiThread
            ),
            "ThreadRegistry::{ctx}() requires a multi-thread Tokio runtime: an \
             OS-thread worker drives its loop via Handle::block_on and needs the \
             runtime's timer/IO driver, but a current_thread runtime can only \
             drive one block_on at a time"
        );
    }

    /// Gen-gated exit epilogue, run on the worker after its body returns
    /// (or unwinds): clear this slot's running flag only if a newer start
    /// has not since installed a replacement.
    fn run_epilogue(&self, key: K, my_gen: u64) {
        if let Some(slot) = self.lock_slots().get_mut(&key) {
            if slot.generation == my_gen {
                slot.cancel = None;
            }
        }
    }

    /// Spawn the named OS worker thread, surfacing a spawn failure as
    /// `io::Result` instead of panicking so the caller can roll back. The
    /// `#[cfg(test)]` seam forces a synthetic failure to exercise that path.
    fn spawn_os_thread<C>(
        &self,
        key: K,
        stack_size: Option<NonZeroUsize>,
        closure: C,
    ) -> std::io::Result<std::thread::JoinHandle<()>>
    where
        C: FnOnce() + Send + 'static,
    {
        #[cfg(test)]
        if self.force_spawn_failure.load(Ordering::Acquire) {
            return Err(std::io::Error::other("forced spawn failure (test seam)"));
        }
        let mut builder = std::thread::Builder::new().name(format!("tr-worker-{key:?}"));
        if let Some(stack_size) = stack_size {
            builder = builder.stack_size(stack_size.get());
        }
        builder.spawn(closure)
    }

    /// Park a restarted key's prior handle into orphans. **Must be called
    /// while the slot lock is held** — the resulting `slots`->`orphans`
    /// nesting is the only such nesting in this module and is deadlock-free
    /// (no path ever acquires `slots` while holding `orphans`, so there is no
    /// cycle). Parking the prior here, rather than after the slot lock is
    /// released, is what lets `shutdown`'s under-lock tier snapshot never
    /// miss it: the take-prior and the park-prior are then atomic from
    /// `shutdown`'s view. A finished task is dropped (detaching a finished
    /// task is a no-op); a live task and any OS thread are parked. Returns
    /// the parked OS thread's id so [`reap_parked_prior`](Self::reap_parked_prior)
    /// can find and bounded-join it; tasks (reaped asynchronously) return
    /// `None`.
    fn park_prior_locked(
        &self,
        key: K,
        prior: Option<WorkerHandle>,
    ) -> Option<std::thread::ThreadId> {
        match prior {
            Some(WorkerHandle::OsThread(h)) => {
                let tid = h.thread().id();
                self.lock_orphans().push((key, WorkerHandle::OsThread(h)));
                Some(tid)
            }
            Some(task) => {
                if task.is_finished() {
                    // Already finished: classify (non-blocking for a task) so a
                    // panicked prior generation is logged rather than dropped
                    // silently on the floor.
                    let status = task.classify();
                    if !status.is_clean() {
                        tracing::error!(
                            ?key,
                            ?status,
                            "prior-generation task ended non-cleanly at restart"
                        );
                    }
                } else {
                    self.lock_orphans().push((key, task));
                }
                None
            }
            None => None,
        }
    }

    /// Bounded reap of an OS-thread prior that [`park_prior_locked`](Self::park_prior_locked)
    /// parked under `key` at restart. Must be called with no registry lock
    /// held (it spins synchronously). The instant the parked thread finishes
    /// it is removed from orphans and joined — the join itself stays OUT of
    /// any lock (only the bookkeeping is taken under the orphans lock). A
    /// genuine wedge past the reap backstop is left parked, so teardown can
    /// still account for it. No-op when no OS thread was parked (`None`), or
    /// when the orphan was already taken by a concurrent reaper / `shutdown`
    /// (which then owns the join).
    fn reap_parked_prior(&self, key: K, prior_tid: Option<std::thread::ThreadId>) {
        let Some(tid) = prior_tid else {
            return;
        };
        let deadline = Instant::now() + self.reap_backstop;
        loop {
            // Bookkeeping under the orphans lock only: locate our parked
            // prior by thread id and, once it has finished, take it out to
            // join after the lock is released. Never hold the lock across the
            // join.
            let taken = {
                let mut orphans = self.lock_orphans();
                let pos = orphans.iter().position(|(k, h)| {
                    *k == key && matches!(h, WorkerHandle::OsThread(t) if t.thread().id() == tid)
                });
                match pos {
                    // Already taken by a concurrent reaper / shutdown: it owns
                    // the join now.
                    None => return,
                    Some(i) if orphans[i].1.is_finished() => Some(orphans.remove(i).1),
                    Some(_) if Instant::now() >= deadline => {
                        tracing::warn!(
                            ?key,
                            backstop = ?self.reap_backstop,
                            "prior worker thread did not finish within the reap \
                             backstop after cancellation; leaving it parked as an \
                             orphan for teardown to join rather than detaching it"
                        );
                        return;
                    }
                    Some(_) => None,
                }
            };
            if let Some(handle) = taken {
                // Join through `classify` rather than discarding the result: a
                // prior generation that panicked must not vanish silently at
                // restart. `classify` performs the actual join.
                let status = handle.classify();
                if !status.is_clean() {
                    tracing::error!(
                        ?key,
                        ?status,
                        "prior-generation worker ended non-cleanly during restart reap"
                    );
                }
                return;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    /// Drain the orphan list, polling until `grace`. Returns the terminal
    /// status and the number of survivors re-parked for an idempotent
    /// retry.
    async fn reap_orphans_impl(&self, grace: Duration) -> (WorkerStatus, usize) {
        let mut pending: Vec<(K, WorkerHandle)> = {
            let mut guard = self.lock_orphans();
            std::mem::take(&mut *guard)
        };
        if pending.is_empty() {
            return (WorkerStatus::Ok, 0);
        }

        let deadline = Instant::now() + grace;
        // Keep the first non-clean terminal status; a live survivor still
        // takes precedence at the deadline.
        let mut non_clean: Option<WorkerStatus> = None;
        loop {
            let mut still_live = Vec::with_capacity(pending.len());
            for (key, handle) in pending.drain(..) {
                if handle.is_finished() {
                    let status = handle.classify();
                    if !status.is_clean() {
                        non_clean.get_or_insert(status);
                    }
                } else {
                    still_live.push((key, handle));
                }
            }
            pending = still_live;

            if pending.is_empty() {
                return (non_clean.unwrap_or(WorkerStatus::Ok), 0);
            }
            if Instant::now() >= deadline {
                let survivors = pending.len();
                self.lock_orphans().extend(pending);
                return (WorkerStatus::Detached, survivors);
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    }

    /// Test-only seam: park a raw thread handle as an orphan under `key`.
    /// Injects a wedged prior-generation thread into the reap path without
    /// driving the full restart-reap dance. In-crate tests only.
    #[cfg(test)]
    fn park_orphan_for_test(&self, key: K, handle: std::thread::JoinHandle<()>) {
        self.lock_orphans()
            .push((key, WorkerHandle::OsThread(handle)));
    }
}

/// `true` if a slot is running or holds an unfinished handle.
fn slot_alive(slot: &SlotState) -> bool {
    slot.cancel.is_some() || slot.handle.as_ref().is_some_and(|h| !h.is_finished())
}

/// Re-park guard for [`ThreadRegistry::quiesce`]. If the poll-join future
/// is dropped before it finishes (e.g. an outer timeout fires), this moves
/// the slot's still-live handle into the orphan list instead of letting it
/// be dropped-and-detached. On normal completion the handle has already
/// been taken from the slot, so this is a no-op.
///
/// Generation-scoped: it only re-parks the handle if the slot still holds
/// the generation `quiesce` was draining. A concurrent same-key restart
/// bumps the generation and installs its own live handle; this guard leaves
/// that fresh handle alone.
struct Repark<'a, K: RegistryKey> {
    reg: &'a ThreadRegistry<K>,
    key: K,
    my_gen: u64,
}

impl<K: RegistryKey> Drop for Repark<'_, K> {
    fn drop(&mut self) {
        // Take the handle under the slot lock, release it, then push to
        // orphans. This path holds only one lock at a time; the single
        // sanctioned nesting in the module is `slots`->`orphans` in
        // `park_prior_locked`, and nothing ever takes `slots` while holding
        // `orphans`, so the ordering stays acyclic. Skip if a restart
        // superseded our generation (the handle is the new worker's, not
        // ours).
        let handle = self
            .reg
            .lock_slots()
            .get_mut(&self.key)
            .filter(|slot| slot.generation == self.my_gen)
            .and_then(|slot| slot.handle.take());
        if let Some(handle) = handle {
            self.reg.lock_orphans().push((self.key, handle));
        }
    }
}

/// Worker-side exit guard. Runs the generation-gated [`run_epilogue`]
/// from its `Drop`, so a worker whose `body` returns normally **or**
/// unwinds on panic still clears its running flag — `is_running()` then
/// reflects reality and `start()` can relaunch a crashed loop.
///
/// Panic-strategy caveat: the clear-on-panic half relies on `Drop` running
/// while the stack unwinds, so it holds under
/// `panic = "unwind"`. Under `panic = "abort"` a worker panic aborts the
/// process and there is no "after" to gate. When the binary is built with
/// `panic = "abort"`, [`ThreadRegistry::with_reap_backstop`] emits a
/// one-shot `tracing::warn!` so operators can audit the risk.
struct EpilogueGuard<K: RegistryKey> {
    reg: Arc<ThreadRegistry<K>>,
    key: K,
    my_gen: u64,
}

impl<K: RegistryKey> Drop for EpilogueGuard<K> {
    fn drop(&mut self) {
        self.reg.run_epilogue(self.key, self.my_gen);
    }
}

/// RAII guard returned by [`ThreadRegistry::hold_clearing`]. While at
/// least one guard for a key is alive, the registry refuses any
/// `start_thread`/`start_task` for that key. Drop decrements the
/// per-key holder count and removes the entry only when the count
/// reaches zero — so nested or concurrent holders for the same key
/// compose, and the latch stays raised until the LAST guard drops.
/// Drop runs on every exit path, including panic unwinding, so a
/// caller cannot leak the latch by forgetting to call an end function.
pub struct ClearingGuard<K: RegistryKey> {
    reg: Arc<ThreadRegistry<K>>,
    key: K,
}

impl<K: RegistryKey> Drop for ClearingGuard<K> {
    fn drop(&mut self) {
        let mut clearing = self.reg.lock_clearing();
        match clearing.get(&self.key) {
            // Decrement; remove the entry when the last holder drops.
            Some(n) => match NonZeroUsize::new(n.get() - 1) {
                Some(remaining) => {
                    clearing.insert(self.key, remaining);
                }
                None => {
                    clearing.remove(&self.key);
                }
            },
            // Defensive: a balanced hold/drop should always find the
            // entry. A missing entry means someone removed it out of
            // band — refuse to underflow.
            None => {
                debug_assert!(
                    false,
                    "ClearingGuard::drop saw no entry for its key — \
                     someone removed the latch out of band"
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic::{catch_unwind, AssertUnwindSafe};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::mpsc;
    use tokio::runtime::{Builder, Handle};
    use tokio::sync::Barrier;

    type Reg = Arc<ThreadRegistry<&'static str>>;

    /// Start an OS-thread worker that exits cleanly when cancelled. The
    /// runtime handle is captured from the caller's context (the worker
    /// thread is not itself a tokio worker, so it can't fetch its own).
    fn start_clean(reg: &Reg, key: &'static str, cfg: WorkerConfig) {
        let handle = Handle::current();
        reg.start_thread(key, cfg, move |cancel| {
            handle.block_on(async move { cancel.cancelled().await });
        });
    }

    /// Body for a worker wedged in a non-yielding section: blocks on a
    /// channel and ignores its cancellation token (stands in for a thread
    /// stuck in a `Drop` that never observes cancel).
    fn wedged_body(rx: mpsc::Receiver<()>) -> impl FnOnce(CancellationToken) + Send + 'static {
        move |_cancel| {
            let _ = rx.recv();
        }
    }

    fn orphan_len(reg: &Reg) -> usize {
        reg.lock_orphans().len()
    }

    // ----- Group 1: F1 regression -------------------------------------

    /// A `quiesce` whose outer future is dropped (a tiny enclosing
    /// timeout) must re-park the live handle, never drop-and-detach it. The
    /// slot is cleared (`is_running == false`) but the handle lives in
    /// orphans and `any_alive()` stays true.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn quiesce_drop_reparks_handle_not_detach() {
        let reg = ThreadRegistry::<&str>::new();
        let (release_tx, release_rx) = mpsc::channel::<()>();
        reg.start_thread("alpha", WorkerConfig::default(), wedged_body(release_rx));
        assert!(reg.is_running("alpha"));

        // The wedged worker never observes cancel, so the internal 30s
        // budget can't fire here; the tiny outer timeout drops the quiesce
        // future mid-poll. A naive by-value-into-future impl would detach
        // the handle (orphans empty, any_alive false); the slot-owned
        // handle is re-parked instead.
        let result = tokio::time::timeout(Duration::from_millis(100), reg.quiesce("alpha")).await;
        assert!(
            result.is_err(),
            "outer timeout must fire on the wedged worker"
        );

        assert!(reg.any_alive(), "re-parked handle keeps any_alive true");
        assert!(!reg.is_running("alpha"), "slot cleared (cancel taken)");
        assert_eq!(orphan_len(&reg), 1, "handle was re-parked, not detached");
        assert!(!WorkerStatus::Timeout.is_clean());

        // Release + reap: the orphan joins cleanly and liveness clears.
        release_tx.send(()).unwrap();
        assert_eq!(
            reg.reap_orphans(Duration::from_secs(2)).await,
            WorkerStatus::Ok
        );
        assert!(!reg.any_alive());
    }

    /// Internal-budget variant: a wedged worker with a tiny `join_budget`
    /// makes `quiesce` itself time out, re-park, and return `Timeout` (no
    /// outer drop involved).
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn quiesce_internal_budget_timeout_reparks() {
        let reg = ThreadRegistry::<&str>::new();
        let (release_tx, release_rx) = mpsc::channel::<()>();
        let cfg = WorkerConfig {
            join_budget: Duration::from_millis(50),
            ..WorkerConfig::default()
        };
        reg.start_thread("alpha", cfg, wedged_body(release_rx));

        let status = reg.quiesce("alpha").await;
        assert_eq!(status, WorkerStatus::Timeout);
        assert_eq!(orphan_len(&reg), 1);
        assert!(reg.any_alive());
        assert!(!reg.is_running("alpha"));

        release_tx.send(()).unwrap();
        assert_eq!(
            reg.reap_orphans(Duration::from_secs(2)).await,
            WorkerStatus::Ok
        );
        assert!(!reg.any_alive());
    }

    /// A wedged worker reached through the `shutdown()` path: with a tiny
    /// budget it surfaces as `Timeout` in the report, its handle is
    /// re-parked (`detached == 1`, `any_alive`), and the result is
    /// non-clean — never a clean detach.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn shutdown_path_reparks_wedged_worker() {
        let reg = ThreadRegistry::<&str>::new();
        let (release_tx, release_rx) = mpsc::channel::<()>();
        let cfg = WorkerConfig {
            join_budget: Duration::from_millis(50),
            ..WorkerConfig::default()
        };
        reg.start_thread("alpha", cfg, wedged_body(release_rx));

        let report = tokio::time::timeout(Duration::from_secs(10), reg.shutdown())
            .await
            .expect("shutdown must complete within bound");
        assert_eq!(report.per_worker.get("alpha"), Some(&WorkerStatus::Timeout));
        assert_eq!(report.detached, 1, "wedged handle re-parked, survived reap");
        assert!(!report.all_clean());
        assert!(reg.any_alive());

        // Cleanup.
        release_tx.send(()).unwrap();
        let _ = reg.reap_orphans(Duration::from_secs(5)).await;
        assert!(!reg.any_alive());
    }

    // ----- Group 3: registry unit suite -------------------------------

    /// A slow prior-generation thread's epilogue must NOT clear a newer
    /// generation's token. Restarting reaps the prior generation fully (its
    /// epilogue runs); the new generation stays tracked.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn generation_match_epilogue_preserves_new_token() {
        let reg = ThreadRegistry::<&str>::new();
        start_clean(&reg, "beta", WorkerConfig::default()); // gen 1
        assert!(reg.is_running("beta"));

        // Cancel gen 1, then restart. start_thread's reap joins gen 1
        // (running its gen-gated epilogue) before returning, so this is
        // deterministic: if the epilogue ignored generation it would have
        // cleared gen 2's token during that join.
        reg.cancel("beta");
        start_clean(&reg, "beta", WorkerConfig::default()); // gen 2

        assert!(
            reg.is_running("beta"),
            "gen-2 token must survive gen-1's epilogue"
        );
        assert_eq!(reg.quiesce("beta").await, WorkerStatus::Ok);
    }

    /// A worker started with an explicit `WorkerConfig::stack_size` spawns,
    /// runs its body, and joins cleanly — the custom-stack spawn path is
    /// wired through `spawn_os_thread`. (A deliberate stack-overflow test is
    /// avoided: an overflow aborts the whole process, not just the test.)
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn start_thread_honors_custom_stack_size() {
        let reg = ThreadRegistry::<&str>::new();
        let cfg = WorkerConfig {
            stack_size: Some(NonZeroUsize::new(8 * 1024 * 1024).expect("8 MiB is non-zero")),
            ..WorkerConfig::default()
        };
        start_clean(&reg, "big-stack", cfg);
        assert!(reg.is_running("big-stack"));
        assert_eq!(reg.quiesce("big-stack").await, WorkerStatus::Ok);
    }

    /// A naturally-finished prior thread is joined cleanly on restart, with
    /// no parking.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn restart_reaps_finished_prior_without_parking() {
        let reg = ThreadRegistry::<&str>::new();
        start_clean(&reg, "gamma", WorkerConfig::default());
        // Cancel so the prior exits, then restart: the reap must join it,
        // not park it.
        reg.cancel("gamma");
        start_clean(&reg, "gamma", WorkerConfig::default());
        assert_eq!(orphan_len(&reg), 0, "finished prior was joined, not parked");
        assert!(reg.is_running("gamma"));
        assert_eq!(reg.quiesce("gamma").await, WorkerStatus::Ok);
    }

    /// A prior thread wedged past the reap backstop is parked in orphans
    /// (not dropped), then drained after release.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn restart_parks_wedged_prior() {
        let reg = ThreadRegistry::with_reap_backstop(Duration::from_millis(100));
        let (release_tx, release_rx) = mpsc::channel::<()>();

        // gen 1: wedged (ignores cancel).
        reg.start_thread("delta", WorkerConfig::default(), wedged_body(release_rx));
        reg.cancel("delta");

        // gen 2: clean. The restart reaps gen 1 — wedged past the 100ms
        // backstop, so it is parked. Run off the runtime workers since the
        // reap spins synchronously.
        let reg_for_start = Arc::clone(&reg);
        let parent = Handle::current();
        tokio::task::spawn_blocking(move || {
            let handle = parent.clone();
            reg_for_start.start_thread("delta", WorkerConfig::default(), move |cancel| {
                handle.block_on(async move { cancel.cancelled().await });
            });
        })
        .await
        .unwrap();

        assert_eq!(orphan_len(&reg), 1, "wedged prior parked, not dropped");
        assert!(reg.any_alive());
        assert!(reg.is_running("delta"), "gen-2 loop started");

        // Release the wedged prior; reap drains it.
        release_tx.send(()).unwrap();
        assert_eq!(
            reg.reap_orphans(Duration::from_secs(2)).await,
            WorkerStatus::Ok
        );
        assert_eq!(orphan_len(&reg), 0);

        // Cleanup gen 2.
        assert_eq!(reg.quiesce("delta").await, WorkerStatus::Ok);
    }

    /// Orphan drain: a survivor at the grace deadline is reported
    /// `Detached` and re-parked; once released it reaps `Ok`.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn orphan_drain_detached_then_ok() {
        let reg = ThreadRegistry::<&str>::new();
        let (release_tx, release_rx) = mpsc::channel::<()>();
        let wedged = std::thread::spawn(move || {
            let _ = release_rx.recv();
        });
        reg.park_orphan_for_test("orphan", wedged);

        assert_eq!(
            reg.reap_orphans(Duration::from_millis(50)).await,
            WorkerStatus::Detached
        );
        assert_eq!(orphan_len(&reg), 1, "survivor re-parked for retry");
        assert!(reg.any_alive());

        release_tx.send(()).unwrap();
        assert_eq!(
            reg.reap_orphans(Duration::from_secs(2)).await,
            WorkerStatus::Ok
        );
        assert_eq!(orphan_len(&reg), 0);
        assert!(!reg.any_alive());
    }

    /// A reaped orphan whose body PANICKED (finishes within the reap grace,
    /// so `detached == 0`) must surface in `ShutdownReport::orphan_status`
    /// and flip `all_clean()`.
    ///
    /// Non-vacuous: `orphan_status` is the only place a panicked-but-reaped
    /// orphan is observable — without it, an empty/clean `per_worker` plus
    /// `detached == 0` would let `all_clean()` return true and swallow the
    /// panic.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn shutdown_report_surfaces_panicked_reaped_orphan() {
        let reg = ThreadRegistry::<&str>::new();
        // Park a panicking thread directly as an orphan via the test seam.
        // The body panics immediately, so the thread is finished by the
        // time `shutdown()` runs the reap and classifies as `Panicked`.
        let panicker = std::thread::spawn(|| {
            panic!("deliberate orphan-body panic");
        });
        reg.park_orphan_for_test("k", panicker);

        let report = reg.shutdown().await;

        assert_eq!(
            report.detached, 0,
            "panicked orphan finished within the reap grace"
        );
        assert!(
            matches!(report.orphan_status, WorkerStatus::Panicked(_)),
            "reaped orphan's panic must surface in orphan_status, got {:?}",
            report.orphan_status
        );
        assert!(
            !report.all_clean(),
            "all_clean() must reflect the panicked reaped orphan, not pass it"
        );
        assert!(!reg.any_alive());
    }

    /// Complement: a clean reaped orphan (`Ok`) leaves `all_clean()` true,
    /// so the orphan-status fold doesn't over-trigger on the common case of
    /// orphans that drained cleanly within the grace.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn shutdown_report_clean_reaped_orphan_is_clean() {
        let reg = ThreadRegistry::<&str>::new();
        let clean = std::thread::spawn(|| { /* exits cleanly */ });
        reg.park_orphan_for_test("k", clean);

        let report = reg.shutdown().await;
        assert_eq!(report.detached, 0);
        assert_eq!(report.orphan_status, WorkerStatus::Ok);
        assert!(report.all_clean());
    }

    /// Weight-ordered shutdown drains a lower tier before a higher one.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn weight_ordered_shutdown_drains_low_first() {
        let reg = ThreadRegistry::<&str>::new();
        let log = Arc::new(Mutex::new(Vec::<&'static str>::new()));

        let mk_hook = |tag: &'static str, log: Arc<Mutex<Vec<&'static str>>>| -> DrainHook {
            Arc::new(move || {
                let log = Arc::clone(&log);
                Box::pin(async move {
                    log.lock().unwrap().push(tag);
                })
            })
        };

        start_clean(
            &reg,
            "w0",
            WorkerConfig {
                weight: ShutdownWeight(0),
                drain: Some(mk_hook("w0", Arc::clone(&log))),
                ..WorkerConfig::default()
            },
        );
        start_clean(
            &reg,
            "w5",
            WorkerConfig {
                weight: ShutdownWeight(5),
                drain: Some(mk_hook("w5", Arc::clone(&log))),
                ..WorkerConfig::default()
            },
        );
        start_clean(
            &reg,
            "w10",
            WorkerConfig {
                weight: ShutdownWeight(10),
                drain: Some(mk_hook("w10", Arc::clone(&log))),
                ..WorkerConfig::default()
            },
        );

        let report = reg.shutdown().await;
        assert!(report.all_clean());

        let log = log.lock().unwrap();
        let pos = |tag| log.iter().position(|t| *t == tag).unwrap();
        assert!(pos("w0") < pos("w5"));
        assert!(pos("w5") < pos("w10"));
    }

    /// Equal-weight workers drain concurrently. A shared `Barrier(2)` in
    /// both drain hooks would deadlock under sequential draining (caught by
    /// the enclosing timeout); the event log proves both arrived before
    /// either passed.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn equal_weight_drains_concurrently() {
        let reg = ThreadRegistry::<&str>::new();
        let log = Arc::new(Mutex::new(Vec::<&'static str>::new()));
        let barrier = Arc::new(Barrier::new(2));

        let mk_hook = |arrived: &'static str,
                       passed: &'static str,
                       log: Arc<Mutex<Vec<&'static str>>>,
                       barrier: Arc<Barrier>|
         -> DrainHook {
            Arc::new(move || {
                let log = Arc::clone(&log);
                let barrier = Arc::clone(&barrier);
                Box::pin(async move {
                    log.lock().unwrap().push(arrived);
                    barrier.wait().await;
                    log.lock().unwrap().push(passed);
                })
            })
        };

        start_clean(
            &reg,
            "a",
            WorkerConfig {
                weight: ShutdownWeight(0),
                drain: Some(mk_hook(
                    "a_arrived",
                    "a_passed",
                    Arc::clone(&log),
                    Arc::clone(&barrier),
                )),
                ..WorkerConfig::default()
            },
        );
        start_clean(
            &reg,
            "b",
            WorkerConfig {
                weight: ShutdownWeight(0),
                drain: Some(mk_hook(
                    "b_arrived",
                    "b_passed",
                    Arc::clone(&log),
                    Arc::clone(&barrier),
                )),
                ..WorkerConfig::default()
            },
        );

        let report = tokio::time::timeout(Duration::from_secs(5), reg.shutdown())
            .await
            .expect("equal-weight drain must not deadlock (proves concurrency)");
        assert!(report.all_clean());

        let log = log.lock().unwrap();
        let pos = |tag| log.iter().position(|t| *t == tag).unwrap();
        let last_arrived = pos("a_arrived").max(pos("b_arrived"));
        let first_passed = pos("a_passed").min(pos("b_passed"));
        assert!(
            last_arrived < first_passed,
            "both hooks must reach the barrier before either passes: {log:?}"
        );
    }

    /// `any_alive()` accounts for both live slots and orphans.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn any_alive_spans_slots_and_orphans() {
        let reg = ThreadRegistry::<&str>::new();
        start_clean(&reg, "alpha", WorkerConfig::default());
        assert!(reg.any_alive());

        let (release_tx, release_rx) = mpsc::channel::<()>();
        let wedged = std::thread::spawn(move || {
            let _ = release_rx.recv();
        });
        reg.park_orphan_for_test("orphan", wedged);
        assert!(reg.any_alive());

        assert_eq!(reg.quiesce("alpha").await, WorkerStatus::Ok);
        assert!(
            reg.any_alive(),
            "orphan still contributes after slot drains"
        );
        assert!(!reg.is_running("alpha"));

        release_tx.send(()).unwrap();
        let _ = reg.reap_orphans(Duration::from_secs(2)).await;
        assert!(!reg.any_alive());
    }

    /// `any_alive_for(key)` is scoped: an orphan parked under one key does
    /// not make a different key look alive (the F2 gate must not be
    /// blocked by unrelated workers).
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn any_alive_for_is_key_scoped() {
        let reg = ThreadRegistry::<&str>::new();
        let (release_tx, release_rx) = mpsc::channel::<()>();
        let wedged = std::thread::spawn(move || {
            let _ = release_rx.recv();
        });
        reg.park_orphan_for_test("shielded", wedged);

        // A live, unrelated worker.
        start_clean(&reg, "identity", WorkerConfig::default());

        assert!(reg.any_alive(), "registry-wide liveness sees both");
        assert!(reg.any_alive_for("shielded"), "shielded orphan is alive");
        assert!(
            !reg.any_alive_for("address"),
            "an unrelated key with no slot/orphan is not alive"
        );

        // The running 'identity' worker must not make 'shielded' look alive
        // beyond its own orphan, and vice versa.
        assert!(reg.any_alive_for("identity"), "running identity is alive");

        release_tx.send(()).unwrap();
        let _ = reg.reap_orphans(Duration::from_secs(2)).await;
        assert!(
            !reg.any_alive_for("shielded"),
            "shielded clear once its orphan is reaped"
        );
        assert_eq!(reg.quiesce("identity").await, WorkerStatus::Ok);
    }

    /// `shutdown()` panics with a documented message on a current-thread
    /// runtime.
    #[test]
    fn shutdown_asserts_multi_thread_runtime() {
        let rt = Builder::new_current_thread().enable_all().build().unwrap();
        let reg = ThreadRegistry::<&str>::new();
        let result = catch_unwind(AssertUnwindSafe(|| {
            // We're proving `shutdown()` panics — its return value is
            // moot here, but `#[must_use]` requires an explicit drop.
            let _ = rt.block_on(async { reg.shutdown().await });
        }));
        let payload = result.expect_err("shutdown must panic on current_thread");
        let msg = payload
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| payload.downcast_ref::<&str>().copied())
            .unwrap_or("");
        assert!(
            msg.contains("multi-thread"),
            "panic must name the runtime constraint, got: {msg}"
        );
    }

    // ----- Group 4: DrainHook ordering --------------------------------

    /// The drain hook is fully awaited before the cancel signal is observed
    /// by the worker.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn drain_hook_completes_before_cancel() {
        let reg = ThreadRegistry::<&str>::new();
        let log = Arc::new(Mutex::new(Vec::<&'static str>::new()));

        let log_hook = Arc::clone(&log);
        let drain: DrainHook = Arc::new(move || {
            let log = Arc::clone(&log_hook);
            Box::pin(async move {
                log.lock().unwrap().push("drain_hook_start");
                tokio::time::sleep(Duration::from_millis(10)).await;
                log.lock().unwrap().push("drain_hook_complete");
            })
        });

        let log_worker = Arc::clone(&log);
        let handle = Handle::current();
        reg.start_thread(
            "epsilon",
            WorkerConfig {
                drain: Some(drain),
                ..WorkerConfig::default()
            },
            move |cancel| {
                handle.block_on(async move {
                    cancel.cancelled().await;
                    log_worker.lock().unwrap().push("cancel_observed");
                });
            },
        );

        assert_eq!(reg.quiesce("epsilon").await, WorkerStatus::Ok);
        assert!(!reg.is_running("epsilon"));

        let log = log.lock().unwrap();
        let pos = |tag| log.iter().position(|t| *t == tag).unwrap();
        assert!(pos("drain_hook_start") < pos("drain_hook_complete"));
        assert!(pos("drain_hook_complete") < pos("cancel_observed"));
    }

    /// A `quiesce` blocks in the drain hook until an `is_syncing` barrier
    /// the hook polls falls, and only then cancels + joins.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn drain_hook_observes_barrier_before_join() {
        let reg = ThreadRegistry::<&str>::new();
        let is_syncing = Arc::new(AtomicBool::new(true));

        let gate = Arc::clone(&is_syncing);
        let drain: DrainHook = Arc::new(move || {
            let gate = Arc::clone(&gate);
            Box::pin(async move {
                while gate.load(Ordering::Acquire) {
                    tokio::time::sleep(Duration::from_millis(5)).await;
                }
            })
        });
        start_clean(
            &reg,
            "zeta",
            WorkerConfig {
                drain: Some(drain),
                ..WorkerConfig::default()
            },
        );

        let quiesce_completed = Arc::new(AtomicBool::new(false));
        let reg_q = Arc::clone(&reg);
        let done = Arc::clone(&quiesce_completed);
        let quiesce_task = tokio::spawn(async move {
            let status = reg_q.quiesce("zeta").await;
            done.store(true, Ordering::Release);
            status
        });

        // While the barrier is held, quiesce must stay pending.
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(
            !quiesce_completed.load(Ordering::Acquire),
            "quiesce must block while is_syncing is held"
        );

        // Release the barrier; quiesce drains, cancels, joins.
        is_syncing.store(false, Ordering::Release);
        let status = tokio::time::timeout(Duration::from_secs(2), quiesce_task)
            .await
            .expect("quiesce must complete once the barrier falls")
            .unwrap();
        assert_eq!(status, WorkerStatus::Ok);
        assert!(quiesce_completed.load(Ordering::Acquire));
    }

    // ----- Group 5: status classification -----------------------------

    /// Only the `Task` kind can classify as `Stopped` (from a runtime-level
    /// cancel/abort JoinError); a cooperatively token-cancelled task exits
    /// normally as `Ok`. Verifies the kind-dispatch at the classification
    /// boundary.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn task_kind_classifies_stopped_and_ok() {
        // Stopped: an aborted task yields a cancelled JoinError.
        let aborted = tokio::spawn(std::future::pending::<()>());
        aborted.abort();
        while !aborted.is_finished() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        let status = WorkerHandle::Task(aborted).classify();
        assert!(matches!(status, WorkerStatus::Stopped(_)), "got {status:?}");
        assert!(!status.is_clean());

        // Ok: a cooperatively token-cancelled task returns normally.
        let reg = ThreadRegistry::<&str>::new();
        reg.start_task("task_a", WorkerConfig::default(), |cancel| async move {
            cancel.cancelled().await;
        });
        assert_eq!(reg.quiesce("task_a").await, WorkerStatus::Ok);
        assert!(!reg.is_running("task_a"));
    }

    /// An `OsThread` worker yields `Ok` (clean) or `Panicked` (`&str` and
    /// `String` payloads), never `Stopped`.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn os_thread_ok_and_panicked_never_stopped() {
        let reg = ThreadRegistry::<&str>::new();
        start_clean(&reg, "os_clean", WorkerConfig::default());
        let ok = reg.quiesce("os_clean").await;
        assert_eq!(ok, WorkerStatus::Ok);
        assert!(ok.is_clean());

        // &str panic payload.
        reg.start_thread("os_panic_str", WorkerConfig::default(), |_cancel| {
            panic!("deliberate test panic");
        });
        match reg.quiesce("os_panic_str").await {
            WorkerStatus::Panicked(msg) => assert!(msg.contains("deliberate test panic")),
            other => panic!("expected Panicked, got {other:?}"),
        }

        // String panic payload.
        reg.start_thread("os_panic_string", WorkerConfig::default(), |_cancel| {
            std::panic::panic_any(String::from("deliberate string panic"));
        });
        match reg.quiesce("os_panic_string").await {
            WorkerStatus::Panicked(msg) => assert!(msg.contains("deliberate string panic")),
            other => panic!("expected Panicked, got {other:?}"),
        }
    }

    // ----- Gaps -------------------------------------------------------

    /// `shutdown()` is idempotent: a second call finds every slot already
    /// joined and reports `NotRunning`, still clean.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn shutdown_is_idempotent() {
        let reg = ThreadRegistry::<&str>::new();
        start_clean(&reg, "alpha", WorkerConfig::default());

        let first = reg.shutdown().await;
        assert_eq!(first.per_worker.get("alpha"), Some(&WorkerStatus::Ok));
        assert!(first.all_clean());

        let second = reg.shutdown().await;
        assert_eq!(
            second.per_worker.get("alpha"),
            Some(&WorkerStatus::NotRunning)
        );
        assert!(second.all_clean());
    }

    /// `cancel(key)` is selective: cancelling A does not touch B.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn cancel_is_selective() {
        let reg = ThreadRegistry::<&str>::new();
        start_clean(&reg, "a", WorkerConfig::default());
        start_clean(&reg, "b", WorkerConfig::default());

        reg.cancel("a");
        assert!(reg.is_running("b"), "cancel(a) must not cancel b");
        assert_eq!(reg.quiesce("a").await, WorkerStatus::Ok);
        assert!(reg.is_running("b"), "b still running after a drains");
        assert_eq!(reg.quiesce("b").await, WorkerStatus::Ok);
    }

    /// `cancel_all()` cancels every registered worker in one call; a
    /// subsequent `quiesce` per key drains each one cleanly. Covers the
    /// public method that has no in-tree caller yet (the rs-dapi-client
    /// adoption will use it).
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn cancel_all_signals_every_worker() {
        let reg = ThreadRegistry::<&str>::new();
        start_clean(&reg, "a", WorkerConfig::default());
        start_clean(&reg, "b", WorkerConfig::default());
        start_clean(&reg, "c", WorkerConfig::default());
        assert!(reg.is_running("a") && reg.is_running("b") && reg.is_running("c"));

        reg.cancel_all();
        assert!(!reg.is_running("a"));
        assert!(!reg.is_running("b"));
        assert!(!reg.is_running("c"));

        // All three drain cleanly — the cancel reached every worker.
        assert_eq!(reg.quiesce("a").await, WorkerStatus::Ok);
        assert_eq!(reg.quiesce("b").await, WorkerStatus::Ok);
        assert_eq!(reg.quiesce("c").await, WorkerStatus::Ok);
        assert!(!reg.any_alive());
    }

    /// `WorkerConfig::default()` values are pinned.
    #[test]
    fn worker_config_defaults_pinned() {
        let cfg = WorkerConfig::default();
        assert_eq!(cfg.weight, ShutdownWeight(0));
        assert!(cfg.drain.is_none());
        assert_eq!(cfg.join_budget, DEFAULT_JOIN_BUDGET);
        assert!(cfg.stack_size.is_none());
    }

    /// `hold_clearing(key)` refuses both `start_thread` and `start_task`
    /// for that key, but ONLY for that key — other keys are unaffected.
    /// After the guard drops the latch releases and starts succeed again.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn hold_clearing_blocks_starts_for_latched_key_only() {
        let reg = ThreadRegistry::<&str>::new();
        let _gate = reg.hold_clearing("shielded");

        // start_thread on the latched key is a no-op.
        start_clean(&reg, "shielded", WorkerConfig::default());
        assert!(
            !reg.is_running("shielded"),
            "start_thread must be refused while the key is latched"
        );

        // start_task on the latched key is also a no-op.
        reg.start_task("shielded", WorkerConfig::default(), |cancel| async move {
            cancel.cancelled().await;
        });
        assert!(
            !reg.is_running("shielded"),
            "start_task must be refused while the key is latched"
        );

        // An unrelated key starts cleanly — the latch is per-key.
        start_clean(&reg, "identity", WorkerConfig::default());
        assert!(reg.is_running("identity"));

        // Drop the latch; the same key now starts.
        drop(_gate);
        start_clean(&reg, "shielded", WorkerConfig::default());
        assert!(
            reg.is_running("shielded"),
            "latch release allows the key to start again"
        );

        // Cleanup.
        assert_eq!(reg.quiesce("shielded").await, WorkerStatus::Ok);
        assert_eq!(reg.quiesce("identity").await, WorkerStatus::Ok);
    }

    /// Refcounted nesting: two concurrent / nested holders for the same
    /// key keep the latch raised until the LAST guard drops. The inner
    /// guard's drop must NOT release the latch the outer still holds —
    /// otherwise a re-entrant or concurrent caller silently lapses the
    /// invariant. The `start_clean` between `drop(inner)` and `drop(outer)`
    /// below stays refused and `is_clearing` stays true, proving the outer
    /// holder's protection survives the inner drop.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn hold_clearing_inner_drop_does_not_lapse_outer_protection() {
        let reg = ThreadRegistry::<&str>::new();
        let outer = reg.hold_clearing("shielded");
        let inner = reg.hold_clearing("shielded");

        // While both guards are live, starts are refused and the latch
        // reports clearing.
        start_clean(&reg, "shielded", WorkerConfig::default());
        assert!(!reg.is_running("shielded"));
        assert!(reg.is_clearing("shielded"));

        // Drop the INNER guard while the outer is still alive.
        drop(inner);

        // The outer protection MUST survive: the latch is still raised
        // and a fresh start is still refused.
        assert!(
            reg.is_clearing("shielded"),
            "outer ClearingGuard must keep the latch raised after the inner drops"
        );
        start_clean(&reg, "shielded", WorkerConfig::default());
        assert!(
            !reg.is_running("shielded"),
            "start must still be refused while the outer guard is alive"
        );

        // After the outer drops too, the latch fully releases.
        drop(outer);
        assert!(!reg.is_clearing("shielded"));
        assert_eq!(reg.lock_clearing().len(), 0);

        // And a fresh start succeeds.
        start_clean(&reg, "shielded", WorkerConfig::default());
        assert!(reg.is_running("shielded"));
        assert_eq!(reg.quiesce("shielded").await, WorkerStatus::Ok);
    }

    /// The latch holds across panic unwinding (RAII guarantee). A
    /// closure that holds the guard and panics still removes the key
    /// from the clearing set on its drop path.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn hold_clearing_releases_on_panic_unwind() {
        let reg = ThreadRegistry::<&str>::new();
        let reg_clone = Arc::clone(&reg);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _gate = reg_clone.hold_clearing("shielded");
            assert_eq!(reg_clone.lock_clearing().len(), 1);
            panic!("simulated clear-flow panic");
        }));
        assert!(result.is_err());
        assert_eq!(
            reg.lock_clearing().len(),
            0,
            "ClearingGuard's Drop must release the latch even when the clear flow panics"
        );

        // The key is startable again post-panic.
        start_clean(&reg, "shielded", WorkerConfig::default());
        assert!(reg.is_running("shielded"));
        assert_eq!(reg.quiesce("shielded").await, WorkerStatus::Ok);
    }

    // ----- Group 6: concurrency-hazard regressions --------------------

    /// `quiesce` is generation-guarded. A same-key restart that lands after
    /// quiesce takes the prior's cancel must not have its fresh, live handle
    /// parked or reported `Timeout`: the superseded quiesce returns
    /// `NotRunning` and the new generation survives.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn quiesce_generation_guard_spares_concurrent_restart() {
        let reg = ThreadRegistry::<&str>::new();
        // gen-1: a task that ignores cancellation (pending forever), with a
        // tiny join budget so a non-guarded quiesce would Timeout quickly.
        reg.start_task(
            "k",
            WorkerConfig {
                join_budget: Duration::from_millis(150),
                ..WorkerConfig::default()
            },
            |_cancel| async move { std::future::pending::<()>().await },
        );

        // Drive quiesce concurrently; it snapshots gen=1, cancels (ignored),
        // and enters the poll loop with cancel already taken.
        let reg_q = Arc::clone(&reg);
        let q = tokio::spawn(async move { reg_q.quiesce("k").await });

        // Let quiesce pass cancel.take() so a restart can proceed.
        tokio::time::sleep(Duration::from_millis(40)).await;

        // Restart: cancel is now None, so this proceeds — it takes gen-1's
        // live handle as its prior (parked) and installs gen-2.
        reg.start_task("k", WorkerConfig::default(), |cancel| async move {
            cancel.cancelled().await;
        });

        // The superseded quiesce must NOT park gen-2 / report Timeout.
        let status = q.await.unwrap();
        assert_eq!(
            status,
            WorkerStatus::NotRunning,
            "superseded quiesce returns NotRunning, never a spurious Timeout"
        );
        assert!(reg.is_running("k"), "gen-2 survives the racing quiesce");

        // gen-2 quiesces cleanly.
        assert_eq!(reg.quiesce("k").await, WorkerStatus::Ok);
    }

    /// A thread-spawn failure must neither panic nor detach the live prior
    /// handle: it rolls back (prior re-installed, running flag cleared) and
    /// the slot stays usable / reapable.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn spawn_failure_reparks_live_prior_without_panic() {
        let reg = ThreadRegistry::<&str>::new();
        let (release_tx, release_rx) = mpsc::channel::<()>();
        // gen-1: wedged (ignores cancel), stays live until released.
        reg.start_thread("k", WorkerConfig::default(), wedged_body(release_rx));
        // cancel() takes the token (slot.cancel = None) but the wedged thread
        // keeps running — the slot now holds a LIVE prior handle with cancel
        // cleared, the exact shape a racing restart would take as its prior.
        reg.cancel("k");
        assert!(!reg.is_running("k"));

        // Force the restart's spawn to fail; it must not panic.
        reg.force_spawn_failure.store(true, Ordering::Release);
        reg.start_thread("k", WorkerConfig::default(), |_cancel| {});
        assert!(
            !reg.is_running("k"),
            "failed spawn clears the running flag, never leaves it wedged"
        );
        assert!(reg.any_alive(), "live prior re-installed, never detached");

        // Recover: release the prior; quiesce reaps the now-finished handle
        // cleanly, proving it was owned (not leaked/detached) and the slot is
        // not wedged.
        reg.force_spawn_failure.store(false, Ordering::Release);
        release_tx.send(()).unwrap();
        assert_eq!(reg.quiesce("k").await, WorkerStatus::Ok);
        assert!(!reg.any_alive());
    }

    /// A thread-spawn failure must roll the slot back to its PRIOR config, not
    /// leave the failed start's weight / drain / join_budget / generation
    /// behind: the re-installed prior worker keeps its own teardown config for
    /// a later quiesce/shutdown.
    ///
    /// Non-vacuous: against a partial rollback (only cancel/handle restored),
    /// the slot would carry the failed start's weight/budget, a `None` drain,
    /// and the bumped generation.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn spawn_failure_restores_prior_slot_config() {
        let reg = ThreadRegistry::<&str>::new();
        let (release_tx, release_rx) = mpsc::channel::<()>();

        // gen-1 with a DISTINCTIVE config (drain hook + non-default weight and
        // join budget). Wedged so it stays the live prior after cancel.
        let hook: DrainHook = Arc::new(|| Box::pin(async {}));
        let cfg1 = WorkerConfig {
            weight: ShutdownWeight(7),
            join_budget: Duration::from_secs(11),
            drain: Some(hook),
            ..WorkerConfig::default()
        };
        reg.start_thread("k", cfg1, wedged_body(release_rx));
        reg.cancel("k");
        let gen_after_gen1 = reg.lock_slots().get("k").unwrap().generation;

        // Failed restart with a DIFFERENT config; the rollback must discard it.
        reg.force_spawn_failure.store(true, Ordering::Release);
        let cfg2 = WorkerConfig {
            weight: ShutdownWeight(99),
            join_budget: Duration::from_secs(99),
            drain: None,
            ..WorkerConfig::default()
        };
        reg.start_thread("k", cfg2, |_cancel| {});
        reg.force_spawn_failure.store(false, Ordering::Release);

        {
            let slots = reg.lock_slots();
            let slot = slots.get("k").expect("slot present");
            assert_eq!(slot.weight, ShutdownWeight(7), "weight restored to prior");
            assert_eq!(
                slot.join_budget,
                Duration::from_secs(11),
                "join_budget restored to prior"
            );
            assert!(
                slot.drain.is_some(),
                "prior drain hook restored, not the failed start's None"
            );
            assert_eq!(
                slot.generation, gen_after_gen1,
                "generation rolled back to its pre-bump value"
            );
            assert!(
                slot.cancel.is_none(),
                "running flag cleared after failed spawn"
            );
            assert!(
                slot.handle.is_some(),
                "prior handle re-installed (alive), not detached"
            );
        }
        assert!(reg.any_alive(), "live prior still accounted for");

        // Recover: release + quiesce reaps the prior cleanly.
        release_tx.send(()).unwrap();
        assert_eq!(reg.quiesce("k").await, WorkerStatus::Ok);
        assert!(!reg.any_alive());
    }

    /// A panicking worker body still runs its epilogue (via the drop-guard),
    /// so `is_running()` reflects the crash and `start()` can relaunch the
    /// loop instead of silently no-op'ing.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn panicked_worker_clears_running_and_allows_restart() {
        let reg = ThreadRegistry::<&str>::new();
        // A worker whose body panics immediately.
        reg.start_thread("k", WorkerConfig::default(), |_cancel| {
            panic!("deliberate worker-body panic");
        });

        // The drop-guard epilogue clears the running flag despite the panic.
        let mut waited = Duration::ZERO;
        while reg.is_running("k") && waited < Duration::from_secs(2) {
            tokio::time::sleep(Duration::from_millis(5)).await;
            waited += Duration::from_millis(5);
        }
        assert!(
            !reg.is_running("k"),
            "panicked worker clears its running flag via the epilogue guard"
        );

        // start() can relaunch a crashed loop.
        let ran = Arc::new(AtomicBool::new(false));
        let ran_w = Arc::clone(&ran);
        let handle = Handle::current();
        reg.start_thread("k", WorkerConfig::default(), move |cancel| {
            ran_w.store(true, Ordering::Release);
            handle.block_on(async move { cancel.cancelled().await });
        });
        assert!(
            reg.is_running("k"),
            "start() relaunches a previously-panicked worker"
        );
        assert_eq!(reg.quiesce("k").await, WorkerStatus::Ok);
        assert!(
            ran.load(Ordering::Acquire),
            "restarted worker body executed"
        );
    }

    /// `shutdown()` latches the registry closed: a start racing (or
    /// following) teardown is refused, so no worker is left un-joined.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn shutdown_latches_closed_refusing_new_workers() {
        let reg = ThreadRegistry::<&str>::new();
        start_clean(&reg, "live", WorkerConfig::default());
        let report = reg.shutdown().await;
        assert!(report.all_clean());

        // One-way door: both worker kinds are refused after shutdown.
        start_clean(&reg, "late_thread", WorkerConfig::default());
        assert!(
            !reg.is_running("late_thread"),
            "start_thread after shutdown is refused"
        );
        reg.start_task("late_task", WorkerConfig::default(), |cancel| async move {
            cancel.cancelled().await;
        });
        assert!(
            !reg.is_running("late_task"),
            "start_task after shutdown is refused"
        );
        assert!(!reg.any_alive(), "nothing started post-shutdown");
    }

    /// `start_thread` must park a restarted key's still-wedged prior into the
    /// orphan list UNDER the slot lock — at the START of the restart, not only
    /// after the out-of-lock reap backstop elapses.
    /// Otherwise a `shutdown()` that snapshots tiers in the window between
    /// "prior taken out of the slot" and "prior parked" sees neither the
    /// prior (already moved out of the slot) nor an orphan, and reports
    /// clean while the wedged prior is still live and un-joined.
    ///
    /// Deterministic via a long backstop: parking under the slot lock makes
    /// the prior observable in orphans well before the backstop could elapse,
    /// so the early assertion lands. Parking only at the end of the
    /// out-of-lock spin would fail it.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn start_thread_parks_wedged_prior_under_slot_lock_at_restart() {
        // Long backstop so the under-lock parking is observable well before
        // it could possibly elapse.
        let reg = ThreadRegistry::with_reap_backstop(Duration::from_secs(10));
        let (release_tx, release_rx) = mpsc::channel::<()>();

        // gen-1: wedged (ignores cancel), stays live until released.
        reg.start_thread("k", WorkerConfig::default(), wedged_body(release_rx));
        reg.cancel("k");

        // gen-2 restart on a blocking thread: its bounded reap of the wedged
        // gen-1 spins the (long) backstop, so start_thread does not return
        // promptly. gen-1 is parked under the slot lock at the start of
        // this call, before that spin.
        let reg2 = Arc::clone(&reg);
        let parent = Handle::current();
        let restart = tokio::task::spawn_blocking(move || {
            let handle = parent.clone();
            reg2.start_thread("k", WorkerConfig::default(), move |cancel| {
                handle.block_on(async move { cancel.cancelled().await });
            });
        });

        // The wedged prior must appear in orphans far sooner than the 10s
        // backstop — it was parked under the slot lock at restart.
        let mut waited = Duration::ZERO;
        while orphan_len(&reg) == 0 && waited < Duration::from_secs(2) {
            tokio::time::sleep(Duration::from_millis(10)).await;
            waited += Duration::from_millis(10);
        }
        assert_eq!(
            orphan_len(&reg),
            1,
            "wedged prior must be parked under the slot lock at restart, not \
             only after the backstop spin"
        );
        assert!(reg.is_running("k"), "gen-2 installed under the same lock");

        // Release the wedged prior: the restart's bounded reap then finds it
        // finished, removes it from orphans, and joins it.
        release_tx.send(()).unwrap();
        restart.await.unwrap();
        assert_eq!(
            orphan_len(&reg),
            0,
            "finished prior removed from orphans by the bounded reap"
        );

        // gen-2 quiesces cleanly.
        assert_eq!(reg.quiesce("k").await, WorkerStatus::Ok);
    }

    /// `with_reap_backstop` MUST emit a one-shot `tracing::warn!` when
    /// compiled under `panic = "abort"` so an operator can audit the
    /// orphan-liveness-gate risk documented on `EpilogueGuard`.
    ///
    /// Aspirational / manual-only: the standard `cargo test` profile is
    /// `panic = "unwind"`, so this test is cfg-compiled OUT of every normal CI
    /// run. It exercises the warn path only under a deliberate
    /// `RUSTFLAGS="-C panic=abort"` build (mirroring the iOS release profile);
    /// treat it as a local audit tool, not a signal CI enforces on its own.
    ///
    /// Functional assertion is on the process-wide `Once` latch, which is
    /// the most reliable artifact we can probe without subscribing to
    /// tracing from a `#[test]`.
    #[cfg(panic = "abort")]
    #[test]
    fn with_reap_backstop_emits_panic_abort_warn_under_abort_builds() {
        let _reg = ThreadRegistry::<&'static str>::with_reap_backstop(Duration::from_secs(1));
        assert!(
            super::PANIC_ABORT_WARNED.is_completed(),
            "with_reap_backstop must trip the panic=abort warn latch on first call"
        );
        // Second construction must NOT re-fire — `Once` guarantees this, but
        // we exercise it to lock the one-shot contract into the test.
        let _reg2 = ThreadRegistry::<&'static str>::with_reap_backstop(Duration::from_secs(1));
        assert!(super::PANIC_ABORT_WARNED.is_completed());
    }

    /// Sentinel for the no-op cfg branch: under `panic = "unwind"` (the
    /// dev-profile default) `EpilogueGuard`'s `Drop` runs and releases the
    /// orphan slot, so the operator warn is unnecessary. This test just
    /// proves the unwind branch compiles and `with_reap_backstop` keeps
    /// behaving like a plain constructor — no observable warn-related state
    /// to assert because the gated `static` doesn't exist on this build.
    #[cfg(not(panic = "abort"))]
    #[test]
    fn with_reap_backstop_no_warn_under_unwind() {
        let reg = ThreadRegistry::<&'static str>::with_reap_backstop(Duration::from_millis(250));
        assert!(!reg.any_alive(), "fresh registry has no live workers");
    }

    // ----- Group: register_thread (join/status-only, token-less) ------

    /// Spawn an OS thread that blocks until its channel is released, so a
    /// test can hold it "live" and then let it exit cleanly on demand.
    fn spawn_gated(rx: mpsc::Receiver<()>) -> std::thread::JoinHandle<()> {
        std::thread::spawn(move || {
            let _ = rx.recv();
        })
    }

    /// A registered (externally-owned) handle is joined and classified
    /// `Ok` by `shutdown`, and — because no cancel token is installed —
    /// `is_running` stays `false` while `any_alive_for` still tracks the
    /// live handle.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn register_thread_join_reports_ok() {
        let reg = ThreadRegistry::<&str>::new();
        let (tx, rx) = mpsc::channel::<()>();
        reg.register_thread("alpha", WorkerConfig::default(), spawn_gated(rx));

        assert!(
            !reg.is_running("alpha"),
            "register_thread installs no token, so is_running stays false"
        );
        assert!(
            reg.any_alive_for("alpha"),
            "the live handle is tracked for liveness gating"
        );

        drop(tx); // release the worker so its join lands cleanly
        let report = reg.shutdown().await;
        assert_eq!(report.per_worker.get("alpha"), Some(&WorkerStatus::Ok));
        assert!(report.all_clean(), "clean join: {report:?}");
    }

    /// A restart (`register_thread` while a prior handle is still live)
    /// parks the prior as an orphan rather than detaching it; teardown then
    /// joins the new slot handle AND reaps the parked prior, both cleanly.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn register_thread_restart_parks_prior_then_teardown_reaps_both() {
        let reg = ThreadRegistry::<&str>::with_reap_backstop(Duration::from_millis(50));
        let (tx_a, rx_a) = mpsc::channel::<()>();
        reg.register_thread("alpha", WorkerConfig::default(), spawn_gated(rx_a));

        // Restart B while A is still wedged: A is parked (the bounded reap
        // can't join it within the short backstop, so it stays parked).
        let (tx_b, rx_b) = mpsc::channel::<()>();
        reg.register_thread("alpha", WorkerConfig::default(), spawn_gated(rx_b));
        assert_eq!(
            orphan_len(&reg),
            1,
            "prior A parked as an orphan on restart"
        );

        drop(tx_a);
        drop(tx_b);
        let report = reg.shutdown().await;
        assert_eq!(report.per_worker.get("alpha"), Some(&WorkerStatus::Ok));
        assert!(
            report.all_clean(),
            "both A and B joined cleanly: {report:?}"
        );
    }

    /// A late registration racing teardown (registry already `closing`)
    /// must not be dropped-and-detached: it is parked as an orphan and a
    /// subsequent teardown joins it.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn register_thread_after_shutdown_parks_as_orphan() {
        let reg = ThreadRegistry::<&str>::new();
        assert!(reg.shutdown().await.all_clean());

        let (tx, rx) = mpsc::channel::<()>();
        reg.register_thread("late", WorkerConfig::default(), spawn_gated(rx));
        assert_eq!(orphan_len(&reg), 1, "late registration parked as orphan");
        assert!(!reg.is_running("late"));

        drop(tx);
        let second = reg.shutdown().await;
        assert!(second.all_clean(), "late orphan reaped cleanly: {second:?}");
    }

    /// A registration for a key under a [`ClearingGuard`] is parked as an
    /// orphan (not installed into the slot), so a clear-then-wipe caller
    /// holding the latch never has a fresh handle-only worker slip into the
    /// slot mid-clear. Dropping the guard restores normal installation.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn register_thread_under_clearing_latch_parks_as_orphan() {
        let reg = ThreadRegistry::<&str>::new();
        let latch = reg.hold_clearing("shielded");
        assert!(reg.is_clearing("shielded"));

        let (tx, rx) = mpsc::channel::<()>();
        reg.register_thread("shielded", WorkerConfig::default(), spawn_gated(rx));
        assert_eq!(
            orphan_len(&reg),
            1,
            "registration under the latch is parked"
        );

        // Release the latch and the worker; a later registration installs
        // normally, and teardown reaps the parked one cleanly.
        drop(latch);
        drop(tx);
        assert!(reg.shutdown().await.all_clean());
    }

    /// `quiesce` on a handle-only slot classifies it correctly: the cancel
    /// step is a no-op (no token), the join is the real work, and a second
    /// call is idempotent (`NotRunning`).
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn register_thread_quiesce_joins_handle_only_slot() {
        let reg = ThreadRegistry::<&str>::new();
        let (tx, rx) = mpsc::channel::<()>();
        reg.register_thread("alpha", WorkerConfig::default(), spawn_gated(rx));

        drop(tx);
        assert_eq!(reg.quiesce("alpha").await, WorkerStatus::Ok);
        assert!(!reg.is_running("alpha"));
        assert_eq!(reg.quiesce("alpha").await, WorkerStatus::NotRunning);
    }

    /// A registered worker that panics surfaces as `Panicked` in the
    /// shutdown report (join captures the payload), flipping `all_clean`.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn register_thread_surfaces_panicked_worker() {
        let reg = ThreadRegistry::<&str>::new();
        let (tx, rx) = mpsc::channel::<()>();
        let handle = std::thread::spawn(move || {
            let _ = rx.recv();
            panic!("worker boom");
        });
        reg.register_thread("alpha", WorkerConfig::default(), handle);

        drop(tx);
        let report = reg.shutdown().await;
        match report.per_worker.get("alpha") {
            Some(WorkerStatus::Panicked(msg)) => assert!(msg.contains("worker boom")),
            other => panic!("expected Panicked, got {other:?}"),
        }
        assert!(!report.all_clean());
    }

    /// `is_closing` reflects the one-way teardown latch: `false` on a fresh
    /// registry, `true` once `shutdown` has begun. Consumers gate their own
    /// out-of-registry `start` on it so they never spawn a loop teardown has
    /// stopped waiting for.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn is_closing_tracks_shutdown_latch() {
        let reg = ThreadRegistry::<&str>::new();
        assert!(!reg.is_closing(), "fresh registry is not closing");
        assert!(reg.shutdown().await.all_clean());
        assert!(
            reg.is_closing(),
            "shutdown latched the registry closed (one-way)"
        );
    }

    /// A late registration that stays WEDGED past the reap grace is folded
    /// into the report as `detached` — `all_clean` cannot false-pass on a
    /// straggler that outlives teardown.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn register_thread_after_shutdown_wedged_orphan_flips_all_clean() {
        let reg = ThreadRegistry::<&str>::with_reap_backstop(Duration::from_millis(50));
        assert!(reg.shutdown().await.all_clean());

        let (tx, rx) = mpsc::channel::<()>();
        reg.register_thread("late", WorkerConfig::default(), spawn_gated(rx));
        assert_eq!(orphan_len(&reg), 1, "late registration parked as orphan");

        let report = reg.shutdown().await;
        assert!(
            !report.all_clean(),
            "a live straggler flips all_clean: {report:?}"
        );
        assert!(
            report.detached >= 1,
            "wedged orphan counted as detached: {report:?}"
        );

        drop(tx);
        assert_eq!(
            reg.reap_orphans(Duration::from_secs(2)).await,
            WorkerStatus::Ok
        );
        assert!(!reg.any_alive());
    }

    /// A prior generation that panicked is JOINED (and classified), not left
    /// dangling, when `register_thread` restarts the key: the restarting
    /// caller neither hangs nor inherits the panic, and the reap removes the
    /// prior from the orphan list.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn register_thread_restart_reaps_panicked_prior() {
        let reg = ThreadRegistry::<&str>::with_reap_backstop(Duration::from_millis(200));
        let (tx1, rx1) = mpsc::channel::<()>();
        let gen1 = std::thread::spawn(move || {
            let _ = rx1.recv();
            panic!("gen1 boom");
        });
        reg.register_thread("k", WorkerConfig::default(), gen1);

        // Let gen1 run to its panic so the restart reap joins a *finished*,
        // panicked prior — the path that previously discarded the join result.
        drop(tx1);

        let (tx2, rx2) = mpsc::channel::<()>();
        reg.register_thread("k", WorkerConfig::default(), spawn_gated(rx2));
        assert_eq!(
            orphan_len(&reg),
            0,
            "panicked prior joined + removed by the restart reap"
        );

        drop(tx2);
        assert!(reg.shutdown().await.all_clean());
    }
}
