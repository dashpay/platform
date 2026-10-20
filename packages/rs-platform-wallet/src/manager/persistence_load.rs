//! Persister reads performed off the async runtime.

use crate::changeset::PersistenceError;

/// Run one synchronous persister read on the blocking pool.
// TODO(load-holds-persister-strong-ref): `spawn_blocking` is uncancellable, so
// an abandoned caller retains its persister until `load` returns.
pub(crate) async fn run_blocking_load<T, F>(op: F) -> Result<T, PersistenceError>
where
    F: FnOnce() -> Result<T, PersistenceError> + Send + 'static,
    T: Send + 'static,
{
    match tokio::task::spawn_blocking(op).await {
        Ok(result) => result,
        Err(join_err) if join_err.is_panic() => std::panic::resume_unwind(join_err.into_panic()),
        Err(_cancelled) => Err(PersistenceError::backend(
            "runtime shutting down during persister load",
        )),
    }
}
