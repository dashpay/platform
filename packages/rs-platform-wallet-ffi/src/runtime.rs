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

/// Get the shared tokio runtime.
///
/// All async FFI functions use this runtime. Prefer
/// [`block_on_worker`] over `runtime().block_on(...)` so the heavy
/// work runs on a worker thread with the larger stack configured
/// here, rather than the (small) calling thread.
pub(crate) fn runtime() -> &'static tokio::runtime::Runtime {
    static RT: once_cell::sync::Lazy<tokio::runtime::Runtime> = once_cell::sync::Lazy::new(|| {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_stack_size(WORKER_STACK_BYTES)
            .build()
            .expect("Failed to create tokio runtime for platform-wallet-ffi");

        #[cfg(feature = "tokio-metrics")]
        metrics::spawn_sampler(&rt);

        rt
    });
    &RT
}

/// Drive `future` to completion, moving the actual polling onto a
/// worker thread so the caller's stack size doesn't bound the
/// computation.
///
/// The calling thread still blocks (that's what FFI wants); it just
/// parks on a oneshot instead of driving the future itself.
pub(crate) fn block_on_worker<F>(future: F) -> F::Output
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    let rt = runtime();
    rt.block_on(async move { rt.spawn(future).await.expect("tokio worker panicked") })
}

/// Run `f` to completion on a freshly spawned scoped OS thread with the
/// same 8 MB stack the runtime workers get, blocking the caller until it
/// returns.
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
pub(crate) fn run_on_big_stack_thread<T: Send>(f: impl FnOnce() -> T + Send) -> T {
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .name("pw-ffi-bigstack".into())
            .stack_size(WORKER_STACK_BYTES)
            .spawn_scoped(scope, f)
            .expect("failed to spawn big-stack FFI thread")
            .join()
            .expect("big-stack FFI thread panicked")
    })
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
