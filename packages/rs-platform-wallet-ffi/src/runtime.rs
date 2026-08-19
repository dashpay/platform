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

/// Convert a caught worker/thread panic into a value of the driven future's
/// own output type, so a panic surfaces as a typed error at the FFI boundary
/// instead of unwinding across `extern "C"` and aborting the host (workspace
/// policy — `Cargo.toml`: "a JNI library must never abort the app process").
///
/// Implemented for every output type actually driven through
/// [`block_on_worker`]: any `Result<T, E>` whose error type recovers (the
/// overwhelmingly common shape — the panic becomes a typed `Err`), plus the
/// handful of non-`Result`, best-effort outputs whose panic degrades to a
/// logged, empty value. The [`block_on_worker`] bound is fail-closed: a new
/// call site whose output does not implement this will not compile until it
/// opts into an explicit recovery here.
pub(crate) trait RecoverWorkerPanic {
    fn recover_from_worker_panic(reason: String) -> Self;
}

impl<T, E: RecoverWorkerPanic> RecoverWorkerPanic for Result<T, E> {
    fn recover_from_worker_panic(reason: String) -> Self {
        Err(E::recover_from_worker_panic(reason))
    }
}

impl RecoverWorkerPanic for platform_wallet::PlatformWalletError {
    fn recover_from_worker_panic(reason: String) -> Self {
        platform_wallet::PlatformWalletError::InternalPanic(reason)
    }
}

/// Fire-and-forget `..._sync_now` entry points returning `()`: the panic is
/// already logged by the runtime helper; the sync is a no-op for this pass and
/// the host's next periodic sync retries.
impl RecoverWorkerPanic for () {
    fn recover_from_worker_panic(_reason: String) -> Self {}
}

/// Contact-crypto counters: a recovered panic reports zero (logged), which the
/// host reconciles on its next sync — never an abort.
impl RecoverWorkerPanic for usize {
    fn recover_from_worker_panic(_reason: String) -> Self {
        0
    }
}

/// Best-effort sync summaries: a recovered panic yields an empty summary
/// (0 processed / 0 errors), logged; the host's next sync retries. Never an
/// abort.
impl RecoverWorkerPanic for platform_wallet::DashPaySyncSummary {
    fn recover_from_worker_panic(_reason: String) -> Self {
        Self::default()
    }
}

impl RecoverWorkerPanic for platform_wallet::manager::dpns_sync::DpnsSyncPassSummary {
    fn recover_from_worker_panic(_reason: String) -> Self {
        Self::default()
    }
}

impl RecoverWorkerPanic for platform_wallet::manager::shielded_sync::ShieldedSyncPassSummary {
    fn recover_from_worker_panic(_reason: String) -> Self {
        Self::default()
    }
}

/// Best-effort message from a caught panic payload (`Box<dyn Any>`), which is a
/// `&'static str` or `String` for essentially every panic (`panic!`, `unwrap`,
/// `expect`, assertions).
fn panic_payload_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(s) = payload.downcast_ref::<&'static str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "non-string panic payload".to_string()
    }
}

