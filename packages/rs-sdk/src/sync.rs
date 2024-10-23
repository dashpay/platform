//! Handle async calls from sync code.
//!
//! This is a workaround for an issue in tokio, where you cannot call `block_on` from sync call that is called
//! inside a tokio runtime. This module spawns async futures in active tokio runtime, and retrieves the result
//! using a channel.
use backon::Retryable;
use drive_proof_verifier::error::ContextProviderError;
use futures::future::BoxFuture;
use rs_dapi_client::{CanRetry, ExecutionError, ExecutionResponse};
use std::{
    fmt::Debug,
    future::Future,
    sync::mpsc::SendError,
};
use tokio::runtime::TryCurrentError;
#[derive(Debug, thiserror::Error)]
pub enum AsyncError {
    /// Not running inside tokio runtime
    #[error("not running inside tokio runtime: {0}")]
    NotInTokioRuntime(#[from] TryCurrentError),

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

impl From<AsyncError> for ContextProviderError {
    fn from(error: AsyncError) -> Self {
        ContextProviderError::AsyncError(error.to_string())
    }
}

impl From<AsyncError> for crate::Error {
    fn from(error: AsyncError) -> Self {
        Self::ContextProviderError(error.into())
    }
}

/// Blocks on the provided future and returns the result.
///
/// This function is used to call async functions from sync code.
/// Requires the current thread to be running in a tokio runtime.
///
/// Due to limitations of tokio runtime, we cannot use `tokio::runtime::Runtime::block_on` if we are already inside a tokio runtime.
/// This function is a workaround for that limitation.
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
        hdl.abort(); // cleanup the worker future
    }

    Ok(resp)
}

/// Worker function that runs the provided future and sends the result back to the caller using oneshot channel.
async fn worker<F: Future>(
    fut: F,
    // response: oneshot::Sender<F::Output>,
    response: std::sync::mpsc::Sender<F::Output>,
) -> Result<(), AsyncError> {
    tracing::trace!("Worker start");
    let result = fut.await;
    tracing::trace!("Worker async function completed, sending response");
    response.send(result)?;
    tracing::trace!("Worker response sent");

    Ok(())
}

/// Retries the provided future `count` times.
pub async fn retry<'a, F, T, E>(retry_factory: F,configured_retries:  usize) ->Result<ExecutionResponse<T>, ExecutionError<E>>
where
    F: FnMut() -> BoxFuture<'a, Result<ExecutionResponse<T>, ExecutionError<E>>>,
    E: CanRetry + Debug,
{
    // TODO: make configurable
    let backoff_strategy = backon::ConstantBuilder::default()
        .with_delay(std::time::Duration::from_millis(10)) // we use different server, so no real delay needed, just to avoid spamming
        .with_max_times(configured_retries); // no retries by default

    // let retries = atomic::AtomicUsize::new(0);
    let retries: usize = 0;

    let  result= retry_factory.retry(backoff_strategy)
        .when(|e| {
            if e.can_retry() {
                
                // retries used in this attempt; `e.retries` on rs-dapi-client-layer and `+1` on sdk layer
                let used = e.retries + 1;
                
                // retries used in all preceeding attempts
                let retries_so_far = retries+used;//  retries.fetch_add(used, Ordering::Relaxed) + used; // relaxed as only 1 thread accesses that
                if retries_so_far >= configured_retries {
                    tracing::warn!(retry = retries_so_far, error=?e, "retrying request");
                    true
                } else {
                    tracing::warn!(retry = retries_so_far, error=?e, "no more retries left, giving up");
                    false
                }
            } else {
                false
            }
    })
    .notify(|error, duration| {
        tracing::warn!(?duration, ?error, "request failed, retrying");
    })
    .await;

    let retry_count = retries;
    result.map_err(|mut e| {
        e.retries = retry_count;
        e
    })
}

#[cfg(test)]
mod test {
    use super::*;
    use std::future::Future;
    use tokio::{
        runtime::Builder,
        sync::mpsc::{self, Receiver},
    };

    /// Test for block_on with async code that calls sync code, which then calls async code again.
    ///
    /// Given: An async function that calls a sync function, which then calls another async function.
    /// When: The async function is executed using block_on.
    /// Then: Other threads can still do some work
    #[test]
    fn test_block_on_nested_async_sync() {
        let rt = Builder::new_multi_thread()
            .worker_threads(1) // we should be good with 1 worker thread
            .max_blocking_threads(1) // we should be good with 1 blocking thread
            .enable_all()
            .build()
            .expect("Failed to create Tokio runtime");
        // we repeat this test a few times, to make sure it's stable
        for _repeat in 0..5 {
            // Create a Tokio runtime; we use the current thread runtime for this test.

            const MSGS: usize = 10;
            let (tx, rx) = mpsc::channel::<usize>(1);

            // Spawn new worker that will just count.
            let worker = async move {
                for count in 0..MSGS {
                    tx.send(count).await.unwrap();
                }
            };
            let worker_join = rt.spawn(worker);
            // Define the number of levels of execution
            let levels = 4;

            // Define the innermost async function
            async fn innermost_async_function(
                mut rx: Receiver<usize>,
            ) -> Result<String, ContextProviderError> {
                for i in 0..MSGS {
                    let count = rx.recv().await.unwrap();
                    assert_eq!(count, i);
                }

                Ok(String::from("Success"))
            }

            // Define the nested sync function
            fn nested_sync_function<F>(fut: F) -> Result<String, ContextProviderError>
            where
                F: Future<Output = Result<String, ContextProviderError>> + Send + 'static,
                F::Output: Send,
            {
                block_on(fut)?.map_err(|e| ContextProviderError::Generic(e.to_string()))
            }

            // Define the outer async function
            async fn outer_async_function(
                levels: usize,
                rx: Receiver<usize>,
            ) -> Result<String, ContextProviderError> {
                let mut result = innermost_async_function(rx).await;
                for _ in 0..levels {
                    result = nested_sync_function(async { result });
                }
                result
            }

            // Run the outer async function using block_on
            let result = rt.block_on(outer_async_function(levels, rx));

            rt.block_on(worker_join).unwrap();
            // Assert the result
            assert_eq!(result.unwrap(), "Success");
        }
    }
}
