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
//! [`RuntimeHandle::block_on`] and [`run_on_big_stack_thread`] between them run
//! the async body of nearly every `extern "C"` entry point. A panic that
//! escapes any of them reaches a non-unwind ABI boundary and aborts the host
//! process — see [`crate::panic_guard`] for the mechanics and for why the
//! JNI shim's own `catch_unwind` cannot save it. All three convert a panic
//! into an error value instead.
//!
//! ## Acquiring the runtime is itself fallible
//!
//! Building the runtime can fail *before* any of those guards exist. Tokio's
//! `Builder::build` reports driver-init failure as an `io::Error`, and its
//! multi-thread worker launch can panic outright when the OS refuses a thread.
//! While `runtime()` handed out a `Lazy` that `expect`ed the build, the very
//! first async FFI call on a constrained device evaluated that `Lazy` *outside*
//! any guard, and the resulting panic went straight into the caller's
//! `extern "C"` abort shim (dashpay/platform#4424 review).
//!
//! So acquisition is fallible now, and the one-time construction happens
//! *inside* [`crate::panic_guard::guard_ffi`]: [`runtime_checked`] returns
//! `Result<&'static FfiRuntime, FfiBoundaryError>`, and the ergonomic
//! [`runtime()`] handle is zero-sized — it holds no runtime, so merely calling
//! it can neither build nor panic. Every method on it acquires through
//! `runtime_checked` and folds a failure into the entry point's own error
//! channel, which is what makes "all entry paths route through the fallible
//! acquisition" true by construction rather than by review.

use std::panic::Location;

use crate::panic_guard::{
    guard_ffi, panic_payload_message, report_panic, FfiBoundaryError, FfiOutcome, GuardedOutput,
};

/// Worker thread stack size for the shared runtime. 8 MB gives proof
/// verification + GroveDB comfortable headroom without meaningfully
/// affecting memory footprint (we spin up a small number of workers).
const WORKER_STACK_BYTES: usize = 8 * 1024 * 1024;

/// The shared runtime, once it exists.
///
/// Reachable only through [`runtime_checked`] (or the [`RuntimeHandle`] methods
/// built on it), so there is no way to touch it without having handled the
/// possibility that it could not be built.
pub(crate) struct FfiRuntime(tokio::runtime::Runtime);

impl std::ops::Deref for FfiRuntime {
    type Target = tokio::runtime::Runtime;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FfiRuntime {
    /// The unwrapped tokio runtime, for call sites that compose their own
    /// guarding ([`block_on_worker`], anything already inside
    /// [`run_on_big_stack_thread`]) or that need to pass a `&Runtime` on.
    pub(crate) fn raw(&self) -> &tokio::runtime::Runtime {
        &self.0
    }
}

/// Construct the runtime. Fallible **by value**: `Builder::build` surfaces a
/// failed driver init as an `io::Error` rather than panicking, so this function
/// contributes no panic of its own. (Tokio's worker launch still can panic;
/// that is what the `guard_ffi` around the call below is for.)
fn build_runtime() -> std::io::Result<FfiRuntime> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_stack_size(WORKER_STACK_BYTES)
        .build()?;

    #[cfg(feature = "tokio-metrics")]
    metrics::spawn_sampler(&rt);

    Ok(FfiRuntime(rt))
}

/// One-time construction, performed inside the panic guard.
///
/// Both failure shapes become a *value* here, so nothing about first-call
/// initialization can unwind into an `extern "C"` frame:
///
/// * the driver's `io::Error`, which the old `.expect(...)` turned into a
///   panic, and
/// * a panic out of tokio's worker-thread launch, which no `expect` of ours
///   could have caught at all.
///
/// The `Lazy` closure itself is therefore infallible in the panic sense, which
/// also means it can never poison and re-panic on a later access.
static RT: once_cell::sync::Lazy<Result<FfiRuntime, FfiBoundaryError>> =
    once_cell::sync::Lazy::new(|| match guard_ffi(build_runtime) {
        FfiOutcome::Ok(Ok(runtime)) => Ok(runtime),
        FfiOutcome::Ok(Err(error)) => Err(FfiBoundaryError::runtime_unavailable(&format!(
            "failed to create the tokio runtime for platform-wallet-ffi: {error}"
        ))),
        // Already carries the panic marker at position 0 — a panic is a panic
        // wherever it happened, and re-labelling it would cost the host the
        // one thing the marker is for.
        FfiOutcome::Panicked(message) => Err(FfiBoundaryError::caught_panic(message)),
    });

