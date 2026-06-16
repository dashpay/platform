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
