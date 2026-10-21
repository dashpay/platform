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

/// Wrap every blocking runtime entry so callers cannot lose a confirmed
/// preparation while their future waits to be polled.
pub(crate) struct WalletRuntime(tokio::runtime::Runtime);

impl WalletRuntime {
    pub(crate) fn block_on<F: std::future::Future>(&self, future: F) -> F::Output {
        self.0.block_on(dash_sdk::with_platform_write_permits(
            crate::preparation::current_permits(),
            future,
        ))
    }
}

impl std::ops::Deref for WalletRuntime {
    type Target = tokio::runtime::Runtime;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Get the shared tokio runtime.
///
/// All async FFI functions use this runtime. Prefer
/// [`block_on_worker`] over `runtime().block_on(...)` so the heavy
/// work runs on a worker thread with the larger stack configured
/// here, rather than the (small) calling thread.
pub(crate) fn runtime() -> &'static WalletRuntime {
    static RT: once_cell::sync::Lazy<WalletRuntime> = once_cell::sync::Lazy::new(|| {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_stack_size(WORKER_STACK_BYTES)
            .build()
            .expect("Failed to create tokio runtime for platform-wallet-ffi");

        #[cfg(feature = "tokio-metrics")]
        metrics::spawn_sampler(&rt);

        WalletRuntime(rt)
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
    // Tokio does not inherit task locals when spawning. Scope the worker's
    // future itself, rather than only the caller awaiting its JoinHandle.
    let permits = crate::preparation::current_permits();
    rt.0.block_on(async move {
        rt.spawn(dash_sdk::with_platform_write_permits(permits, future))
            .await
            .expect("tokio worker panicked")
    })
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
/// A panic inside `f` is propagated as a panic here, matching
/// [`block_on_worker`]'s "tokio worker panicked" convention — a panic
/// in the pass is a bug, not a recoverable condition.
pub(crate) fn run_on_big_stack_thread<T: Send>(f: impl FnOnce() -> T + Send) -> std::io::Result<T> {
    let permits = crate::preparation::current_permits();
    std::thread::scope(|scope| {
        let handle = std::thread::Builder::new()
            .name("pw-ffi-bigstack".into())
            .stack_size(WORKER_STACK_BYTES)
            .spawn_scoped(scope, move || crate::preparation::with_permits(permits, f))?;
        Ok(handle.join().expect("big-stack FFI thread panicked"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use dash_sdk::{
        Error, ProtocolCompatibilityError, ProtocolCompatibilityPolicy, Sdk, SdkBuilder,
    };
    use dpp::block::extended_epoch_info::v0::ExtendedEpochInfoV0;
    use dpp::block::extended_epoch_info::ExtendedEpochInfo;
    use dpp::version::PlatformVersion;

    fn verified_sdk() -> Sdk {
        let mut sdk = SdkBuilder::new_mock().build().expect("mock sdk");
        sdk.set_protocol_compatibility_policy(ProtocolCompatibilityPolicy::RequireVerified);
        runtime().block_on(async {
            sdk.mock()
                .expect_fetch::<ExtendedEpochInfo, _>(
                    0u16,
                    Some(ExtendedEpochInfo::from(ExtendedEpochInfoV0 {
                        index: 0,
                        first_block_time: 0,
                        first_block_height: 0,
                        first_core_block_height: 0,
                        fee_multiplier_permille: 0,
                        protocol_version: PlatformVersion::latest().protocol_version,
                    })),
                )
                .await
                .expect("proven check");
            let generation = sdk.begin_protocol_compatibility_check();
            sdk.refresh_protocol_compatibility(generation).await;
        });
        sdk.platform_write_permit().expect("verified SDK");
        sdk
    }

    #[test]
    fn should_keep_queued_preparation_across_every_wallet_runtime_entry() {
        let sdk = verified_sdk();
        let original = sdk.platform_write_permit().expect("original preparation");
        let next = sdk.begin_protocol_compatibility_check();
        runtime().block_on(sdk.refresh_protocol_compatibility(next));
        sdk.platform_write_permit().expect("new check completed");
        let expired = |result| {
            assert!(matches!(
                result,
                Err(Error::ProtocolCompatibility(
                    ProtocolCompatibilityError::PreparationExpired
                ))
            ));
        };
        // The caller has a confirmed old permit, but native execution starts
        // only after a subsequent lifecycle check is fully supported again.
        crate::preparation::with_permits(vec![original.clone()], || {
            expired(runtime().block_on(async { sdk.platform_write_permit() }));
            let worker_sdk = sdk.clone();
            expired(block_on_worker(async move {
                tokio::task::yield_now().await;
                worker_sdk.platform_write_permit()
            }));
            expired(
                run_on_big_stack_thread(|| {
                    runtime().block_on(async { sdk.platform_write_permit() })
                })
                .expect("big-stack worker"),
            );
        });
        let current = sdk.platform_write_permit().expect("current preparation");
        let barrier = std::sync::Arc::new(tokio::sync::Barrier::new(2));
        std::thread::scope(|threads| {
            let stale_sdk = sdk.clone();
            let stale_barrier = barrier.clone();
            let stale = threads.spawn(|| {
                // A nested current scope must not erase the old confirmation.
                crate::preparation::with_permits(vec![original, current.clone()], || {
                    block_on_worker(async move {
                        stale_barrier.wait().await;
                        stale_sdk.platform_write_permit()
                    })
                })
            });
            let fresh_sdk = sdk.clone();
            let fresh = threads.spawn(|| {
                crate::preparation::with_permits(vec![current.clone()], || {
                    block_on_worker(async move {
                        barrier.wait().await;
                        fresh_sdk.platform_write_permit()
                    })
                })
            });
            expired(stale.join().expect("stale request"));
            assert_eq!(
                fresh
                    .join()
                    .expect("fresh request")
                    .expect("independent scope"),
                current
            );
        });
        // Worker request state cannot leak into later, freshly confirmed work.
        block_on_worker(async move { sdk.platform_write_permit() })
            .expect("unscoped operation remains available");
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
