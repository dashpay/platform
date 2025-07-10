//! Handle async calls from sync code.
//!
//! This is a workaround for an issue in tokio, where you cannot call `block_on` from sync call that is called
//! inside a tokio runtime. This module spawns async futures in active tokio runtime, and retrieves the result
//! using a channel.

use arc_swap::ArcSwap;
use drive_proof_verifier::error::ContextProviderError;
use rs_dapi_client::{
    update_address_ban_status, AddressList, CanRetry, ExecutionResult, RequestSettings,
};
use std::fmt::Display;
use std::{
    fmt::Debug,
    future::Future,
    sync::{mpsc::SendError, Arc},
};
use tokio::sync::Mutex;

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
        hdl.abort(); // cleanup the worker future
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

/// Retry the provided closure.
///
/// This function is used to retry async code. It takes into account number of retries already executed by lower
/// layers and stops retrying once the maximum number of retries is reached.
///
/// The `settings` should contain maximum number of retries that should be executed. In case of failure, total number of
/// requests sent is expected to be at least `settings.retries + 1` (initial request + `retries` configured in settings).
/// The actual number of requests sent can be higher, as the lower layers can retry the request multiple times.
///
/// `future_factory_fn` should be a `FnMut()` closure that returns a future that should be retried.
/// It takes [`RequestSettings`] as an argument and returns [`ExecutionResult`].
/// Retry mechanism can change [`RequestSettings`] between invocations of the `future_factory_fn` closure
/// to limit the number of retries for lower layers.
///
/// ## Parameters
///
/// - `address_list` - list of addresses to be used for the requests.
/// - `settings` - global settings with any request-specific settings overrides applied.
/// - `future_factory_fn` - closure that returns a future that should be retried. It should take [`RequestSettings`] as
///   an argument and return [`ExecutionResult`].
///
/// ## Returns
///
/// Returns future that resolves to [`ExecutionResult`].
///
/// ## Example
///
/// ```rust
/// # use dash_sdk::RequestSettings;
/// # use dash_sdk::error::{Error,StaleNodeError};
/// # use rs_dapi_client::{ExecutionResult, ExecutionError};
/// async fn retry_test_function(settings: RequestSettings) -> ExecutionResult<(), dash_sdk::Error> {
/// // do something
///     Err(ExecutionError {
///         inner: Error::StaleNode(StaleNodeError::Height{
///             expected_height: 10,
///             received_height: 3,
///             tolerance_blocks: 1,
///         }),
///        retries: 0,
///       address: None,
///    })
/// }
/// #[tokio::main]
///     async fn main() {
///     let address_list = rs_dapi_client::AddressList::default();
///     let global_settings = RequestSettings::default();
///     dash_sdk::sync::retry(&address_list, global_settings, retry_test_function).await.expect_err("should fail");
/// }
/// ```
///
/// ## Troubleshooting
///
/// Compiler error: `no method named retry found for closure`:
/// - ensure returned value is [`ExecutionResult`].,
/// - consider adding `.await` at the end of the closure.
///
///
/// ## See also
///
/// - [`::backon`] crate that is used by this function.
pub async fn retry<Fut, FutureFactoryFn, R, E>(
    address_list: &AddressList,
    settings: RequestSettings,
    future_factory_fn: FutureFactoryFn,
) -> ExecutionResult<R, E>
where
    Fut: Future<Output = ExecutionResult<R, E>>,
    FutureFactoryFn: FnMut(RequestSettings) -> Fut,
    E: CanRetry + Display + Debug,
{
    let max_retries = settings.retries.unwrap_or_default();

    let backoff_strategy = backon::ConstantBuilder::default()
        .with_delay(std::time::Duration::from_millis(10)) // we use different server, so no real delay needed, just to avoid spamming
        .with_max_times(max_retries);

    let mut retries: usize = 0;

    // Settings must be modified inside `when()` closure, so we need to use `ArcSwap` to allow mutable access to settings.
    let settings = ArcSwap::new(Arc::new(settings));

    // Closure below needs to be FnMut, so we need mutable future_factory_fn. In order to achieve that,
    // we use Arc<Mutex<.>>> pattern, to NOT move `future_factory_fn` directly into closure (as this breaks FnMut),
    // while still allowing mutable access to it.
    let inner_fn = Arc::new(Mutex::new(future_factory_fn));

    let closure_settings = &settings;
    // backon also support [backon::RetryableWithContext], but it doesn't pass the context to `when()` call.
    // As we need to modify the settings inside `when()`, context doesn't solve our problem and we have to implement
    // our own "context-like" logic using the closure below and `ArcSwap` for settings.
    let closure = move || {
        let inner_fn = inner_fn.clone();
        async move {
            let settings = closure_settings.load_full().clone();
            // Extract the future before executing it to release the lock
            tracing::trace!("retry: acquiring lock on future factory function");
            let fut = {
                let mut func = inner_fn.lock().await;
                tracing::trace!("retry: lock acquired, extracting future");
                (*func)(*settings)
            }; // Lock released here
            tracing::trace!("retry: lock released, executing future");
            
            let result = fut.await;

            // Ban or unban the address based on the result
            update_address_ban_status(address_list, &result, &settings.finalize());

            result
        }
    };

    let result = ::backon::Retryable::retry(closure, backoff_strategy)
        .when(|e| {
            if e.can_retry() {
                // requests sent for current execution attempt;
                let requests_sent = e.retries + 1;

                // requests sent in all preceeding attempts; user expects `settings.retries +1`
                retries += requests_sent;
                let all_requests_sent = retries;

                if all_requests_sent <= max_retries { // we account for initial request
                    tracing::warn!(retry = all_requests_sent, max_retries, error=?e, "retrying request");
                    let new_settings = RequestSettings {
                        retries: Some(max_retries - all_requests_sent), // limit num of retries for lower layer
                        ..**settings.load()
                    };
                    settings.store(Arc::new(new_settings));
                    true
                } else {
                    tracing::error!(retry = all_requests_sent, max_retries, error=?e, "no more retries left, giving up");
                    false
                }
            } else {
                false
            }
        })
        .sleep(rs_dapi_client::transport::BackonSleeper::default())
        .notify(|error, duration| {
            tracing::warn!(?duration, ?error, "request failed, retrying");
        })
        .await;

    result.map_err(|mut e| {
        e.retries = retries;
        e
    })
}