/// Fallible acquisition of the shared runtime — the only way to reach it.
///
/// Cheap after the first call (a `Lazy` deref and a clone of nothing on the
/// success path). The `Err` is cloned rather than borrowed so callers can put
/// it in their own `Result` without borrowing from the static.
pub(crate) fn runtime_checked() -> Result<&'static FfiRuntime, FfiBoundaryError> {
    RT.as_ref().map_err(Clone::clone)
}

/// Test-only unguarded accessor, for test bodies that need to drive async
/// setup directly (and for which a failure to build the runtime is simply a
/// failed test, not something to report across an ABI).
#[cfg(test)]
pub(crate) fn test_runtime() -> &'static tokio::runtime::Runtime {
    runtime_checked()
        .expect("the FFI runtime must build in tests")
        .raw()
}

/// A zero-sized, deferred handle to the shared runtime.
///
/// Constructing it does not build, touch, or even initialize anything — which
/// is the point: `runtime()` is evaluated at the top of entry points, *outside*
/// any guard, so it must not be able to fail. Each method below acquires the
/// real runtime through [`runtime_checked`] and deals with a failure inside its
/// own error channel.
///
/// [`Self::block_on`] shadows what used to be a `Deref` to
/// `tokio::runtime::Runtime::block_on`, and keeping that name is deliberate: a
/// future entry point that reaches for `runtime().block_on(...)` gets
/// containment without having to know this module exists, which is the only way
/// the invariant survives across 478 entry points.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RuntimeHandle;

/// Get a handle to the shared tokio runtime.
///
/// All async FFI functions use this runtime. Prefer [`block_on_worker`] over
/// `runtime().block_on(...)` so the heavy work runs on a worker thread with the
/// larger stack configured here, rather than the (small) calling thread.
pub(crate) fn runtime() -> RuntimeHandle {
    RuntimeHandle
}

impl RuntimeHandle {
    /// Drive `future` to completion on the calling thread, converting a
    /// boundary failure into `F::Output`'s lossless error representation
    /// instead of letting it unwind into the caller's `extern "C"` abort shim.
    ///
    /// The output is reshaped by [`GuardedOutput`]: a future that already
    /// returns `Result<T, E>` comes back as `Result<T, GuardedError<E>>`, so
    /// the boundary failure travels in its own variant instead of being folded
    /// into (and re-coded by) the domain error. Outputs that cannot carry a
    /// failure at all do not implement the trait and must use
    /// [`Self::try_block_on`].
    #[track_caller]
    pub(crate) fn block_on<F>(self, future: F) -> <F::Output as GuardedOutput>::Guarded
    where
        F: std::future::Future,
        F::Output: GuardedOutput,
    {
        match self.try_block_on(future) {
            Ok(value) => value.into_guarded(),
            Err(error) => F::Output::from_boundary_error(error),
        }
    }

    /// [`Self::block_on`] with the boundary failure returned in an **outer**
    /// `Result` instead of folded into the output.
    ///
    /// This is the shape every call site needs whose future already returns a
    /// domain `Result`, or whose output cannot represent failure at all — a
    /// balance `u64`, a `Vec` of peers, a sync summary, a lock guard.
    /// Fabricating a zero balance or an empty peer list out of a panic would
    /// turn a crash into silent, plausible-looking wrong data; folding it into
    /// a domain error would cost it its code and marker. The outer `Err` does
    /// neither.
    #[track_caller]
    pub(crate) fn try_block_on<F>(self, future: F) -> Result<F::Output, FfiBoundaryError>
    where
        F: std::future::Future,
    {
        let rt = runtime_checked()?;
        guard_ffi(|| rt.raw().block_on(future)).into_result()
    }

