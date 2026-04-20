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
///
/// Due to limitations of tokio runtime, we cannot use `tokio::runtime::Runtime::block_on`
/// if we are already inside a tokio runtime. This function is a workaround for that limitation.
///
/// Handles three scenarios:
/// - No active runtime: creates a temporary current-thread runtime and drives the future directly.
/// - Current-thread runtime: spawns a dedicated OS thread with its own independent runtime,
///   since `block_in_place` panics when there are no other worker threads.
/// - Any other runtime flavor (multi-thread, etc.): uses `block_in_place` + spawn for efficient bridging.
#[cfg(not(target_arch = "wasm32"))]
pub fn block_on<F>(fut: F) -> Result<F::Output, AsyncError>
where
    F: Future + Send + 'static,
    F::Output: Send,
{
    use tokio::runtime::RuntimeFlavor;

    tracing::trace!("block_on: running async function from sync code");

    let handle = match tokio::runtime::Handle::try_current() {
        Ok(h) => h,
        Err(e) => {
            tracing::trace!("block_on: no active runtime ({e}), creating temporary runtime");
            return Ok(tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| AsyncError::Generic(e.to_string()))?
                .block_on(fut));
        }
    };

    match handle.runtime_flavor() {
        RuntimeFlavor::CurrentThread => {
            tracing::trace!("block_on: current-thread runtime, spawning dedicated OS thread");
            let (tx, rx) = std::sync::mpsc::sync_channel::<Result<F::Output, AsyncError>>(1);
            let join_handle = std::thread::spawn(move || {
                let result = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| {
                        tracing::error!("block_on: failed to create worker runtime: {}", e);
                        AsyncError::Generic(format!("failed to create worker runtime: {e}"))
                    })
                    .map(|rt| rt.block_on(fut));
                let _ = tx.send(result);
            });
            let recv_result = rx.recv();
            let join_result = join_handle.join();
            match (join_result, recv_result) {
                (Err(_), _) => Err(AsyncError::Generic(
                    "block_on worker thread panicked".to_string(),
                )),
                (Ok(()), Err(_)) => Err(AsyncError::Generic(
                    "block_on worker exited without sending a result".to_string(),
                )),
                (Ok(()), Ok(result)) => result,
            }
        }
        // RuntimeFlavor is #[non_exhaustive]; all multi-threaded flavors (MultiThread,
        // MultiThreadAlt, and any future variants) support block_in_place.
        _ => {
            tracing::trace!("block_on: multi-thread runtime, using block_in_place");
            let (tx, rx) = std::sync::mpsc::sync_channel::<F::Output>(1);
            let hdl = handle.spawn(worker(fut, tx));
            let resp = tokio::task::block_in_place(|| rx.recv())?;
            if !hdl.is_finished() {
                tracing::debug!("async-sync worker future is not finished, aborting");
                hdl.abort();
            }
            Ok(resp)
        }
    }
}

/// WASM stub for `block_on`.
///
/// True async-to-sync bridging on WASM requires the JS Promise Integration (JSPI)
/// proposal, which is not yet supported by wasm-bindgen.
/// See <https://github.com/rustwasm/wasm-bindgen/issues/3633>.
///
/// Until JSPI lands, WASM callers must use async interfaces directly
/// via `#[wasm_bindgen]` instead.
///
/// The `Send` / `'static` bounds from the native signature are intentionally
/// dropped here: WASM is single-threaded and this stub never drives the
/// future, so requiring them would reject otherwise-valid callers whose
/// futures capture non-`Send` WASM types like `JsFuture` or
/// `reqwest::Response`.
#[cfg(target_arch = "wasm32")]
pub fn block_on<F>(_fut: F) -> Result<F::Output, AsyncError>
where
    F: Future,
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
    response: std::sync::mpsc::SyncSender<F::Output>,
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

    /// Regression test for https://github.com/dashpay/platform/issues/3432.
    ///
    /// `block_on` previously called `tokio::task::block_in_place` unconditionally, which
    /// panics on a current-thread (single-threaded) tokio runtime.  The fix detects the
    /// runtime flavor and spawns a dedicated OS thread with its own runtime when running
    /// on a current-thread scheduler.
    #[test]
    fn test_block_on_succeeds_on_current_thread_runtime() {
        let rt = Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create current-thread Tokio runtime");

        const MSGS: usize = 3;
        let (tx, rx) = mpsc::channel::<usize>(1);

        let worker_task = async move {
            for count in 0..MSGS {
                tx.send(count).await.unwrap();
            }
        };
        let worker_join = rt.spawn(worker_task);

        async fn innermost(mut rx: Receiver<usize>) -> Result<String, String> {
            for i in 0..MSGS {
                let count = rx.recv().await.unwrap();
                assert_eq!(count, i);
            }
            Ok("Success".to_string())
        }

        fn sync_bridge<F>(fut: F) -> Result<String, String>
        where
            F: Future<Output = Result<String, String>> + Send + 'static,
            F::Output: Send,
        {
            block_on(fut)
                .map_err(|e| e.to_string())?
                .map_err(|e| e.to_string())
        }

        async fn outer(rx: Receiver<usize>) -> Result<String, String> {
            let result = innermost(rx).await;
            sync_bridge(async { result })
        }

        let result = rt.block_on(outer(rx));

        rt.block_on(worker_join).ok();

        assert_eq!(
            result.unwrap(),
            "Success",
            "block_on should succeed on a current-thread runtime"
        );
    }
}
