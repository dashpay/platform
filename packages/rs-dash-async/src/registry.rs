//! Shared lifecycle engine for background workers (`ThreadRegistry`).
//!
//! Centralizes the dangerous 80% of a background OS-thread worker's
//! lifecycle — the generation-match exit epilogue, the reap-or-park of a
//! restarted worker's prior thread, and the orphan drain — into one
//! tested place. The domain-specific 20% (the "is a pass in flight?"
//! drain barrier) stays with the consumer, which drains its own passes
//! *before* calling [`shutdown`](ThreadRegistry::shutdown).
//!
//! Workers are dedicated OS threads
//! ([`start_thread`](ThreadRegistry::start_thread)), for loops that
//! `block_on` `!Send` futures internally — the `!Send` value never
//! crosses the spawn boundary; the body itself is `Send`.
//!
//! # Why join at all
//!
//! Host callback contexts are owned by the workers themselves (the FFI
//! layer's persister/event wrappers release them on last-`Arc`-drop), so
//! joining is not about callback memory safety. It is about the
//! **runtime**: a consumer that owns its tokio runtime (e.g.
//! dash-evo-tool) must not drop it while a worker thread is still inside
//! `Handle::block_on`. [`shutdown`](ThreadRegistry::shutdown) is that
//! barrier, and the orphan accounting below exists so a wedged thread is
//! *reported* (`Timeout` / `Detached`) instead of silently detached —
//! the caller can then decide whether dropping the runtime is safe.
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
//! - **A restart never detaches a still-draining prior generation.** The
//!   prior handle is parked and bounded-joined; a genuine wedge stays
//!   parked for teardown to account for.

use std::collections::BTreeMap;
use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::runtime::RuntimeFlavor;
use tokio_util::sync::CancellationToken;

// ---------------------------------------------------------------------
// Key
// ---------------------------------------------------------------------

/// Worker identity — a consumer supplies a fixed enum.
/// Blanket-implemented — consumers just derive the listed bounds on
/// their own key type.
pub trait RegistryKey: Copy + Ord + Eq + std::fmt::Debug + Send + Sync + 'static {}
impl<T: Copy + Ord + Eq + std::fmt::Debug + Send + Sync + 'static> RegistryKey for T {}

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
    /// aborted at the runtime level). Never produced by the registry
    /// itself (its workers are OS threads); provided so a consumer
    /// classifying its own tokio tasks into a [`ShutdownReport`] (e.g.
    /// the wallet's event-adapter join) has a variant to use.
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
#[must_use = "inspect all_clean() before dropping the runtime: a non-clean status flags a still-live worker or orphan"]
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

/// Default managed-join budget when a [`WorkerConfig`] does not override
/// it. Pinned so an accidental change surfaces in tests.
pub const DEFAULT_JOIN_BUDGET: Duration = Duration::from_secs(30);

/// Default orphan reap backstop (start-time reap and shutdown grace).
pub const DEFAULT_REAP_BACKSTOP: Duration = Duration::from_secs(1);

/// Per-worker registration options.
#[derive(Clone, Copy, Debug)]
pub struct WorkerConfig {
    /// Managed-join timeout for this worker.
    pub join_budget: Duration,
    /// OS-thread stack size. `None` uses the platform default. Raise it
    /// for a loop whose body recurses deeply — e.g. GroveDB proof
    /// verification, which overflows the default stack and faults with
    /// SIGBUS on the guard page.
    pub stack_size: Option<NonZeroUsize>,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            join_budget: DEFAULT_JOIN_BUDGET,
            stack_size: None,
        }
    }
}

// ---------------------------------------------------------------------
// Internal handle + slot state
// ---------------------------------------------------------------------

/// A live worker's OS-thread join handle. Kept owned by its slot so a
/// cancellable caller can never move it into a future frame and detach it
/// on drop.
struct WorkerHandle(std::thread::JoinHandle<()>);

impl WorkerHandle {
    fn is_finished(&self) -> bool {
        self.0.is_finished()
    }