    /// Fallible access to the runtime itself, for the handful of call sites
    /// that need `enter()`, `spawn()` or `spawn_blocking()` rather than a
    /// blocking drive.
    pub(crate) fn checked(self) -> Result<&'static FfiRuntime, FfiBoundaryError> {
        runtime_checked()
    }

    /// Test-only shorthand for [`test_runtime`], so test bodies that drive
    /// async setup directly keep reading as `runtime().raw().block_on(...)`.
    ///
    /// Deliberately `#[cfg(test)]`: in a test, a runtime that will not build is
    /// a failed test, but in production it is something an entry point has to
    /// *report* — which is why the production sibling ([`Self::checked`]) is
    /// fallible and this one is not.
    #[cfg(test)]
    pub(crate) fn raw(self) -> &'static tokio::runtime::Runtime {
        test_runtime()
    }
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
/// itself (e.g. driving the runtime from inside another runtime), and a runtime
/// that could not be built is reported rather than `expect`ed.
///
/// `F::Output` is reshaped by [`GuardedOutput`] exactly as in
/// [`RuntimeHandle::block_on`]; see that trait for why bare value types are
/// deliberately excluded.
#[track_caller]
pub(crate) fn block_on_worker<F>(future: F) -> <F::Output as GuardedOutput>::Guarded
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static + GuardedOutput,
{
    match try_block_on_worker(future) {
        Ok(value) => value.into_guarded(),
        Err(error) => F::Output::from_boundary_error(error),
    }
}

/// [`block_on_worker`] with the boundary failure in an **outer** `Result`.
///
/// Same rationale as [`RuntimeHandle::try_block_on`]: rather than inventing a
/// value for a `usize` count or a sync summary — or hiding the panic inside a
/// domain error whose FFI mapping would re-code it — the failure is returned as
/// the `Err` half so the entry point turns it into a real error result.
#[track_caller]
pub(crate) fn try_block_on_worker<F>(future: F) -> Result<F::Output, FfiBoundaryError>
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    let location = Location::caller();
    let rt = runtime_checked()?;
    guard_ffi(|| {
        // `raw()`: the surrounding `guard_ffi` already covers this frame, and
        // the join below reports the worker's panic with better context than
        // a second, nested guard could.
        rt.raw().block_on(async move {
            match rt.raw().spawn(future).await {
                Ok(value) => Ok(value),
                Err(join_error) => Err(from_join_error(location, join_error)),
            }
        })
    })
    .into_result()
    // Flatten "the guard caught a panic" and "the join reported one" into the
    // single outer channel the callers handle.
    .and_then(std::convert::identity)
}

