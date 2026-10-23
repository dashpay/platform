//! Argument-grammar tests for `#[stack_size]`'s `worker_threads` override.
//!
//! Async bodies always run on a multi-threaded runtime; `worker_threads = N`
//! sets its worker count and the bare form defaults to two workers.

use dash_platform_macros::stack_size;

/// `worker_threads = N` must compile and drive the body
/// on an `N`-worker multi-threaded runtime. `block_in_place` panics on a
/// current-thread runtime, so it doubles as a runtime-flavor assertion.
#[stack_size(8 * 1024 * 1024, worker_threads = 3)]
#[test]
async fn worker_threads_runs_on_n_worker_runtime() {
    let sum = tokio::task::block_in_place(|| 1 + 1);
    assert_eq!(sum, 2);
    assert_eq!(
        tokio::runtime::Handle::current().metrics().num_workers(),
        3,
        "worker_threads = 3 must configure a 3-worker runtime"
    );
}

/// A bare `#[stack_size(EXPR)]` async body defaults to the two-worker runtime.
#[stack_size(8 * 1024 * 1024)]
#[test]
async fn bare_stack_size_defaults_to_two_workers() {
    assert_eq!(
        tokio::runtime::Handle::current().metrics().num_workers(),
        2,
        "the single-arg form must default to two workers"
    );
}
