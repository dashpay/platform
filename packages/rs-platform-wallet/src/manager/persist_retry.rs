//! Bounded retry for transient persister *reads*.
//!
//! Only `load` is retried in-crate: it is idempotent and the crate owns both
//! ends. A failed `store` propagates typed and kind-classified instead, and the
//! caller decides.
//!
//! Each attempt runs on the blocking pool; worst case per call is
//! `attempts × backend timeout + Σ backoff` (SQLite `busy_timeout` defaults to
//! 5 s).

use std::sync::Arc;
use std::time::Duration;

use crate::changeset::PersistenceError;

/// Backoff before each retry of a transient `load` failure. Four total
/// attempts (the initial call plus one per entry).
pub(crate) const LOAD_RETRY_BACKOFF: [Duration; 3] = [
    Duration::from_millis(20),
    Duration::from_millis(40),
    Duration::from_millis(80),
];

/// Retry a synchronous persister `load` while it fails *transiently*, off the
/// async runtime, on the fixed [`LOAD_RETRY_BACKOFF`] schedule.
///
/// `op` runs on the blocking pool once per attempt; success or a fatal error
/// returns immediately. A panic inside `op` propagates to the caller; a
/// cancelled attempt (runtime shutting down) surfaces as a backend error.
pub(crate) async fn retry_transient_load<T, F>(op: F) -> Result<T, PersistenceError>
where
    F: Fn() -> Result<T, PersistenceError> + Send + Sync + 'static,
    T: Send + 'static,
{
    // The initial call, then one retry per backoff entry: the loop runs the
    // schedule and the last attempt's result falls out of it, so there is no
    // terminating sentinel and no escape hatch to panic through.
    let op = Arc::new(op);
    let mut outcome = load_attempt(&op).await;
    for (retries_done, backoff) in LOAD_RETRY_BACKOFF.iter().enumerate() {
        match outcome {
            Ok(value) => return Ok(value),
            Err(e) if e.is_transient() => {
                tracing::debug!(
                    // 1-based, matching the schedule this module documents.
                    attempt = retries_done + 1,
                    backoff_ms = backoff.as_millis() as u64,
                    error = %e,
                    "transient persister load failure — retrying"
                );
                tokio::time::sleep(*backoff).await;
                outcome = load_attempt(&op).await;
            }
            Err(e) => return Err(e),
        }
    }
    outcome
}

/// Run one `load` attempt on the blocking pool.
// TODO(load-retry-holds-persister-strong-ref): an in-flight load retry holds a
// strong persister reference the caller cannot reclaim, contradicting the
// documented "only a batch commit holds one" relationship — `spawn_blocking` is
// uncancellable, so an abandoned caller's `Arc<P>` stays alive until the
// backend call returns.
async fn load_attempt<T, F>(op: &Arc<F>) -> Result<T, PersistenceError>
where
    F: Fn() -> Result<T, PersistenceError> + Send + Sync + 'static,
    T: Send + 'static,
{
    let call = Arc::clone(op);
    match tokio::task::spawn_blocking(move || call()).await {
        Ok(result) => result,
        Err(join_err) if join_err.is_panic() => std::panic::resume_unwind(join_err.into_panic()),
        Err(_cancelled) => Err(PersistenceError::backend(
            "runtime shutting down before load retry",
        )),
    }
}
