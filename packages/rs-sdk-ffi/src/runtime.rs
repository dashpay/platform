//! A Tokio runtime wrapper that drives blocking FFI calls on an OS thread with
//! a large stack.
//!
//! ## Why this exists
//!
//! [`Runtime::block_on`] polls the root future on the *calling* thread. On iOS
//! the SDK's synchronous FFI entry points are invoked from libdispatch QoS
//! worker threads whose stacks are tiny (~512 KiB). Post-broadcast proof
//! verification (`Drive::verify_state_transition_was_executed_with_proof`) drives
//! deep recursion through grovedb proof verification and `platform_value::Value`
//! decoding; on a 512 KiB stack this overflows and the app crashes with
//! `EXC_BAD_ACCESS` / SIGBUS ("Thread stack size exceeded") *after* the
//! transition has already been signed and broadcast (e.g. Identity Transfer
//! Credits / Withdraw).
//!
//! `drive-abci` runs all consensus ABCI work on a Tokio runtime configured with
//! an 8 MiB `thread_stack_size` for exactly this reason. This wrapper gives the
//! client the same budget: [`block_on`](BigStackRuntime::block_on) runs the
//! future on a dedicated scoped OS thread with a large stack, while transparently
//! delegating every other [`Runtime`] method (`spawn`, `handle`, `enter`, …) to
//! the inner runtime via [`Deref`].

use std::future::Future;
use std::ops::Deref;
#[cfg(feature = "tokio-metrics")]
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::runtime::{Builder, Runtime};

/// Stack size for the OS thread that drives blocking FFI calls.
///
/// Set to twice `drive-abci`'s 8 MiB consensus runtime stack so that client-side
/// proof verification has at least the same recursion budget as the nodes that
/// produced the proof, with headroom for the larger frames of unoptimized debug
/// builds. Thread stacks are reserved lazily by the OS, so this costs negligible
/// physical memory until it is actually touched.
const FFI_BLOCKING_STACK_SIZE: usize = 16 * 1024 * 1024;

/// Worker-thread stack size for tasks spawned onto the runtime with
/// [`Runtime::spawn`]. Matches the consensus runtime so spawned work has the same
/// budget as [`BigStackRuntime::block_on`].
const WORKER_STACK_SIZE: usize = 8 * 1024 * 1024;

/// A [`Runtime`] whose [`block_on`](BigStackRuntime::block_on) drives the future
/// on an OS thread with a large stack. See the module docs for the rationale.
pub(crate) struct BigStackRuntime(Runtime);

impl BigStackRuntime {
    /// Wrap an already-built runtime (used for the mock/test runtimes).
    pub(crate) fn new(runtime: Runtime) -> Self {
        BigStackRuntime(runtime)
    }

    /// Build the shared multi-threaded runtime used by the FFI.
    ///
    /// Mirrors the previous inline builder (single worker thread for mobile, all
    /// drivers enabled) and additionally gives worker threads a large stack so
    /// that work spawned with [`Runtime::spawn`] has the same budget as
    /// [`block_on`](Self::block_on).
    pub(crate) fn build_shared() -> std::io::Result<Self> {
        let runtime = Builder::new_multi_thread()
            .thread_name("dash-sdk-worker")
            .worker_threads(1) // Reduce threads for mobile
            .thread_stack_size(WORKER_STACK_SIZE)
            .enable_all()
            .build()?;

        #[cfg(feature = "tokio-metrics")]
        static COUNTER: AtomicUsize = AtomicUsize::new(0);

        #[cfg(feature = "tokio-metrics")]
        metrics::spawn_sampler(
            &runtime,
            format!(
                "dash-sdk-ffi-shared-runtime-{}",
                COUNTER.fetch_add(1, Ordering::SeqCst)
            )
            .as_str(),
        );

        Ok(BigStackRuntime(runtime))
    }

    /// Build an isolated multi-threaded runtime (the `Runtime::new()` equivalent
    /// used by the standalone-runtime query helpers), with large worker stacks.
    pub(crate) fn new_isolated() -> std::io::Result<Self> {
        let runtime = Builder::new_multi_thread()
            .thread_stack_size(WORKER_STACK_SIZE)
            .enable_all()
            .build()?;

        #[cfg(feature = "tokio-metrics")]
        static COUNTER: AtomicUsize = AtomicUsize::new(0);

        #[cfg(feature = "tokio-metrics")]
        metrics::spawn_sampler(
            &runtime,
            format!(
                "dash-sdk-ffi-isolated-runtime-{}",
                COUNTER.fetch_add(1, Ordering::SeqCst)
            )
            .as_str(),
        );

        Ok(BigStackRuntime(runtime))
    }

