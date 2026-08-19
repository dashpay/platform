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
//!
//! ## Panic containment
//!
//! This module is also the crate's execution choke point, and therefore
//! where panic containment is cheapest: [`block_on_worker`],
//! [`FfiRuntime::block_on`] and [`run_on_big_stack_thread`] between them run
//! the async body of nearly every `extern "C"` entry point. A panic that
//! escapes any of them reaches a non-unwind ABI boundary and aborts the host
//! process — see [`crate::panic_guard`] for the mechanics and for why the
//! JNI shim's own `catch_unwind` cannot save it. All three convert a panic
//! into an error value instead.

use std::panic::Location;

use platform_wallet::PlatformWalletError;

use crate::panic_guard::{guard_ffi, panic_payload_message, report_panic, FromCaughtPanic};

/// Worker thread stack size for the shared runtime. 8 MB gives proof
/// verification + GroveDB comfortable headroom without meaningfully
/// affecting memory footprint (we spin up a small number of workers).
const WORKER_STACK_BYTES: usize = 8 * 1024 * 1024;

/// The shared runtime, wrapped so [`FfiRuntime::block_on`] shadows
/// `tokio::runtime::Runtime::block_on` with a panic-guarded version.
///
/// Every other `Runtime` method (`spawn`, `spawn_blocking`, `enter`,
/// `handle`, …) still resolves through [`Deref`](std::ops::Deref), so this is
/// a drop-in replacement for the `&'static Runtime` that [`runtime`] used to
/// hand out — the existing `runtime().block_on(...)` call sites are guarded
/// without being touched.
///
/// Making the guarded `block_on` the *default* one is deliberate: a future
/// entry point that reaches for `runtime().block_on(...)` gets containment
/// without having to know this module exists, which is the only way the
/// invariant survives across 478 entry points.
pub(crate) struct FfiRuntime(tokio::runtime::Runtime);

impl std::ops::Deref for FfiRuntime {
    type Target = tokio::runtime::Runtime;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FfiRuntime {
    /// Drive `future` to completion on the calling thread, converting a panic
    /// into `F::Output`'s error representation instead of letting it unwind
    /// into the caller's `extern "C"` abort shim.
    ///
    /// Shadows the inherent `Runtime::block_on` (an inherent method on the
    /// newtype wins over the `Deref` target's). Use [`Self::raw`] on the rare
    /// path that genuinely wants the unguarded one.
    #[track_caller]
    pub(crate) fn block_on<F>(&self, future: F) -> F::Output
    where
        F: std::future::Future,
        F::Output: FromCaughtPanic,
    {
        guard_ffi(|| self.0.block_on(future))
    }

    /// [`Self::block_on`] for futures whose output **cannot** represent
    /// failure — a balance `u64`, a `Vec` of peers, a sync summary, a lock
    /// guard.
    ///
    /// Those outputs deliberately do not implement
    /// [`FromCaughtPanic`]: fabricating a zero balance or an empty peer list
    /// out of a panic would turn a crash into silent, plausible-looking wrong
    /// data, which is worse than the abort this module exists to prevent. The
    /// panic is surfaced as the `Err` half instead, and the caller must decide
    /// what to do with it.
    #[track_caller]
    pub(crate) fn try_block_on<F>(&self, future: F) -> Result<F::Output, PlatformWalletError>
    where
        F: std::future::Future,
    {
        guard_ffi(|| Ok(self.0.block_on(future)))
    }

    /// The unwrapped tokio runtime, for call sites that compose their own
    /// guarding ([`block_on_worker`], anything already inside
    /// [`run_on_big_stack_thread`]) or that need to pass a `&Runtime` on.
    pub(crate) fn raw(&self) -> &tokio::runtime::Runtime {
        &self.0
    }
}

/// Get the shared tokio runtime.
///
/// All async FFI functions use this runtime. Prefer
/// [`block_on_worker`] over `runtime().block_on(...)` so the heavy
/// work runs on a worker thread with the larger stack configured
/// here, rather than the (small) calling thread.
pub(crate) fn runtime() -> &'static FfiRuntime {
    static RT: once_cell::sync::Lazy<FfiRuntime> = once_cell::sync::Lazy::new(|| {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_stack_size(WORKER_STACK_BYTES)
            .build()
            .expect("Failed to create tokio runtime for platform-wallet-ffi");

        #[cfg(feature = "tokio-metrics")]
        metrics::spawn_sampler(&rt);

        FfiRuntime(rt)
    });
    &RT
}