    /// Classify a **finished** handle: an OS thread yields only `Ok` /
    /// `Panicked`.
    fn classify(self) -> WorkerStatus {
        match self.0.join() {
            Ok(()) => WorkerStatus::Ok,
            Err(payload) => WorkerStatus::Panicked(panic_message(payload)),
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
        self.join_budget = cfg.join_budget;
        (prior, token, my_gen)
    }
}

// ---------------------------------------------------------------------
// The registry
// ---------------------------------------------------------------------

/// Shared lifecycle engine for background workers. See the module docs.
///
/// Parked orphans carry their originating key so restart reaps and
/// teardown accounting stay key-scoped.
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
            // worker must keep its OWN join budget, not inherit the failed
            // start's. Generation is rolled back too — the bump is only ever
            // observed under this lock and a failed start spawns no thread to
            // reference it, so the rollback is net-zero and the
            // externally-visible generation stays monotonic.
            let prev_generation = slot.generation;
            let prev_join_budget = slot.join_budget;
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
                    // under-lock snapshot can never see the new slot
                    // without also seeing the prior accounted (R1: store handle
                    // -> park prior -> drop guard -> THEN bounded reap below).
                    // See `park_prior_locked` for the lock-order rationale; the
                    // bounded join stays out of the lock in `reap_parked_prior`.
                    slot.handle = Some(WorkerHandle(join));
                    self.park_prior_locked(key, prior)
                }
                Err(e) => {
                    // Spawn failed (e.g. EAGAIN at the OS thread ceiling). Roll
                    // the slot back to exactly its pre-start state: clear the
                    // running flag, re-install the prior handle (never
                    // detached), and restore the prior join budget + generation
                    // so nothing of the failed start lingers. Generation
                    // returns to its pre-bump value (the bump was never
                    // observed outside this lock and spawned no thread).
                    // Nothing was parked, so there is no prior to reap below.
                    tracing::error!(
                        ?key,
                        error = %e,
                        "failed to spawn registry worker thread; rolling back \
                         start (prior handle re-installed, not detached)"
                    );
                    slot.cancel = None;
                    slot.handle = prior;
                    slot.generation = prev_generation;
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
    /// The latch is one-way: once teardown begins it never reopens.
    /// `start_thread` honours it internally; the accessor exists so a
    /// consumer can observe teardown state before side effects of its own.
    pub fn is_closing(&self) -> bool {
        self.closing.load(Ordering::Acquire)
    }

    /// Cancel this worker, then join within its budget. The live handle is
    /// owned by the slot and is **never** moved into this future's frame,
    /// so a dropped/timed-out call cannot detach it; on the managed
    /// timeout — or if this future is dropped mid-poll — the handle is
    /// re-parked into the orphan list.
    ///
    /// The registry owns no domain drain semantics: a consumer whose
    /// worker has an "in-flight pass" concept drains it itself before
    /// calling this (or [`shutdown`](Self::shutdown)).
    pub async fn quiesce(&self, key: K) -> WorkerStatus {
        // Snapshot the budget + generation, and bail early if nothing is
        // registered for this key. The generation is the anchor for the
        // supersede guard below.
        //
        // The inhabited check accepts a finished-but-unreaped handle
        // (`handle.is_some()`, not a liveness probe): it must still be
        // classified into its terminal status here rather than
        // short-circuited to `NotRunning`, which would drop the result on
        // the floor.
        let (budget, my_gen) = {
            let slots = self.lock_slots();
            match slots.get(&key) {
                Some(s) if s.cancel.is_some() || s.handle.is_some() => {
                    (s.join_budget, s.generation)
                }
                _ => return WorkerStatus::NotRunning,
            }
        };

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

    /// Reap parked orphans with a short grace; survivors are re-parked and
    /// reported as [`WorkerStatus::Detached`] (idempotent retry).
    pub async fn reap_orphans(&self, grace: Duration) -> WorkerStatus {
        self.reap_orphans_impl(grace).await.0
    }

    /// Teardown: every worker's (cancel -> join) runs concurrently;
    /// orphan reap runs last. **Requires a multi-thread runtime.**
    ///
    /// Latches the registry closed first (under the slot lock, before the
    /// key snapshot), so any `start_thread` racing teardown is either
    /// already in the snapshot or refused outright — shutdown is a
    /// one-way door and never leaves a worker un-joined. Idempotent.
    ///
    /// # Panics
    ///
    /// Panics if called outside a multi-thread Tokio runtime: an OS-thread
    /// worker drives its loop via `Handle::block_on` and needs the shared
    /// timer/IO driver, so a `current_thread` runtime would deadlock the
    /// join.
    pub async fn shutdown(&self) -> ShutdownReport<K> {
        Self::assert_multi_thread("shutdown");

        // Snapshot the registered keys. Latch the registry closed under
        // the same lock and before the snapshot so a racing start is
        // serialized: it either landed before this lock (and is in the
        // snapshot) or sees `closing` and bails.
        let keys: Vec<K> = {
            let slots = self.lock_slots();
            self.closing.store(true, Ordering::Release);
            slots.keys().copied().collect()
        };

        // Cancel + join every worker concurrently; `join_all` polls them
        // on one task so the joins interleave.
        let mut per_worker = BTreeMap::new();
        let drained = keys
            .into_iter()
            .map(|key| async move { (key, self.quiesce(key).await) });
        for (key, status) in futures::future::join_all(drained).await {
            per_worker.insert(key, status);
        }

        // Account for parked orphans last. The terminal status is folded
        // into the report so a panicked/errored reaped orphan that finished
        // within the grace (`detached == 0`) still flips `all_clean()`.
        let (mut orphan_status, mut detached) = self.reap_orphans_impl(self.reap_backstop).await;

        // Late parkers: a quiesce racing this teardown can re-park a handle
        // into orphans AFTER the reap above snapshotted the list. Re-drain
        // until the orphan list is stable so such a straggler cannot slip
        // through and let `all_clean()` false-pass. Bounded: `closing` is
        // one-way so no new worker can start, and each pass either drains a
        // finite backlog or re-parks genuine survivors (`detached > 0`) —
        // which we fold in and stop, leaving them for an idempotent retry
        // rather than spinning on a wedged thread.
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
    /// released, is what lets `shutdown`'s under-lock snapshot never
    /// miss it: the take-prior and the park-prior are then atomic from
    /// `shutdown`'s view. Returns the parked thread's id so
    /// [`reap_parked_prior`](Self::reap_parked_prior) can find and
    /// bounded-join it.
    fn park_prior_locked(
        &self,
        key: K,
        prior: Option<WorkerHandle>,
    ) -> Option<std::thread::ThreadId> {
        match prior {
            Some(h) => {
                let tid = h.0.thread().id();
                self.lock_orphans().push((key, h));
                Some(tid)
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
                let pos = orphans
                    .iter()
                    .position(|(k, h)| *k == key && h.0.thread().id() == tid);
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
        self.lock_orphans().push((key, WorkerHandle(handle)));
    }

    /// Test-only liveness probe spanning live slots and parked orphans —
    /// the assertion the re-park/reap tests are built on.
    #[cfg(test)]
    fn any_alive(&self) -> bool {
        {
            let slots = self.lock_slots();
            for slot in slots.values() {
                if slot.cancel.is_some() || slot.handle.as_ref().is_some_and(|h| !h.is_finished()) {
                    return true;
                }
            }
        }
        self.lock_orphans().iter().any(|(_, h)| !h.is_finished())
    }
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

    // ----- Group 5: status classification -----------------------------

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

    /// `WorkerConfig::default()` values are pinned.
    #[test]
    fn worker_config_defaults_pinned() {
        let cfg = WorkerConfig::default();
        assert_eq!(cfg.join_budget, DEFAULT_JOIN_BUDGET);
        assert!(cfg.stack_size.is_none());
    }

    /// `hold_clearing(key)` refuses `start_thread` for that key, but ONLY
    /// for that key — other keys are unaffected. After the guard drops the
    /// latch releases and starts succeed again.
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
        // gen-1: a worker that ignores cancellation (wedged on a channel),
        // with a tiny join budget so a non-guarded quiesce would Timeout
        // quickly.
        let (gen1_release_tx, gen1_release_rx) = mpsc::channel::<()>();
        reg.start_thread(
            "k",
            WorkerConfig {
                join_budget: Duration::from_millis(150),
                ..WorkerConfig::default()
            },
            wedged_body(gen1_release_rx),
        );

        // Drive quiesce concurrently; it snapshots gen=1, cancels (ignored),
        // and enters the poll loop with cancel already taken.
        let reg_q = Arc::clone(&reg);
        let q = tokio::spawn(async move { reg_q.quiesce("k").await });

        // Let quiesce pass cancel.take() so a restart can proceed.
        tokio::time::sleep(Duration::from_millis(40)).await;

        // Restart: cancel is now None, so this proceeds — it takes gen-1's
        // live handle as its prior (parked) and installs gen-2. Release the
        // wedge first so the restart's bounded prior-reap can join it.
        gen1_release_tx.send(()).expect("release gen-1");
        start_clean(&reg, "k", WorkerConfig::default());

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
    /// the slot would carry the failed start's join budget and the bumped
    /// generation.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn spawn_failure_restores_prior_slot_config() {
        let reg = ThreadRegistry::<&str>::new();
        let (release_tx, release_rx) = mpsc::channel::<()>();

        // gen-1 with a DISTINCTIVE (non-default) join budget. Wedged so it
        // stays the live prior after cancel.
        let cfg1 = WorkerConfig {
            join_budget: Duration::from_secs(11),
            ..WorkerConfig::default()
        };
        reg.start_thread("k", cfg1, wedged_body(release_rx));
        reg.cancel("k");
        let gen_after_gen1 = reg.lock_slots().get("k").unwrap().generation;

        // Failed restart with a DIFFERENT config; the rollback must discard it.
        reg.force_spawn_failure.store(true, Ordering::Release);
        let cfg2 = WorkerConfig {
            join_budget: Duration::from_secs(99),
            ..WorkerConfig::default()
        };
        reg.start_thread("k", cfg2, |_cancel| {});
        reg.force_spawn_failure.store(false, Ordering::Release);

        {
            let slots = reg.lock_slots();
            let slot = slots.get("k").expect("slot present");
            assert_eq!(
                slot.join_budget,
                Duration::from_secs(11),
                "join_budget restored to prior"
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

        // One-way door: a start after shutdown is refused.
        start_clean(&reg, "late_thread", WorkerConfig::default());
        assert!(
            !reg.is_running("late_thread"),
            "start_thread after shutdown is refused"
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
}
