//! Shared lifecycle engine for background workers (`ThreadRegistry`).
//!
//! Centralizes the dangerous, previously-triplicated 80% of a background
//! worker's lifecycle — the generation-match exit epilogue, the
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
//! # Why F1 and F2 cannot recur
//!
//! - **F1** (timeout-dropped quiesce detaches a live thread): every join
//!   path takes `&self`; the live join handle stays owned by the slot
//!   and is never moved into a cancellable future's frame. A
//!   dropped/timed-out [`quiesce`](ThreadRegistry::quiesce) therefore
//!   cannot drop-and-detach the handle — on timeout (or on an external
//!   drop) the handle is deterministically re-parked into the orphan
//!   list, and the slot reports [`WorkerStatus::Timeout`], never a clean
//!   `NotRunning`.
//! - **F2** (store wipe races a parked prior-generation thread):
//!   orphans live in the registry and [`any_alive`](ThreadRegistry::any_alive)
//!   is the single liveness gate spanning live slots **and** parked
//!   orphans. Every store-wiping path consults it, so a parked
//!   still-live thread blocks the wipe.

use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;
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

/// Terminal status of one worker. Variant set and payloads are
/// byte-identical to the wallet's `CoordinatorThreadStatus`, which is
/// constructed from this via `From` so the FFI surface stays stable.
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
pub struct ShutdownReport<K: RegistryKey> {
    /// Per-worker terminal status, keyed by worker id.
    pub per_worker: BTreeMap<K, WorkerStatus>,
    /// Number of parked orphans still alive at the reap deadline.
    pub detached: usize,
}