/// Drive `future` to completion, moving the actual polling onto a
/// worker thread so the caller's stack size doesn't bound the
/// computation.
///
/// The calling thread still blocks (that's what FFI wants); it just
/// parks on a oneshot instead of driving the future itself.
///
/// ## Panics are returned, not propagated
///
/// This is the crate's highest-traffic execution site, so it is also where
/// panic containment pays off most. Two distinct failure shapes are handled:
///
/// * **The spawned task panicked.** tokio polls the task inside its own
///   `catch_unwind`, so the panic never unwinds through our frames — it
///   arrives as [`tokio::task::JoinError`]. The previous
///   `.expect("tokio worker panicked")` turned that value back into a *live
///   panic on the calling thread*, which then unwound into the entry point's
///   `extern "C"` abort shim and SIGABRTed the process. It is now converted
///   to `F::Output`'s error representation instead.
/// * **The task was cancelled** (runtime shutdown, `abort()`), which the same
///   `.expect` also treated as a panic. It becomes an error result too: the
///   work definitively did not finish, but that is not a reason to kill the
///   host.
///
/// The outer [`guard_ffi`] additionally covers a panic raised by `block_on`
/// itself (e.g. driving the runtime from inside another runtime).
///
/// `F::Output` must be able to represent failure ([`FromCaughtPanic`]); see
/// that trait for why bare value types are deliberately excluded.
#[track_caller]
pub(crate) fn block_on_worker<F>(future: F) -> F::Output
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static + FromCaughtPanic,
{
    let rt = runtime();
    let location = Location::caller();
    guard_ffi(|| {
        // `raw()`: the surrounding `guard_ffi` already covers this frame, and
        // the join below reports the worker's panic with better context than
        // a second, nested guard could.
        rt.raw().block_on(async move {
            match rt.raw().spawn(future).await {
                Ok(value) => value,
                Err(join_error) => from_join_error(location, join_error),
            }
        })
    })
}

/// Convert a [`tokio::task::JoinError`] into the output's error
/// representation: the replacement for `.expect("tokio worker panicked")`.
///
/// Split out of [`block_on_worker`] so both `JoinError` shapes — panicked and
/// cancelled — can be exercised directly by tests; producing a *cancelled*
/// join through the full `block_on_worker` path would need the worker's
/// `JoinHandle`, which that function owns.
fn from_join_error<T: FromCaughtPanic>(
    location: &'static Location<'static>,
    join_error: tokio::task::JoinError,
) -> T {
    let detail = if join_error.is_panic() {
        format!(
            "tokio worker task panicked: {}",
            panic_payload_message(join_error.into_panic().as_ref())
        )
    } else {
        // Cancellation (runtime shutdown, an explicit `abort()`). The work
        // definitively did not finish — but that is an error to report, not a
        // reason to take the host process down with it.
        format!("tokio worker task did not complete: {join_error}")
    };
    T::from_caught_panic(report_panic(location, &detail))
}