#[cfg(test)]
mod test {
    use super::*;
    use derive_more::Display;
    use rs_dapi_client::ExecutionError;
    use std::{
        future::Future,
        sync::atomic::{AtomicUsize, Ordering},
    };
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

    #[derive(Debug, Display)]
    enum MockError {
        Generic,
    }
    impl CanRetry for MockError {
        fn can_retry(&self) -> bool {
            true
        }
    }

    async fn retry_test_function(
        settings: RequestSettings,
        counter: Arc<AtomicUsize>,
    ) -> ExecutionResult<(), MockError> {
        // num or retries increases with each call
        let retries = counter.load(Ordering::Relaxed);
        let retries = if settings.retries.unwrap_or_default() < retries {
            settings.retries.unwrap_or_default()
        } else {
            retries
        };

        // we sent 1 initial request plus `retries` retries
        counter.fetch_add(1 + retries, Ordering::Relaxed);

        Err(ExecutionError {
            inner: MockError::Generic,
            retries,
            address: Some("http://localhost".parse().expect("valid address")),
        })
    }

    #[test_case::test_matrix([1,2,3,5,7,8,10,11,23,49, usize::MAX])]
    #[tokio::test]
    async fn test_retry(expected_requests: usize) {
        for _ in 0..1 {
            let counter = Arc::new(AtomicUsize::new(0));

            let address_list = AddressList::default();

            // we retry 5 times, and expect 5 retries + 1 initial request
            let mut global_settings = RequestSettings::default();
            global_settings.retries = Some(expected_requests - 1);

            let closure = |s| {
                let counter = counter.clone();
                retry_test_function(s, counter)
            };

            retry(&address_list, global_settings, closure)
                .await
                .expect_err("should fail");

            assert_eq!(
                counter.load(Ordering::Relaxed),
                expected_requests,
                "test failed for expected {} requests",
                expected_requests
            );
        }
    }
}