impl<K: RegistryKey> ShutdownReport<K> {
    /// `true` only when every per-worker status is clean and no orphan
    /// survived the reap.
    pub fn all_clean(&self) -> bool {
        self.detached == 0 && self.per_worker.values().all(WorkerStatus::is_clean)
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
/// compile as a `DrainHook`:
///
/// ```compile_fail
/// use std::rc::Rc;
/// use std::sync::Arc;
/// use dash_async::DrainHook;
/// let rc = Rc::new(42u32); // !Send
/// let _hook: DrainHook =
///     Arc::new(move || { let r = Rc::clone(&rc); Box::pin(async move { let _ = &r; }) });
/// ```
pub type DrainHook =
    Arc<dyn Fn() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

/// Default managed-join budget when a [`WorkerConfig`] does not override
/// it. Pinned so an accidental change surfaces in tests.
pub const DEFAULT_JOIN_BUDGET: Duration = Duration::from_secs(30);

/// Default orphan reap backstop (start-time reap and shutdown grace).
pub const DEFAULT_REAP_BACKSTOP: Duration = Duration::from_secs(1);

/// Per-worker registration options.
pub struct WorkerConfig {
    /// Teardown tier; lower drains first, equal weights concurrently.
    pub weight: ShutdownWeight,
    /// Optional drain barrier awaited before cancellation.
    pub drain: Option<DrainHook>,
    /// Managed-join timeout for this worker.
    pub join_budget: Duration,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            weight: ShutdownWeight::default(),
            drain: None,
            join_budget: DEFAULT_JOIN_BUDGET,
        }
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

impl SlotState {
    fn dormant() -> Self {
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

// ---------------------------------------------------------------------
// The registry
// ---------------------------------------------------------------------

/// Shared lifecycle engine for background workers. See the module docs.
pub struct ThreadRegistry<K: RegistryKey> {
    slots: Mutex<BTreeMap<K, SlotState>>,
    orphans: Mutex<Vec<WorkerHandle>>,
    reap_backstop: Duration,
}

impl<K: RegistryKey> ThreadRegistry<K> {
    /// New registry with the default reap backstop ([`DEFAULT_REAP_BACKSTOP`]).
    pub fn new() -> Arc<Self> {
        Self::with_reap_backstop(DEFAULT_REAP_BACKSTOP)
    }

    /// New registry with an explicit orphan reap backstop (the wallet
    /// uses 1s — the same grace separates "finishing" from "wedged").
    pub fn with_reap_backstop(backstop: Duration) -> Arc<Self> {
        Arc::new(Self {
            slots: Mutex::new(BTreeMap::new()),
            orphans: Mutex::new(Vec::new()),
            reap_backstop: backstop,
        })
    }

    /// Start an OS-thread worker for `!Send` loops. `body` runs on a
    /// fresh `std::thread` and may build and `block_on` `!Send` futures
    /// internally — the `!Send` value never crosses the spawn boundary
    /// (`body` itself is `Send`). Starting a key that already has a live
    /// worker is a no-op; a key whose prior thread has not been reaped is
    /// reaped-or-parked first (the restart-reap path).
    ///
    /// **Requires a multi-thread runtime**: the worker drives its loop
    /// via `Handle::block_on` and needs the shared timer/IO driver.
    pub fn start_thread<F>(self: &Arc<Self>, key: K, cfg: WorkerConfig, body: F)
    where
        F: FnOnce(CancellationToken) + Send + 'static,
    {
        Self::assert_multi_thread("start_thread");
        let prior = {
            let mut slots = self.lock_slots();
            let slot = slots.entry(key).or_insert_with(SlotState::dormant);
            if slot.cancel.is_some() {
                return;
            }
            // Take the prior handle to reap below; bump generation and
            // install the new token under this one lock so a prior
            // thread's epilogue observes the post-swap generation.
            let prior = slot.handle.take();
            let token = CancellationToken::new();
            slot.cancel = Some(token.clone());
            slot.generation += 1;
            let my_gen = slot.generation;
            slot.weight = cfg.weight;
            slot.drain = cfg.drain;
            slot.join_budget = cfg.join_budget;

            let reg = Arc::clone(self);
            let body_token = token;
            let join = std::thread::Builder::new()
                .name(format!("tr-worker-{key:?}"))
                .spawn(move || {
                    body(body_token);
                    reg.run_epilogue(key, my_gen);
                })
                .expect("failed to spawn registry worker thread");
            // Store the handle while still under the slot lock; the guard
            // is released at the end of this block, BEFORE the reap below
            // (R1: store handle -> drop guard -> THEN reap-or-park).
            slot.handle = Some(WorkerHandle::OsThread(join));
            prior
        };

        // The prior thread was cancellation-signalled by a preceding
        // cancel(); with the slot lock released its epilogue completes
        // promptly and the join lands in milliseconds. The backstop fires
        // only on a genuine wedge, in which case the still-live handle is
        // parked (not dropped) so teardown can account for it.
        self.reap_prior_or_park(prior, key);
    }

    /// Start a tokio-task worker for `Send` futures. Same restart-reap
    /// semantics as [`start_thread`](Self::start_thread); does not require
    /// a multi-thread runtime.
    pub fn start_task<F, Fut>(self: &Arc<Self>, key: K, cfg: WorkerConfig, body: F)
    where
        F: FnOnce(CancellationToken) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let prior = {
            let mut slots = self.lock_slots();
            let slot = slots.entry(key).or_insert_with(SlotState::dormant);
            if slot.cancel.is_some() {
                return;
            }
            let prior = slot.handle.take();
            let token = CancellationToken::new();
            slot.cancel = Some(token.clone());
            slot.generation += 1;
            let my_gen = slot.generation;
            slot.weight = cfg.weight;
            slot.drain = cfg.drain;
            slot.join_budget = cfg.join_budget;

            let reg = Arc::clone(self);
            let body_token = token;
            let join = tokio::spawn(async move {
                body(body_token).await;
                reg.run_epilogue(key, my_gen);
            });
            slot.handle = Some(WorkerHandle::Task(join));
            prior
        };
        self.reap_prior_or_park(prior, key);
    }

    /// Whether a worker is currently registered and running for `key`.
    pub fn is_running(&self, key: K) -> bool {
        self.lock_slots()
            .get(&key)
            .map(|s| s.cancel.is_some())
            .unwrap_or(false)
    }

    /// Signal-only cancellation of one worker (was `stop()`).
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

    /// Await this worker's drain hook, cancel it, then join within its
    /// budget. The live handle is owned by the slot and is **never** moved
    /// into this future's frame, so a dropped/timed-out call cannot detach
    /// it; on the managed timeout — or if this future is dropped
    /// mid-poll — the handle is re-parked into the orphan list. [F1 FIX]
    pub async fn quiesce(&self, key: K) -> WorkerStatus {
        // Snapshot the drain hook + budget, and bail early if nothing is
        // registered for this key.
        let (drain, budget) = {
            let slots = self.lock_slots();
            match slots.get(&key) {
                Some(s) if s.cancel.is_some() || s.handle.is_some() => {
                    (s.drain.clone(), s.join_budget)
                }
                _ => return WorkerStatus::NotRunning,
            }
        };

        // R2: gate-before-cancel — fully await the drain hook before the
        // cancel signal is observed.
        if let Some(drain) = drain {
            drain().await;
        }

        // Signal-only cancel.
        if let Some(slot) = self.lock_slots().get_mut(&key) {
            if let Some(token) = slot.cancel.take() {
                token.cancel();
            }
        }

        // Poll-join within budget. The re-park guard moves the slot's
        // still-live handle into orphans if this future is dropped before
        // the loop finishes — the handle is never owned by this frame.
        let _repark = Repark { reg: self, key };
        let deadline = Instant::now() + budget;
        loop {
            enum Step {
                Classify(WorkerHandle),
                Park(WorkerHandle),
                NotRunning,
                Wait,
            }
            let step = {
                let mut slots = self.lock_slots();
                match slots.get_mut(&key) {
                    None => Step::NotRunning,
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
                    self.lock_orphans().push(h);
                    return WorkerStatus::Timeout;
                }
                Step::NotRunning => return WorkerStatus::NotRunning,
                Step::Wait => tokio::time::sleep(Duration::from_millis(5)).await,
            }
        }
    }

    /// Is any registered worker **or** parked orphan still alive?
    /// Store-wiping paths must gate on this returning `false` before
    /// destroying shared state. [F2 FIX]
    pub fn any_alive(&self) -> bool {
        {
            let slots = self.lock_slots();
            for slot in slots.values() {
                if slot.cancel.is_some() {
                    return true;
                }
                if let Some(handle) = &slot.handle {
                    if !handle.is_finished() {
                        return true;
                    }
                }
            }
        }
        self.lock_orphans().iter().any(|h| !h.is_finished())
    }

    /// Reap parked orphans with a short grace; survivors are re-parked and
    /// reported as [`WorkerStatus::Detached`] (idempotent retry).
    pub async fn reap_orphans(&self, grace: Duration) -> WorkerStatus {
        self.reap_orphans_impl(grace).await.0
    }

    /// Weight-ordered teardown: ascending tier by tier, each worker's
    /// (drain-hook -> cancel -> join) run concurrently within a tier;
    /// orphan reap runs last. **Requires a multi-thread runtime.**
    pub async fn shutdown(&self) -> ShutdownReport<K> {
        Self::assert_multi_thread("shutdown");

        // Snapshot keys grouped by weight. A `BTreeMap` iterates tiers in
        // ascending weight order, giving the lower-first drain.
        let tiers: BTreeMap<ShutdownWeight, Vec<K>> = {
            let slots = self.lock_slots();
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
            let drained = keys.into_iter().map(|key| async move { (key, self.quiesce(key).await) });
            for (key, status) in futures::future::join_all(drained).await {
                per_worker.insert(key, status);
            }
        }

        // Account for parked orphans last.
        let (_status, detached) = self.reap_orphans_impl(self.reap_backstop).await;
        ShutdownReport {
            per_worker,
            detached,
        }
    }

    // -----------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------

    fn lock_slots(&self) -> std::sync::MutexGuard<'_, BTreeMap<K, SlotState>> {
        self.slots.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn lock_orphans(&self) -> std::sync::MutexGuard<'_, Vec<WorkerHandle>> {
        self.orphans.lock().unwrap_or_else(|e| e.into_inner())
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

    /// Gen-gated exit epilogue, run on the worker after its body returns:
    /// clear this slot's running flag only if a newer start has not since
    /// installed a replacement.
    fn run_epilogue(&self, key: K, my_gen: u64) {
        if let Some(slot) = self.lock_slots().get_mut(&key) {
            if slot.generation == my_gen {
                slot.cancel = None;
            }
        }
    }

    /// Reap a restarted key's prior worker — or park it if it is genuinely
    /// wedged past the reap backstop. Must be called with no registry lock
    /// held (it spins synchronously for an OS thread).
    fn reap_prior_or_park(&self, prior: Option<WorkerHandle>, key: K) {
        let Some(handle) = prior else {
            return;
        };
        match handle {
            WorkerHandle::OsThread(h) => {
                let deadline = Instant::now() + self.reap_backstop;
                loop {
                    if h.is_finished() {
                        let _ = h.join();
                        return;
                    }
                    if Instant::now() >= deadline {
                        tracing::warn!(
                            ?key,
                            backstop = ?self.reap_backstop,
                            "prior worker thread did not finish within the reap \
                             backstop after cancellation; parking it as an orphan \
                             for teardown to join rather than detaching it"
                        );
                        self.lock_orphans().push(WorkerHandle::OsThread(h));
                        return;
                    }
                    std::thread::sleep(Duration::from_millis(5));
                }
            }
            // A task can't be joined synchronously here; park a still-live
            // one for async reap. A finished one is dropped (detaching a
            // finished task is a no-op).
            task => {
                if !task.is_finished() {
                    self.lock_orphans().push(task);
                }
            }
        }
    }

    /// Drain the orphan list, polling until `grace`. Returns the terminal
    /// status and the number of survivors re-parked for an idempotent
    /// retry.
    async fn reap_orphans_impl(&self, grace: Duration) -> (WorkerStatus, usize) {
        let mut pending: Vec<WorkerHandle> = {
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
            for handle in pending.drain(..) {
                if handle.is_finished() {
                    let status = handle.classify();
                    if !status.is_clean() {
                        non_clean.get_or_insert(status);
                    }
                } else {
                    still_live.push(handle);
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

    /// Test-only seam: park a raw thread handle as an orphan. Used by
    /// cross-crate regression tests (e.g. the wallet's F2 gate) that must
    /// inject a wedged prior-generation thread without driving the full
    /// restart-reap path.
    #[doc(hidden)]
    pub fn park_orphan_for_test(&self, handle: std::thread::JoinHandle<()>) {
        self.lock_orphans().push(WorkerHandle::OsThread(handle));
    }
}

/// Re-park guard for [`ThreadRegistry::quiesce`]. If the poll-join future
/// is dropped before it finishes (e.g. an outer timeout fires), this moves
/// the slot's still-live handle into the orphan list instead of letting it
/// be dropped-and-detached. On normal completion the handle has already
/// been taken from the slot, so this is a no-op.
struct Repark<'a, K: RegistryKey> {
    reg: &'a ThreadRegistry<K>,
    key: K,
}

impl<K: RegistryKey> Drop for Repark<'_, K> {
    fn drop(&mut self) {
        // Take the handle under the slot lock, release it, then push to
        // orphans — never nest the two locks.
        let handle = self
            .reg
            .lock_slots()
            .get_mut(&self.key)
            .and_then(|slot| slot.handle.take());
        if let Some(handle) = handle {
            self.reg.lock_orphans().push(handle);
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

    /// TC-001 — a `quiesce` whose outer future is dropped (a tiny enclosing
    /// timeout) must re-park the live handle, never drop-and-detach it. The
    /// slot is cleared (`is_running == false`) but the handle lives in
    /// orphans and `any_alive()` stays true.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn tc001_quiesce_drop_reparks_handle_not_detach() {
        let reg = ThreadRegistry::<&str>::new();
        let (release_tx, release_rx) = mpsc::channel::<()>();
        reg.start_thread("alpha", WorkerConfig::default(), wedged_body(release_rx));
        assert!(reg.is_running("alpha"));

        // The wedged worker never observes cancel, so the internal 30s
        // budget can't fire here; the tiny outer timeout drops the quiesce
        // future mid-poll. A naive by-value-into-future impl would detach
        // the handle (orphans empty, any_alive false); the fix re-parks it.
        let result =
            tokio::time::timeout(Duration::from_millis(100), reg.quiesce("alpha")).await;
        assert!(result.is_err(), "outer timeout must fire on the wedged worker");

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

    /// TC-001b — internal-budget variant: a wedged worker with a tiny
    /// `join_budget` makes `quiesce` itself time out, re-park, and return
    /// `Timeout` (no outer drop involved).
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn tc001b_quiesce_internal_budget_timeout_reparks() {
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

    /// GAP-006 — the F1 scenario via the `shutdown()` path: a wedged worker
    /// with a tiny budget surfaces as `Timeout` in the report, its handle
    /// is re-parked (`detached == 1`, `any_alive`), and the result is
    /// non-clean — never a clean detach.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn gap006_shutdown_path_reparks_wedged_worker() {
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

    /// TC-003 — a slow prior-generation thread's epilogue must NOT clear a
    /// newer generation's token. Restarting reaps the prior generation
    /// fully (its epilogue runs); the new generation stays tracked.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn tc003_generation_match_epilogue_preserves_new_token() {
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

    /// TC-004 — a naturally-finished prior thread is joined cleanly on
    /// restart, with no parking.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn tc004_restart_reaps_finished_prior_without_parking() {
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

    /// TC-005 — a prior thread wedged past the reap backstop is parked in
    /// orphans (not dropped), then drained after release.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn tc005_restart_parks_wedged_prior() {
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

    /// TC-006 — orphan drain: a survivor at the grace deadline is reported
    /// `Detached` and re-parked; once released it reaps `Ok`.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn tc006_orphan_drain_detached_then_ok() {
        let reg = ThreadRegistry::<&str>::new();
        let (release_tx, release_rx) = mpsc::channel::<()>();
        let wedged = std::thread::spawn(move || {
            let _ = release_rx.recv();
        });
        reg.park_orphan_for_test(wedged);

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

    /// TC-007 — weight-ordered shutdown drains a lower tier before a higher
    /// one.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn tc007_weight_ordered_shutdown_drains_low_first() {
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

    /// TC-008 — equal-weight workers drain concurrently. A shared
    /// `Barrier(2)` in both drain hooks would deadlock under sequential
    /// draining (caught by the enclosing timeout); the event log proves
    /// both arrived before either passed.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn tc008_equal_weight_drains_concurrently() {
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
                drain: Some(mk_hook("a_arrived", "a_passed", Arc::clone(&log), Arc::clone(&barrier))),
                ..WorkerConfig::default()
            },
        );
        start_clean(
            &reg,
            "b",
            WorkerConfig {
                weight: ShutdownWeight(0),
                drain: Some(mk_hook("b_arrived", "b_passed", Arc::clone(&log), Arc::clone(&barrier))),
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

    /// TC-009 — `any_alive()` accounts for both live slots and orphans.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn tc009_any_alive_spans_slots_and_orphans() {
        let reg = ThreadRegistry::<&str>::new();
        start_clean(&reg, "alpha", WorkerConfig::default());
        assert!(reg.any_alive());

        let (release_tx, release_rx) = mpsc::channel::<()>();
        let wedged = std::thread::spawn(move || {
            let _ = release_rx.recv();
        });
        reg.park_orphan_for_test(wedged);
        assert!(reg.any_alive());

        assert_eq!(reg.quiesce("alpha").await, WorkerStatus::Ok);
        assert!(reg.any_alive(), "orphan still contributes after slot drains");
        assert!(!reg.is_running("alpha"));

        release_tx.send(()).unwrap();
        let _ = reg.reap_orphans(Duration::from_secs(2)).await;
        assert!(!reg.any_alive());
    }

    /// TC-010 — `shutdown()` panics with a documented message on a
    /// current-thread runtime (R4, variant B).
    #[test]
    fn tc010_shutdown_asserts_multi_thread_runtime() {
        let rt = Builder::new_current_thread().enable_all().build().unwrap();
        let reg = ThreadRegistry::<&str>::new();
        let result = catch_unwind(AssertUnwindSafe(|| {
            rt.block_on(async { reg.shutdown().await });
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

    /// TC-011 — the drain hook is fully awaited before the cancel signal is
    /// observed by the worker.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn tc011_drain_hook_completes_before_cancel() {
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

    /// TC-012 — a `quiesce` blocks in the drain hook until an `is_syncing`
    /// barrier the hook polls falls, and only then cancels + joins.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn tc012_drain_hook_observes_barrier_before_join() {
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

    /// TC-013 — only the `Task` kind can classify as `Stopped` (from a
    /// runtime-level cancel/abort JoinError); a cooperatively token-
    /// cancelled task exits normally as `Ok`. Verifies the kind-dispatch
    /// at the classification boundary.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn tc013_task_kind_classifies_stopped_and_ok() {
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

    /// TC-014 — an `OsThread` worker yields `Ok` (clean) or `Panicked`
    /// (`&str` and `String` payloads), never `Stopped`.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn tc014_os_thread_ok_and_panicked_never_stopped() {
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

    /// GAP-003 — `shutdown()` is idempotent: a second call finds every slot
    /// already joined and reports `NotRunning`, still clean.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn gap003_shutdown_is_idempotent() {
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

    /// GAP-004 — `cancel(key)` is selective: cancelling A does not touch B.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn gap004_cancel_is_selective() {
        let reg = ThreadRegistry::<&str>::new();
        start_clean(&reg, "a", WorkerConfig::default());
        start_clean(&reg, "b", WorkerConfig::default());

        reg.cancel("a");
        assert!(reg.is_running("b"), "cancel(a) must not cancel b");
        assert_eq!(reg.quiesce("a").await, WorkerStatus::Ok);
        assert!(reg.is_running("b"), "b still running after a drains");
        assert_eq!(reg.quiesce("b").await, WorkerStatus::Ok);
    }

    /// GAP-005 — `WorkerConfig::default()` values are pinned.
    #[test]
    fn gap005_worker_config_defaults_pinned() {
        let cfg = WorkerConfig::default();
        assert_eq!(cfg.weight, ShutdownWeight(0));
        assert!(cfg.drain.is_none());
        assert_eq!(cfg.join_budget, DEFAULT_JOIN_BUDGET);
    }
}