    /// Run `future` to completion, driving it on a dedicated OS thread with a
    /// large stack (see [`FFI_BLOCKING_STACK_SIZE`]).
    ///
    /// This is the drop-in replacement for `Runtime::block_on` on the FFI hot
    /// path: it accepts the same (possibly non-`Send`) futures the FFI builds —
    /// many capture `&` references to `#[repr(C)]` parameter structs holding raw
    /// pointers, which are `!Sync` and hence `!Send` — yet still drives them on a
    /// large stack.
    pub(crate) fn block_on<F>(&self, future: F) -> F::Output
    where
        F: Future,
    {
        #[cfg(feature = "tokio-metrics")]
        let _block_on_guard = metrics::BlockOnGuard::new();

        // `std::thread::Builder::spawn_scoped` is the only API that both (a) lets
        // the worker borrow non-`'static` data — the FFI call's references — and
        // (b) lets us set the stack size. It requires the moved-in closure (and
        // its return value) to be `Send`, so we wrap the future and its output in
        // `AssertSend`.
        //
        // SAFETY: asserting `Send` here is sound because execution is strictly
        // sequential, never concurrent:
        //   * `spawn_scoped` starts the worker, then this thread immediately
        //     blocks in `join()` until the worker has fully finished. While the
        //     worker runs the future, this thread touches neither the future nor
        //     any data it captured, so there is no simultaneous access (the usual
        //     reason `Send` is required) and thus no data race.
        //   * `thread::scope` additionally guarantees the worker cannot outlive
        //     this call, so any borrowed FFI pointers/references — valid for the
        //     duration of the FFI call, which has not yet returned — stay valid.
        // The future is created on this thread, run and dropped on the worker, and
        // only its output crosses back, all without overlap.
        struct AssertSend<T>(T);
        // SAFETY: see the explanation above the use of `AssertSend`.
        unsafe impl<T> Send for AssertSend<T> {}
        impl<T> AssertSend<T> {
            // Consuming `self` (rather than pattern-matching `self.0`) forces the
            // worker closure to capture the whole `AssertSend` value: under the
            // 2021 disjoint-closure-capture rules a `let AssertSend(x) = wrapper`
            // pattern would capture the inner `T` field directly and bypass the
            // `unsafe impl Send`.
            fn into_inner(self) -> T {
                self.0
            }
        }

        let future = AssertSend(future);
        std::thread::scope(|scope| {
            let output = std::thread::Builder::new()
                .name("dash-sdk-ffi-block-on".to_string())
                .stack_size(FFI_BLOCKING_STACK_SIZE)
                .spawn_scoped(scope, move || {
                    AssertSend(self.0.block_on(future.into_inner()))
                })
                .expect("failed to spawn FFI block-on thread")
                .join()
                // Preserve `block_on`'s panic behaviour: re-raise on the caller's
                // thread (a no-op under `panic = "abort"`, which aborts first).
                .unwrap_or_else(|payload| std::panic::resume_unwind(payload));
            output.into_inner()
        })
    }
}

impl Deref for BigStackRuntime {
    type Target = Runtime;

    fn deref(&self) -> &Runtime {
        &self.0
    }
}

#[cfg(feature = "tokio-metrics")]
mod metrics {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{Duration, Instant};

    static BLOCK_ON_IN_FLIGHT: AtomicUsize = AtomicUsize::new(0);

    pub(super) struct BlockOnGuard {
        start: Instant,
        in_flight: usize,
    }

    impl BlockOnGuard {
        pub(super) fn new() -> Self {
            let in_flight = BLOCK_ON_IN_FLIGHT.fetch_add(1, Ordering::Relaxed) + 1;
            Self {
                start: Instant::now(),
                in_flight,
            }
        }
    }

    impl Drop for BlockOnGuard {
        fn drop(&mut self) {
            let elapsed_us = self.start.elapsed().as_micros() as u64;
            BLOCK_ON_IN_FLIGHT.fetch_sub(1, Ordering::Relaxed);
            tracing::info!(
                target: "rs_sdk_ffi::metrics",
                kind = "block_on",
                elapsed_us,
                in_flight = self.in_flight,
            );
        }
    }

    pub(super) fn spawn_sampler(rt: &tokio::runtime::Runtime, runtime_name: &str) {
        let runtime_monitor = tokio_metrics::RuntimeMonitor::new(rt.handle());
        let mut rt_intervals = runtime_monitor.intervals();

        let runtime_name = runtime_name.to_string();

        rt.spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;
                let Some(r) = rt_intervals.next() else { break };

                tracing::info!(
                    target: "rs_sdk_ffi::metrics",
                    runtime = %runtime_name,
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Recurses `depth` levels, holding a large, un-elidable frame at each level
    /// so total stack use is roughly `depth * 8 KiB` regardless of optimization.
    fn deep_recurse(depth: usize) -> usize {
        let mut buf = [0u8; 8192];
        let last = buf.len() - 1;
        buf[0] = (depth & 0xff) as u8;
        buf[last] = buf[0];
        // `black_box(&buf)` forces the 8 KiB frame to be materialized so it can't
        // be optimized away regardless of profile.
        std::hint::black_box(&buf);
        if depth == 0 {
            0
        } else {
            1 + deep_recurse(depth - 1)
        }
    }

    /// Regression guard for the iOS proof-verification stack overflow.
    ///
    /// The recursion below needs ~8 MiB of stack — far more than the ~2 MiB
    /// default thread stack the test harness (and the libdispatch QoS workers
    /// that crashed in the field) provide. It completes only because
    /// [`BigStackRuntime::block_on`] drives the future on its dedicated
    /// large-stack thread rather than on the calling thread. If that behaviour
    /// regresses, this test overflows and aborts instead of passing.
    #[test]
    fn block_on_drives_future_on_large_stack() {
        let runtime = BigStackRuntime::new_isolated().expect("build runtime");
        let result = runtime.block_on(async { deep_recurse(1000) });
        assert_eq!(result, 1000);
    }

    /// A value returned from a non-`Send`-friendly closure still comes back
    /// correctly across the worker-thread hand-off.
    #[test]
    fn block_on_returns_future_output() {
        let runtime = BigStackRuntime::new_isolated().expect("build runtime");
        let value = runtime.block_on(async {
            tokio::task::yield_now().await;
            21 * 2
        });
        assert_eq!(value, 42);
    }
}
