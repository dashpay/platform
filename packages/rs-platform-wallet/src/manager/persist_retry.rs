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
    let op = Arc::new(op);
    for (attempt, backoff) in LOAD_RETRY_BACKOFF
        .iter()
        .map(Some)
        .chain([None])
        .enumerate()
    {
        let call = Arc::clone(&op);
        let result = match tokio::task::spawn_blocking(move || call()).await {
            Ok(result) => result,
            Err(join_err) if join_err.is_panic() => {
                std::panic::resume_unwind(join_err.into_panic())
            }
            Err(_cancelled) => {
                return Err(PersistenceError::backend(
                    "runtime shutting down before load retry",
                ))
            }
        };
        match result {
            Ok(value) => return Ok(value),
            Err(e) if e.is_transient() => {
                let Some(backoff) = backoff else {
                    return Err(e);
                };
                tracing::debug!(
                    attempt,
                    backoff_ms = backoff.as_millis() as u64,
                    error = %e,
                    "transient persister load failure — retrying"
                );
                tokio::time::sleep(*backoff).await;
            }
            Err(e) => return Err(e),
        }
    }
    unreachable!("the None-terminated schedule always returns on its final iteration")
}