/// [`block_on_worker`] for futures whose output **cannot** represent failure.
///
/// Same rationale as [`FfiRuntime::try_block_on`]: rather than inventing a
/// value for a `usize` count or a sync summary, the panic is returned as the
/// `Err` half so the entry point turns it into a real error result.
#[track_caller]
pub(crate) fn try_block_on_worker<F>(future: F) -> Result<F::Output, PlatformWalletError>
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    block_on_worker(async move { Ok(future.await) })
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
/// A panic inside `f` is reported through the SAME `io::Result` channel as a
/// failed spawn, rather than being re-raised on the calling thread. Joining a
/// panicked scoped thread hands back the payload as a value (the unwind was
/// already contained by the thread boundary); re-raising it — which
/// `.expect("big-stack FFI thread panicked")` used to do — turned a contained
/// panic back into a live one on a thread that unwinds straight into an
/// `extern "C"` abort shim. Every call site already maps `Err` to
/// [`crate::error::PlatformWalletFFIResultCode::ErrorWalletOperation`] with
/// the error's text, so the panic payload reaches the host unchanged and no
/// call site needed to be touched. The message carries
/// [`crate::panic_guard::FFI_PANIC_PREFIX`], so a panic is still
/// distinguishable from a genuine spawn failure in logs.
#[track_caller]
pub(crate) fn run_on_big_stack_thread<T: Send>(f: impl FnOnce() -> T + Send) -> std::io::Result<T> {
    let location = Location::caller();
    std::thread::scope(|scope| {
        let handle = std::thread::Builder::new()
            .name("pw-ffi-bigstack".into())
            .stack_size(WORKER_STACK_BYTES)
            .spawn_scoped(scope, f)?;
        handle.join().map_err(|payload| {
            std::io::Error::other(report_panic(
                location,
                &format!(
                    "big-stack FFI thread panicked: {}",
                    panic_payload_message(payload.as_ref())
                ),
            ))
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::{PlatformWalletFFIResult, PlatformWalletFFIResultCode};
    use crate::panic_guard::FFI_PANIC_PREFIX;

    /// Read an FFI result's message back as a Rust `String`.
    fn message_of(result: &PlatformWalletFFIResult) -> String {
        assert!(
            !result.message.is_null(),
            "error result must carry a message"
        );
        unsafe { std::ffi::CStr::from_ptr(result.message) }
            .to_str()
            .expect("message is UTF-8")
            .to_string()
    }

    /// A stand-in for the crate's ~478 real entry points, with the shape they
    /// all share: `#[no_mangle] pub unsafe extern "C" fn -> PlatformWalletFFIResult`
    /// whose body drives an async body through `block_on_worker`.
    ///
    /// It has to be a genuine `extern "C"` fn for the test to mean anything:
    /// rustc plants the non-unwind ABI's abort shim in the **callee**, so this
    /// function aborts on an escaping panic no matter who calls it — a plain
    /// Rust fn would not reproduce the bug at all.
    ///
    /// `#[cfg(test)]`, so it never reaches the cdylib's exported surface or
    /// the cbindgen header.
    #[no_mangle]
    unsafe extern "C" fn platform_wallet_ffi_test_panicking_entry_point() -> PlatformWalletFFIResult
    {
        let outcome: Result<(), PlatformWalletError> = block_on_worker(async move {
            // A bounds check on network-supplied data — the panic class the
            // audit calls out, and one that fires in every profile (unlike an
            // overflow check, which only trips where `overflow-checks` is on).
            let payload = [0u8; 4];
            let offset_from_the_wire = std::hint::black_box(7usize);
            let _ = payload[offset_from_the_wire];
            Ok(())
        });

        match outcome {
            Ok(()) => PlatformWalletFFIResult::ok(),
            Err(error) => PlatformWalletFFIResult::from(error),
        }
    }

    /// The headline regression test: a panic raised inside an entry point's
    /// async body comes back as a clean error result.
    ///
    /// **The test process surviving IS half the assertion.** Before this
    /// change the panic was re-raised on the calling thread by
    /// `.expect("tokio worker panicked")` and unwound into the `extern "C"`
    /// shim above, which aborts — the test binary would die with SIGABRT and
    /// no assertion below would ever run.
    #[test]
    fn panicking_entry_point_returns_an_error_result_instead_of_aborting() {
        let result = unsafe { platform_wallet_ffi_test_panicking_entry_point() };

        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorWalletOperation,
            "a caught panic must arrive as the generic wallet-operation code, \
             never as a code that carries retry/outcome semantics"
        );
        let message = message_of(&result);
        assert!(
            message.starts_with(FFI_PANIC_PREFIX),
            "message must be recognizable as an internal panic: {message}"
        );
        assert!(
            message.contains("tokio worker task panicked"),
            "message must say the worker task panicked: {message}"
        );
        assert!(
            message.contains("index out of bounds"),
            "message must carry the panic payload: {message}"
        );
    }

    /// The `Result`-returning half of the same path: the panic arrives as the
    /// typed `PlatformWalletError::InternalPanic`, which the FFI boundary maps
    /// to the same generic code.
    #[test]
    fn panicking_worker_maps_result_outputs_to_internal_panic() {
        let result: Result<u64, PlatformWalletError> =
            block_on_worker(async move { panic!("worker boom") });

        let error = result.expect_err("a panicking worker must not report success");
        assert!(
            matches!(error, PlatformWalletError::InternalPanic(_)),
            "expected InternalPanic, got {error:?}"
        );
        let ffi = PlatformWalletFFIResult::from(error);
        assert_eq!(ffi.code, PlatformWalletFFIResultCode::ErrorWalletOperation);
        assert!(message_of(&ffi).contains("worker boom"));
    }

    /// `try_block_on_worker` covers the outputs that cannot represent failure
    /// themselves (a bare `u64` here), so the panic has to ride the `Err` half.
    #[test]
    fn try_block_on_worker_surfaces_a_panic_as_err() {
        let result = try_block_on_worker(async move { panic!("counting boom") });

        let error = result.expect_err("a panicking worker must not report success");
        assert!(matches!(error, PlatformWalletError::InternalPanic(_)));
        assert!(error.to_string().contains("counting boom"));
        // The success path still yields the bare value untouched.
        assert_eq!(
            try_block_on_worker(async move { 7u64 }).expect("no panic"),
            7
        );
    }

    /// `FfiRuntime::block_on` shadows tokio's, so the many
    /// `runtime().block_on(...)` entry points are guarded without being
    /// rewritten.
    #[test]
    fn runtime_block_on_is_guarded_and_passes_values_through() {
        let result: PlatformWalletFFIResult = runtime().block_on(async { panic!("local boom") });
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorWalletOperation
        );
        assert!(message_of(&result).contains("local boom"));

        let ok: Result<u64, PlatformWalletError> = runtime().block_on(async { Ok(9) });
        assert_eq!(ok.expect("no panic"), 9);
    }

    /// Both `JoinError` shapes, against errors produced by tokio itself.
    ///
    /// The cancelled shape is why this mapping exists as its own function: the
    /// old `.expect("tokio worker panicked")` re-panicked on a *cancelled*
    /// worker too — mislabelling it, and aborting the host over work that
    /// merely did not finish.
    #[test]
    fn join_error_shapes_both_become_error_values() {
        let location = Location::caller();

        let panicked: tokio::task::JoinError = runtime().raw().block_on(async {
            tokio::spawn(async { panic!("joined boom") })
                .await
                .expect_err("the task panicked")
        });
        assert!(panicked.is_panic());
        let result: PlatformWalletFFIResult = from_join_error(location, panicked);
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorWalletOperation
        );
        let message = message_of(&result);
        assert!(message.starts_with(FFI_PANIC_PREFIX));
        assert!(message.contains("joined boom"), "{message}");

        let cancelled: tokio::task::JoinError = runtime().raw().block_on(async {
            let handle = tokio::spawn(std::future::pending::<()>());
            handle.abort();
            handle.await.expect_err("the task was aborted")
        });
        assert!(cancelled.is_cancelled());
        let result: Result<u64, PlatformWalletError> = from_join_error(location, cancelled);
        let error = result.expect_err("a cancelled worker is not a success");
        assert!(matches!(error, PlatformWalletError::InternalPanic(_)));
        assert!(
            error.to_string().contains("did not complete"),
            "a cancelled worker must not be reported as a panic: {error}"
        );
    }

    /// A panic on the big-stack path is reported through the `io::Result` the
    /// call sites already handle, rather than re-raised into the abort shim.
    #[test]
    fn run_on_big_stack_thread_reports_a_panic_as_an_io_error() {
        let result = run_on_big_stack_thread(|| panic!("big-stack boom"));

        let error = result.expect_err("a panicking pass must not report success");
        let rendered = error.to_string();
        assert!(rendered.starts_with(FFI_PANIC_PREFIX), "{rendered}");
        assert!(rendered.contains("big-stack boom"), "{rendered}");
    }

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