/// Convert a [`tokio::task::JoinError`] into the outer boundary error: the
/// replacement for `.expect("tokio worker panicked")`.
///
/// Split out of [`try_block_on_worker`] so both `JoinError` shapes — panicked
/// and cancelled — can be exercised directly by tests; producing a *cancelled*
/// join through the full path would need the worker's `JoinHandle`, which that
/// function owns.
fn from_join_error(
    location: &'static Location<'static>,
    join_error: tokio::task::JoinError,
) -> FfiBoundaryError {
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
    FfiBoundaryError::caught_panic(report_panic(location, &detail))
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
/// A panic inside `f` is reported through the SAME channel as a failed spawn,
/// rather than being re-raised on the calling thread. Joining a panicked scoped
/// thread hands back the payload as a value (the unwind was already contained
/// by the thread boundary); re-raising it — which
/// `.expect("big-stack FFI thread panicked")` used to do — turned a contained
/// panic back into a live one on a thread that unwinds straight into an
/// `extern "C"` abort shim.
///
/// Both shapes come back as [`FfiBoundaryError`], which converts to
/// `ErrorWalletOperation` with its message verbatim. That is what keeps the
/// marker at position 0: while this returned `io::Result`, a call site was free
/// to re-wrap the text (`format!("failed to spawn …: {e}")`) and did, which cost
/// the host the marker on exactly the panic path this exists to report.
#[track_caller]
pub(crate) fn run_on_big_stack_thread<T: Send>(
    f: impl FnOnce() -> T + Send,
) -> Result<T, FfiBoundaryError> {
    let location = Location::caller();
    std::thread::scope(|scope| {
        let handle = std::thread::Builder::new()
            .name("pw-ffi-bigstack".into())
            .stack_size(WORKER_STACK_BYTES)
            .spawn_scoped(scope, f)
            .map_err(|error| {
                FfiBoundaryError::runtime_unavailable(&format!(
                    "failed to spawn the 8 MB FFI worker thread: {error}"
                ))
            })?;
        handle.join().map_err(|payload| {
            FfiBoundaryError::caught_panic(report_panic(
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
    use crate::panic_guard::{FFI_PANIC_PREFIX, FFI_RUNTIME_UNAVAILABLE_PREFIX};

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
        let outcome: Result<(), FfiBoundaryError> = try_block_on_worker(async move {
            // A bounds check on network-supplied data — the panic class the
            // audit calls out, and one that fires in every profile (unlike an
            // overflow check, which only trips where `overflow-checks` is on).
            let payload = [0u8; 4];
            let offset_from_the_wire = std::hint::black_box(7usize);
            let _ = payload[offset_from_the_wire];
        });

        match outcome {
            Ok(()) => PlatformWalletFFIResult::ok(),
            Err(error) => PlatformWalletFFIResult::from(error),
        }
    }

    /// A second stand-in, for the entry-point shape whose async body returns a
    /// **domain** `Result` that the FFI boundary would ordinarily re-map.
    ///
    /// `platform_wallet::PlatformWalletError::TransactionBuild` is a stand-in
    /// for any of the domain errors whose `From` impl re-codes or re-prefixes;
    /// the point of the test below is that a *panic* on this path never reaches
    /// that mapping at all.
    #[no_mangle]
    unsafe extern "C" fn platform_wallet_ffi_test_panicking_domain_entry_point(
    ) -> PlatformWalletFFIResult {
        // The idiom every real call site of this shape uses: intercept the
        // OUTER boundary failure first, then map the domain error.
        let domain: Result<u64, platform_wallet::PlatformWalletError> =
            match try_block_on_worker(async move {
                let payload = [0u8; 4];
                let offset_from_the_wire = std::hint::black_box(7usize);
                Ok(u64::from(payload[offset_from_the_wire]))
            }) {
                Ok(value) => value,
                Err(error) => return error.into(),
            };

        match domain {
            Ok(_) => PlatformWalletFFIResult::ok(),
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

    /// The contract the review asked to be pinned on an **exported path**: a
    /// panic under an entry point whose async body returns a domain `Result`
    /// still arrives as code 6 with the marker at *exactly* position 0.
    ///
    /// This is the regression that the outer-result carrier exists for. With
    /// the panic folded into the domain error instead, this same entry point
    /// would answer with whatever that error's `From` impl decided — for
    /// `Box<dyn Error>` that is `ErrorUnknown` (99) behind an `unclassified
    /// error: ` prefix, which is neither the code nor the position hosts are
    /// told to key on.
    #[test]
    fn exported_domain_result_entry_point_pins_code_six_and_prefix_at_position_zero() {
        let result = unsafe { platform_wallet_ffi_test_panicking_domain_entry_point() };

        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorWalletOperation,
            "code 6 is the documented panic code"
        );
        let message = message_of(&result);
        assert_eq!(
            message.find(FFI_PANIC_PREFIX),
            Some(0),
            "the marker must be at position 0, not merely present: {message}"
        );
        assert!(
            message.contains("index out of bounds"),
            "message must carry the panic payload: {message}"
        );
    }

    /// `try_block_on_worker` covers the outputs that cannot represent failure
    /// themselves (a bare `u64` here), so the panic has to ride the `Err` half.
    #[test]
    fn try_block_on_worker_surfaces_a_panic_as_err() {
        let result = try_block_on_worker(async move { panic!("counting boom") });

        let error = result.expect_err("a panicking worker must not report success");
        assert!(error.to_string().starts_with(FFI_PANIC_PREFIX));
        assert!(error.to_string().contains("counting boom"));
        // The success path still yields the bare value untouched.
        assert_eq!(
            try_block_on_worker(async move { 7u64 }).expect("no panic"),
            7
        );
    }

    /// `RuntimeHandle::block_on` keeps the name tokio's had, so the many
    /// `runtime().block_on(...)` entry points are guarded without being
    /// rewritten.
    #[test]
    fn runtime_block_on_is_guarded_and_passes_values_through() {
        let ok = runtime().block_on(async { PlatformWalletFFIResult::ok() });
        assert_eq!(ok.code, PlatformWalletFFIResultCode::Success);

        let result = runtime().block_on(async {
            panic!("local boom");
            #[allow(unreachable_code)]
            PlatformWalletFFIResult::ok()
        });
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorWalletOperation
        );
        assert!(message_of(&result).contains("local boom"));

        let value: Result<u64, FfiBoundaryError> = runtime().try_block_on(async { 9u64 });
        assert_eq!(value.expect("no panic"), 9);
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

        let panicked: tokio::task::JoinError = test_runtime().block_on(async {
            tokio::spawn(async { panic!("joined boom") })
                .await
                .expect_err("the task panicked")
        });
        assert!(panicked.is_panic());
        let result: PlatformWalletFFIResult = from_join_error(location, panicked).into();
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorWalletOperation
        );
        let message = message_of(&result);
        assert!(message.starts_with(FFI_PANIC_PREFIX));
        assert!(message.contains("joined boom"), "{message}");

        let cancelled: tokio::task::JoinError = test_runtime().block_on(async {
            let handle = tokio::spawn(std::future::pending::<()>());
            handle.abort();
            handle.await.expect_err("the task was aborted")
        });
        assert!(cancelled.is_cancelled());
        let error = from_join_error(location, cancelled);
        assert!(
            error.to_string().contains("did not complete"),
            "a cancelled worker must not be reported as a panic: {error}"
        );
    }

    /// A panic on the big-stack path is reported through the boundary error the
    /// call sites already handle, rather than re-raised into the abort shim.
    #[test]
    fn run_on_big_stack_thread_reports_a_panic_as_a_boundary_error() {
        let result = run_on_big_stack_thread(|| panic!("big-stack boom"));

        let error = result.expect_err("a panicking pass must not report success");
        let rendered = error.to_string();
        assert!(rendered.starts_with(FFI_PANIC_PREFIX), "{rendered}");
        assert!(rendered.contains("big-stack boom"), "{rendered}");

        // …and it keeps code 6 with the marker still at position 0 once it
        // reaches the host, because the only conversion it has is verbatim.
        let ffi: PlatformWalletFFIResult = error.into();
        assert_eq!(ffi.code, PlatformWalletFFIResultCode::ErrorWalletOperation);
        assert_eq!(message_of(&ffi).find(FFI_PANIC_PREFIX), Some(0));
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

    /// The runtime builds, and acquisition is the fallible call every entry
    /// path now goes through.
    #[test]
    fn runtime_acquisition_is_fallible_and_succeeds_here() {
        let runtime = runtime_checked().expect("the runtime must build on a test host");
        assert!(runtime.raw().metrics().num_workers() > 0);
    }

    /// A runtime that could not be built reaches the host as the generic code
    /// with its own marker at position 0 — the same shape as a caught panic,
    /// and never an abort.
    ///
    /// The failure itself cannot be provoked on a healthy test host (that is
    /// the point of the `Lazy`), so this pins the conversion the `Err` arm of
    /// [`runtime_checked`] feeds.
    #[test]
    fn an_unavailable_runtime_is_a_generic_error_result_not_an_abort() {
        let error = FfiBoundaryError::runtime_unavailable(
            "failed to create the tokio runtime for platform-wallet-ffi: too many open files",
        );
        let result: PlatformWalletFFIResult = error.into();

        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorWalletOperation
        );
        let message = message_of(&result);
        assert_eq!(
            message.find(FFI_RUNTIME_UNAVAILABLE_PREFIX),
            Some(0),
            "hosts key on the marker at position 0: {message}"
        );
        assert!(message.contains("too many open files"), "{message}");
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