/// Drive `future` to completion, moving the actual polling onto a
/// worker thread so the caller's stack size doesn't bound the
/// computation.
///
/// The calling thread still blocks (that's what FFI wants); it just
/// parks on a oneshot instead of driving the future itself.
///
/// ## Panic safety
///
/// A panic inside `future` must NOT cross the `extern "C"` FFI boundary: on the
/// `unwind` (Android/host) build that unwind aborts the process with `SIGABRT`
/// (Rust aborts when a panic escapes an `extern "C"` fn), and the JNI shim's
/// own `catch_unwind` (`rs-unified-sdk-jni`) sits ABOVE this callee, so it can
/// never intercept the abort — exactly what the workspace policy in `Cargo.toml`
/// forbids. Two panic surfaces are closed here:
///
///  1. `future` panics — tokio unwind-catches the spawned task into a
///     `JoinError`. The pre-fix `.expect("tokio worker panicked")` re-raised it
///     (that re-panic was the abort). We instead convert it into the output
///     type's typed error via [`RecoverWorkerPanic`].
///  2. Any stray panic while `block_on` drives the `JoinHandle` — caught by the
///     surrounding `catch_unwind` and recovered the same way.
///
/// On the iOS `panic = "abort"` profiles (`dev-ios` / `release-ios`) this is
/// INERT by design: the process aborts at the panic site before any
/// `catch_unwind`/`JoinError` is observed. That matches the in-tree note that
/// `catch_unwind` cannot protect an abort-configured build; this hardens the
/// `unwind` builds without pretending to protect iOS.
///
/// The success path is unchanged: a future that completes normally returns its
/// value directly.
pub(crate) fn block_on_worker<F>(future: F) -> F::Output
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static + RecoverWorkerPanic,
{
    let rt = runtime();
    let joined = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        rt.block_on(async move { rt.spawn(future).await })
    }));
    match joined {
        // Success path (unchanged): the task completed and returned its value.
        Ok(Ok(output)) => output,
        // The spawned task panicked (or was cancelled): tokio captured it as a
        // `JoinError`. Recover into the output's typed error instead of
        // re-raising it across the `extern "C"` caller.
        Ok(Err(join_err)) => {
            let reason = format!("tokio worker task did not complete: {join_err}");
            tracing::error!(target: "platform_wallet_ffi", "{reason}");
            F::Output::recover_from_worker_panic(reason)
        }
        // Belt and suspenders: a panic in the `block_on` driver itself.
        Err(panic_payload) => {
            let reason = format!(
                "panic while driving tokio worker: {}",
                panic_payload_message(panic_payload.as_ref())
            );
            tracing::error!(target: "platform_wallet_ffi", "{reason}");
            F::Output::recover_from_worker_panic(reason)
        }
    }
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
/// ## Panic safety
///
/// A panic inside `f` is CAUGHT (`std::thread::join` captures it) and mapped to
/// an `io::Error`, rather than re-raised: the pre-fix
/// `.expect("big-stack FFI thread panicked")` would have unwound across the
/// `extern "C"` caller and aborted the host on the `unwind` build (`Cargo.toml`
/// policy). The lone caller already threads the returned `io::Result` into its
/// `PlatformWalletFFIResult`. On the iOS `abort` profiles this is inert (the
/// process aborts at the panic site), as documented on [`block_on_worker`].
pub(crate) fn run_on_big_stack_thread<T: Send>(f: impl FnOnce() -> T + Send) -> std::io::Result<T> {
    std::thread::scope(|scope| {
        let handle = std::thread::Builder::new()
            .name("pw-ffi-bigstack".into())
            .stack_size(WORKER_STACK_BYTES)
            .spawn_scoped(scope, f)?;
        match handle.join() {
            Ok(value) => Ok(value),
            Err(panic_payload) => {
                let reason = panic_payload_message(panic_payload.as_ref());
                tracing::error!(
                    target: "platform_wallet_ffi",
                    "big-stack FFI thread panicked: {reason}"
                );
                Err(std::io::Error::other(format!(
                    "big-stack FFI thread panicked: {reason}"
                )))
            }
        }
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

    // --- Panic safety (the abort-hazard fix) -------------------------------
    //
    // Gated to the `unwind` config: on an `abort`-configured build the panic
    // aborts the process at the panic site (documented, accepted iOS behavior),
    // so there is no recoverable outcome to assert. Under the normal test
    // profile (`unwind`) these prove a panicking future/closure returns the
    // typed error instead of aborting the runner.

    #[cfg(panic = "unwind")]
    #[test]
    fn block_on_worker_recovers_panicking_future_as_typed_error() {
        let out: Result<u32, platform_wallet::PlatformWalletError> =
            block_on_worker(async { panic!("boom in worker") });
        match out {
            Err(platform_wallet::PlatformWalletError::InternalPanic(msg)) => {
                assert!(
                    msg.contains("did not complete") || msg.contains("boom in worker"),
                    "unexpected recovered message: {msg}"
                );
            }
            other => panic!("expected recovered InternalPanic, got {other:?}"),
        }
    }

    #[cfg(panic = "unwind")]
    #[test]
    fn block_on_worker_success_path_is_unchanged() {
        let out: Result<u32, platform_wallet::PlatformWalletError> =
            block_on_worker(async { Ok(7) });
        assert!(matches!(out, Ok(7)));
    }

    #[cfg(panic = "unwind")]
    #[test]
    fn run_on_big_stack_thread_maps_panic_to_io_error() {
        let result: std::io::Result<()> =
            run_on_big_stack_thread(|| panic!("boom on big stack"));
        let err = result.expect_err("a panicking closure must map to Err, not abort");
        assert_eq!(err.kind(), std::io::ErrorKind::Other);
        assert!(err.to_string().contains("big-stack FFI thread panicked"));
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
