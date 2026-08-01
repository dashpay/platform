//! Shared tokio runtime for blocking on async wallet operations.
//!
//! ## Stack size
//!
//! iOS dispatch/concurrency worker threads default to ~512 KB of stack.
//! Proof verification in `rs-drive` recurses through GroveDB deeply
//! enough to blow past that — we've seen `EXC_BAD_ACCESS` at
//! `verify_state_transition_was_executed_with_proof`'s function
//! prologue, which is the classic fingerprint of a stack-guard hit.
//!
//! Two mitigations together:
//!
//! 1. Configure the worker-thread stack to 8 MB (matches what rs-sdk
//!    uses internally for similar reasons).
//! 2. Dispatch the heavy async work onto a worker via
//!    [`block_on_worker`] instead of polling directly on the
//!    (small-stacked) calling thread. `block_on` itself still runs
//!    on the caller, but it parks almost immediately — all the
//!    compute happens on the tokio worker.

/// Worker thread stack size for the shared runtime. 8 MB gives proof
/// verification + GroveDB comfortable headroom without meaningfully
/// affecting memory footprint (we spin up a small number of workers).
const WORKER_STACK_BYTES: usize = 8 * 1024 * 1024;

/// Which piece of the shared async machinery failed, independently of the
/// request being served.
///
/// Deliberately a unit-like enum: it names the STAGE and nothing else. The
/// underlying `io::Error`, `JoinError` and panic payload are dropped at the
/// point of mapping, so nothing unbounded or caller-derived travels through
/// this VALUE into an FFI result message or a log.
///
/// That is a property of the value, not of the process. A panicking worker
/// still runs the default panic hook at the point of the panic — before this
/// mapping happens — and that hook may emit the payload on its own channel.
/// Futures submitted through this module must therefore never panic with
/// sensitive or caller-derived data; the classification here is not a redaction
/// mechanism for panics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorkerFailure {
    /// The shared runtime could not be built.
    RuntimeInit,
    /// The worker task did not run to completion (it panicked or was cancelled).
    WorkerJoin,
}

impl std::fmt::Display for WorkerFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            WorkerFailure::RuntimeInit => "async runtime could not be created",
            WorkerFailure::WorkerJoin => "async worker did not complete",
        })
    }
}

impl std::error::Error for WorkerFailure {}

// One-shot failure injection, scoped to the calling thread so parallel tests
// cannot observe or race each other's forcing. Each hook is consumed by the
// first check that sees it and leaves the flag clear, so a forced failure
// affects exactly one call and no state survives the test.
#[cfg(test)]
thread_local! {
    static FORCED_RUNTIME_INIT_FAILURE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static FORCED_WORKER_JOIN_FAILURE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Make the next [`try_runtime`] call on THIS thread report [`WorkerFailure::RuntimeInit`].
#[cfg(test)]
pub(crate) fn force_runtime_init_failure_once() {
    FORCED_RUNTIME_INIT_FAILURE.with(|flag| flag.set(true));
}

/// Make the next [`try_block_on_worker`] call on THIS thread report
/// [`WorkerFailure::WorkerJoin`].
#[cfg(test)]
pub(crate) fn force_worker_join_failure_once() {
    FORCED_WORKER_JOIN_FAILURE.with(|flag| flag.set(true));
}

#[cfg(test)]
fn take_forced_runtime_init_failure() -> bool {
    FORCED_RUNTIME_INIT_FAILURE.with(|flag| flag.replace(false))
}

#[cfg(not(test))]
fn take_forced_runtime_init_failure() -> bool {
    false
}

#[cfg(test)]
fn take_forced_worker_join_failure() -> bool {
    FORCED_WORKER_JOIN_FAILURE.with(|flag| flag.replace(false))
}

#[cfg(not(test))]
fn take_forced_worker_join_failure() -> bool {
    false
}

/// Get the shared tokio runtime, reporting construction failure as a value.
///
/// Preferred by callers that cross a non-unwinding `extern "C"` boundary: a
/// panic there would unwind into a frame that cannot unwind and be turned into
/// a forced abort, so the failure has to be a value they can map.
pub(crate) fn try_runtime() -> Result<&'static tokio::runtime::Runtime, WorkerFailure> {
    // Checked before the shared runtime is touched, so a forced failure never
    // builds, caches, replaces or poisons it — the next call still gets the
    // real runtime. Kept outside the cell mechanism so it exercises the
    // caller's mapping rather than the cell's retry behavior.
    if take_forced_runtime_init_failure() {
        return Err(WorkerFailure::RuntimeInit);
    }

    static RT: once_cell::sync::OnceCell<tokio::runtime::Runtime> =
        once_cell::sync::OnceCell::new();

    get_or_try_init_runtime(&RT, || {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_stack_size(WORKER_STACK_BYTES)
            .build()
            .map_err(|_| WorkerFailure::RuntimeInit)?;

        #[cfg(feature = "tokio-metrics")]
        metrics::spawn_sampler(&rt);

        Ok(rt)
    })
}

