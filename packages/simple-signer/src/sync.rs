//! Async-sync bridge utilities.
//!
//! TODO: Unify with `dash-sdk::sync::block_on` by extracting to a shared
//! `rs-async-bridge` micro-crate. See <https://github.com/dashpay/platform/issues/3399>.

use std::{fmt::Debug, future::Future, sync::mpsc::SendError};

#[derive(Debug, thiserror::Error)]
pub enum AsyncError {
    /// Not running inside tokio runtime
    #[cfg(not(target_arch = "wasm32"))]
    #[error("not running inside tokio runtime: {0}")]
    NotInTokioRuntime(#[from] tokio::runtime::TryCurrentError),

    /// Cannot receive response from async function
    #[error("cannot receive response from async function: {0}")]
    RecvError(#[from] std::sync::mpsc::RecvError),

    /// Cannot send response from async function
    #[error("cannot send response from async function: {0}")]
    SendError(String),

    #[error("asynchronous call from synchronous context failed: {0}")]
    #[allow(unused)]
    Generic(String),
}

impl<T> From<SendError<T>> for AsyncError {
    fn from(error: SendError<T>) -> Self {
        Self::SendError(error.to_string())
    }
}

/// Blocks on the provided future and returns the result.
///
/// This function is used to call async functions from sync code.
/// Requires the current thread to be running in a tokio runtime.
///
/// Due to limitations of tokio runtime, we cannot use
/// `tokio::runtime::Runtime::block_on` if we are already inside a tokio
/// runtime. This function is a workaround for that limitation.
#[cfg(not(target_arch = "wasm32"))]
pub fn block_on<F>(fut: F) -> Result<F::Output, AsyncError>
where
    F: Future + Send + 'static,
    F::Output: Send,
{
    tracing::trace!("block_on: running async function from sync code");
    let rt = tokio::runtime::Handle::try_current()?;
    let (tx, rx) = std::sync::mpsc::channel();
    tracing::trace!("block_on: Spawning worker");
    let hdl = rt.spawn(worker(fut, tx));
    tracing::trace!("block_on: Worker spawned");
    let resp = tokio::task::block_in_place(|| rx.recv())?;

    tracing::trace!("Response received");
    if !hdl.is_finished() {
        tracing::debug!("async-sync worker future is not finished, aborting; this should not happen, but it's fine");
        hdl.abort();
    }

    Ok(resp)
}

#[cfg(target_arch = "wasm32")]
pub fn block_on<F>(_fut: F) -> Result<F::Output, AsyncError>
where
    F: Future + Send + 'static,
    F::Output: Send,
{
    unimplemented!("block_on is not supported in wasm");
}

/// Worker function that runs the provided future and sends the result back
/// to the caller using mpsc channel.
#[cfg(not(target_arch = "wasm32"))]
async fn worker<F: Future>(
    fut: F,
    response: std::sync::mpsc::Sender<F::Output>,
) -> Result<(), AsyncError> {
    tracing::trace!("Worker start");
    let result = fut.await;
    tracing::trace!("Worker async function completed, sending response");
    response.send(result)?;
    tracing::trace!("Worker response sent");

    Ok(())
}
