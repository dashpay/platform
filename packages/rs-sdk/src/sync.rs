//! Handle async calls from sync code.
//!
//! This is a workaround for an issue in tokio, where you cannot call `block_on` from sync call that is called
//! inside a tokio runtime. This module spawns async futures in active tokio runtime, and retrieves the result
//! using a channel.
use drive_proof_verifier::error::ContextProviderError;
use std::future::Future;

#[derive(Debug, thiserror::Error)]
pub enum AsyncError {
    #[error("asynchronous call from synchronous context failed: {0}")]
    #[allow(unused)]
    Generic(String),
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
pub fn block_on<F>(fut: F) -> Result<F::Output, ContextProviderError>
where
    F: Future + Send + 'static,
    F::Output: Send,
{
    tracing::trace!("block_on: running async function from sync code");
    let rt = tokio::runtime::Handle::try_current().map_err(|e| {
        ContextProviderError::AsyncError(format!(
            "sync-async error: cannot get current tokio runtime handle: {:?}",
            e
        ))
    })?;
    let (tx, rx) = std::sync::mpsc::channel();
    tracing::trace!("block_on: Spawning worker");
    let hdl = rt.spawn(worker(fut, tx));
    tracing::trace!("block_on: Worker spawned");
    let recv = tokio::task::block_in_place(|| rx.recv());

    let resp = recv.map_err(|e| {
        ContextProviderError::AsyncError(format!(
            "sync-async error: cannot receive response from async function: {:?}",
            e
        ))
    })?;

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
) -> Result<(), drive_proof_verifier::error::ContextProviderError> {
    tracing::trace!("Worker start");
    let result = fut.await;
    tracing::trace!("Worker async function completed, sending response");
    response.send(result).map_err(|e| {
        ContextProviderError::Generic(format!("sync-async error: response cannot be sent: {}", e))
    })?;
    tracing::trace!("Worker response sent");

    Ok(())
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
