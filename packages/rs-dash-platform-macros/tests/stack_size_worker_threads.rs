//! Argument-grammar tests for `#[stack_size]`'s `worker_threads` override.
//!
//! Async bodies always run on a multi-threaded runtime, so `worker_threads = N`
//! is meaningful on its own — `multi_thread` is an accepted but redundant legacy
//! token. These tests pin both spellings: the standalone override and the legacy
//! `multi_thread, worker_threads = N` shape still used by consumers such as the
//! platform-wallet `tk_001` e2e case.

use dash_platform_macros::stack_size;

/// `worker_threads = N` without `multi_thread` must compile and drive the body
/// on an `N`-worker multi-threaded runtime. `block_in_place` panics on a
/// current-thread runtime, so it doubles as a runtime-flavor assertion.
#[stack_size(8 * 1024 * 1024, worker_threads = 3)]
#[test]
async fn worker_threads_without_multi_thread_runs_on_n_worker_runtime() {
    let sum = tokio::task::block_in_place(|| 1 + 1);
    assert_eq!(sum, 2);
    assert_eq!(
        tokio::runtime::Handle::current().metrics().num_workers(),
        3,
        "worker_threads = 3 must configure a 3-worker runtime"
    );
}

/// Smoke test mirroring `tk_001`'s attribute shape: the legacy
/// `multi_thread, worker_threads = N` form still parses and yields `N` workers.
#[stack_size(8 * 1024 * 1024, multi_thread, worker_threads = 4)]
#[test]
async fn legacy_multi_thread_with_worker_threads_still_compiles() {
    let sum = tokio::task::block_in_place(|| 2 + 2);
    assert_eq!(sum, 4);
    assert_eq!(
        tokio::runtime::Handle::current().metrics().num_workers(),
        4,
        "legacy multi_thread, worker_threads = 4 must still yield 4 workers"
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