/// Return the cell's runtime, initializing it once if it is empty.
///
/// A failing initializer is NOT recorded: construction can fail for conditions
/// that pass, such as the OS momentarily refusing to spawn threads, and
/// remembering that first failure would make one transient refusal permanent
/// for the life of the process. The cell therefore stays empty until an
/// initializer succeeds, after which the runtime is shared by every caller.
/// The returned reference borrows from `cell`, so this works for the shared
/// `static` cell and for a local one a test owns — the retry behavior is the
/// same either way and nothing here assumes a `'static` lifetime.
fn get_or_try_init_runtime(
    cell: &once_cell::sync::OnceCell<tokio::runtime::Runtime>,
    init: impl FnOnce() -> Result<tokio::runtime::Runtime, WorkerFailure>,
) -> Result<&tokio::runtime::Runtime, WorkerFailure> {
    cell.get_or_try_init(init)
}

/// Get the shared tokio runtime.
///
/// All async FFI functions use this runtime. Prefer
/// [`block_on_worker`] over `runtime().block_on(...)` so the heavy
/// work runs on a worker thread with the larger stack configured
/// here, rather than the (small) calling thread.
pub(crate) fn runtime() -> &'static tokio::runtime::Runtime {
    try_runtime().expect("Failed to create tokio runtime for platform-wallet-ffi")
}

/// Drive `future` to completion on a worker thread, reporting runtime and
/// worker failure as values rather than panicking.
///
/// The calling thread still blocks (that's what FFI wants); it just parks on a
/// oneshot instead of driving the future itself.
pub(crate) fn try_block_on_worker<F>(future: F) -> Result<F::Output, WorkerFailure>
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    let rt = try_runtime()?;

    // Consumed on the CALLING thread, before the spawn: the future itself runs
    // on a worker, where a thread-local set by the caller is not visible.
    if take_forced_worker_join_failure() {
        return Err(WorkerFailure::WorkerJoin);
    }

    rt.block_on(async move {
        // The `JoinError` (and any panic payload it carries) is dropped here —
        // only the stage travels onward.
        rt.spawn(future)
            .await
            .map_err(|_| WorkerFailure::WorkerJoin)
    })
}

/// Drive `future` to completion, moving the actual polling onto a
/// worker thread so the caller's stack size doesn't bound the
/// computation.
///
/// Panics if the runtime cannot be built or the worker fails to complete. Call
/// sites that cannot afford a panic use [`try_block_on_worker`] instead.
pub(crate) fn block_on_worker<F>(future: F) -> F::Output
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    try_block_on_worker(future).expect("platform-wallet-ffi async worker failed")
}

