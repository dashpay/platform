use std::future::Future;
use std::sync::mpsc::SendError;

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
/// Due to limitations of tokio runtime, we cannot use `tokio::runtime::Runtime::block_on` if we are already inside a tokio runtime.
/// This function is a workaround for that limitation.
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

/// WASM stub for `block_on`.
///
/// True async-to-sync bridging on WASM requires the JS Promise Integration (JSPI)
/// proposal, which is not yet supported by wasm-bindgen.
/// See <https://github.com/rustwasm/wasm-bindgen/issues/3633>.
///
/// Until JSPI lands, WASM callers must use async interfaces directly
/// via `#[wasm_bindgen]` instead.
#[cfg(target_arch = "wasm32")]
pub fn block_on<F>(_fut: F) -> Result<F::Output, AsyncError>
where
    F: Future + Send + 'static,
    F::Output: Send,
{
    Err(AsyncError::Generic(
        "block_on is not yet supported in WASM. \
         Awaiting wasm-bindgen JSPI support \
         (https://github.com/rustwasm/wasm-bindgen/issues/3633). \
         Use async callers via #[wasm_bindgen] instead."
            .to_string(),
    ))
}

/// Worker function that runs the provided future and sends the result back to the caller using mpsc channel.
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

#[cfg(test)]
mod test {
    use super::*;
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
            .worker_threads(1)
            .max_blocking_threads(1)
            .enable_all()
            .build()
            .expect("Failed to create Tokio runtime");

        for _repeat in 0..5 {
            const MSGS: usize = 10;
            let (tx, rx) = mpsc::channel::<usize>(1);

            let worker_task = async move {
                for count in 0..MSGS {
                    tx.send(count).await.unwrap();
                }
            };
            let worker_join = rt.spawn(worker_task);

            let levels = 4;

            async fn innermost_async_function(mut rx: Receiver<usize>) -> Result<String, String> {
                for i in 0..MSGS {
                    let count = rx.recv().await.unwrap();
                    assert_eq!(count, i);
                }
                Ok(String::from("Success"))
            }

            fn nested_sync_function<F>(fut: F) -> Result<String, String>
            where
                F: Future<Output = Result<String, String>> + Send + 'static,
                F::Output: Send,
            {
                block_on(fut)
                    .map_err(|e| e.to_string())?
                    .map_err(|e| e.to_string())
            }

            async fn outer_async_function(
                levels: usize,
                rx: Receiver<usize>,
            ) -> Result<String, String> {
                let mut result = innermost_async_function(rx).await;
                for _ in 0..levels {
                    result = nested_sync_function(async { result });
                }
                result
            }

            let result = rt.block_on(outer_async_function(levels, rx));
            rt.block_on(worker_join).unwrap();
            assert_eq!(result.unwrap(), "Success");
        }
    }
}