/// Run `f` to completion on a freshly spawned scoped OS thread with the
/// same 8 MB stack the runtime workers get, blocking the caller until it
/// returns. Errors (instead of panicking) if the OS refuses to spawn
/// the thread, so `extern "C"` callers can surface the failure through
/// their `PlatformWalletFFIResult` rather than aborting the host.
///
/// Escape hatch for call sites that need big-stack polling but whose
/// future cannot satisfy [`block_on_worker`]'s `Send + 'static` bounds
/// (e.g. rustc's implied-lifetime-bound limitation, rust-lang/rust
/// issue #100013). The closure typically wraps
/// `runtime().block_on(...)` — the future is then created *and* polled
/// entirely on the big-stack thread, so no `Send`/`'static` proof is
/// needed for the future itself. Prefer [`block_on_worker`] where it
/// compiles: it reuses pooled runtime workers instead of paying a
/// thread spawn per call.
///
/// A panic inside `f` is propagated as a panic here. This helper and
/// [`block_on_worker`] share that stance: a panic in the passed work, or
/// a worker that fails to complete, is a programmer or runtime fault
/// rather than a recoverable condition, and the infallible helper
/// panics on it. Call sites that must not panic — anything crossing a
/// non-unwinding `extern "C"` frame — use [`try_block_on_worker`] and
/// map [`WorkerFailure`] to a result instead.
pub(crate) fn run_on_big_stack_thread<T: Send>(f: impl FnOnce() -> T + Send) -> std::io::Result<T> {
    std::thread::scope(|scope| {
        let handle = std::thread::Builder::new()
            .name("pw-ffi-bigstack".into())
            .stack_size(WORKER_STACK_BYTES)
            .spawn_scoped(scope, f)?;
        Ok(handle.join().expect("big-stack FFI thread panicked"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_on_big_stack_thread_round_trips_return_value() {
        let out = run_on_big_stack_thread(|| 41 + 1).expect("spawn should succeed");
        assert_eq!(out, 42);
    }

    /// The whole point of the helper: recursion far past the ~512 KB
    /// host-thread stacks (and the 2 MB default test-thread stack)
    /// must complete on the 8 MB thread.
    #[test]
    fn run_on_big_stack_thread_survives_deep_recursion() {
        #[inline(never)]
        fn recurse(depth: u32) -> u64 {
            // ~1 KB frame the optimizer can't elide.
            let frame = std::hint::black_box([depth as u64; 128]);
            if depth == 0 {
                frame[0]
            } else {
                recurse(depth - 1) + std::hint::black_box(frame[127])
            }
        }

        // ~1000 frames * >1 KB each (debug frames run several KB with
        // the black_box copies) lands well past the ~512 KB iOS host
        // stacks this helper exists for, while staying comfortably
        // under WORKER_STACK_BYTES.
        let out = run_on_big_stack_thread(|| recurse(1_000)).expect("spawn should succeed");
        assert!(out > 0);
    }

    // ── Fallible runtime + worker contracts ─────────────────────────────────
    //
    // `runtime()` and `block_on_worker()` both panic on failure: one `expect`s
    // the runtime build, the other `expect`s the join handle. Either way the
    // host process goes down. On an unwinding profile the unwind reaches the
    // non-unwinding `extern "C"` frame and is stopped there by a forced abort;
    // on an aborting profile the panic aborts where it is raised.
    //
    // The fallible siblings give the export a value to map instead. What that
    // covers differs by profile: runtime-construction failure, and a join error
    // the runtime reports explicitly, become values on any profile. Recovering
    // from a worker that PANICKED requires unwinding — under `panic = "abort"`
    // the panic aborts the process at its origin and no join mapping runs. The
    // infallible pair stays for callers already built around it.

    /// Runtime construction failure is a distinct outcome from a worker that
    /// panicked, and forcing it must not leave the shared runtime poisoned or
    /// replaced for anything else in the process.
    #[test]
    fn try_runtime_surfaces_construction_failure_as_error() {
        force_runtime_init_failure_once();
        assert!(
            matches!(try_runtime(), Err(WorkerFailure::RuntimeInit)),
            "a forced construction failure must surface as RuntimeInit"
        );

        // The one-shot is spent, so the shared runtime is intact and usable.
        assert!(
            try_runtime().is_ok(),
            "forcing the failure must not poison or replace the shared runtime"
        );
    }

    /// A future that panics on the worker must reach the caller as a value, not
    /// as an unwind. This is the outcome an encrypted C export maps to an
    /// ordinary error code rather than letting it reach the C frame.
    ///
    /// Only meaningful where unwinding exists: under `panic = "abort"` the
    /// worker panic aborts the process and there is nothing to observe.
    #[cfg(panic = "unwind")]
    #[test]
    fn try_block_on_worker_surfaces_a_real_worker_panic_as_join_failure() {
        let outcome: Result<(), WorkerFailure> =
            try_block_on_worker(async { panic!("worker panic under test") });

        assert!(
            matches!(outcome, Err(WorkerFailure::WorkerJoin)),
            "a panicking worker future must be reported as WorkerJoin, not re-raised \
             as a panic in the caller"
        );
    }

    /// A failed initialization must not be remembered.
    ///
    /// Runtime construction can fail for reasons that pass — the OS refusing
    /// threads under momentary pressure, for instance. Caching that first
    /// failure would turn a transient condition into a permanently dead
    /// process, so the cell must stay empty until an initializer succeeds and
    /// only then hold the runtime.
    #[test]
    fn runtime_cell_does_not_cache_a_failed_initializer() {
        let cell: once_cell::sync::OnceCell<tokio::runtime::Runtime> =
            once_cell::sync::OnceCell::new();

        let first = get_or_try_init_runtime(&cell, || Err(WorkerFailure::RuntimeInit));
        assert!(
            matches!(first, Err(WorkerFailure::RuntimeInit)),
            "a failing initializer must surface as RuntimeInit"
        );
        assert!(
            cell.get().is_none(),
            "a failed initialization must leave the cell empty so it can be retried"
        );

        let second = get_or_try_init_runtime(&cell, || {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .map_err(|_| WorkerFailure::RuntimeInit)
        });
        assert!(
            second.is_ok(),
            "a later successful initializer must succeed"
        );
        assert!(
            cell.get().is_some(),
            "a successful initialization must populate the cell"
        );
    }

    /// The ordinary path still returns the future's output untouched.
    #[test]
    fn try_block_on_worker_round_trips_a_normal_output() {
        let out = try_block_on_worker(async { 41 + 1 }).expect("no failure was forced");
        assert_eq!(out, 42);
    }

    /// The failure classification carries no future output and no panic payload
    /// — only which stage failed — so nothing unbounded or caller-derived can
    /// reach an FFI result message through it.
    ///
    /// This covers the RETURNED value only. The default panic hook still runs
    /// at the point of the panic, before any of this mapping, and may print the
    /// payload; that is a separate channel this assertion does not constrain.
    ///
    /// Only meaningful where unwinding exists, for the same reason as the
    /// worker-panic test above.
    #[cfg(panic = "unwind")]
    #[test]
    fn worker_failure_message_is_bounded_and_stage_only() {
        let forced: Result<(), WorkerFailure> =
            try_block_on_worker(async { panic!("payload that must not be echoed") });
        let failure = forced.expect_err("the worker panicked");

        let rendered = failure.to_string();
        assert!(
            !rendered.contains("payload that must not be echoed"),
            "the panic payload must not travel in the failure message: {rendered}"
        );
        assert!(
            !rendered.is_empty() && rendered.len() <= 128,
            "the failure message must be present and bounded, got {} chars",
            rendered.len()
        );
    }
}

#[cfg(feature = "tokio-metrics")]
mod metrics {
    use std::time::Duration;

    pub(super) fn spawn_sampler(rt: &tokio::runtime::Runtime) {
        let runtime_monitor = tokio_metrics::RuntimeMonitor::new(rt.handle());
        let mut rt_intervals = runtime_monitor.intervals();

        rt.spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;
                let Some(r) = rt_intervals.next() else { break };

                tracing::info!(
                    target: "platform_wallet_ffi::metrics",
                    workers = r.workers_count,
                    live_tasks = r.live_tasks_count,
                    busy_ratio = r.busy_ratio(),
                    mean_poll_us = r.mean_poll_duration.as_micros() as u64,
                    mean_polls_per_park = r.mean_polls_per_park(),
                    steals = r.total_steal_count,
                    global_queue_depth = r.global_queue_depth,
                    local_queue_depth = r.total_local_queue_depth,
                    overflow = r.total_overflow_count,
                );
            }
        });
    }
}
