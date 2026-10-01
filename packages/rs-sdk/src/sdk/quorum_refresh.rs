//! Recovery from a context provider whose quorum keys fell behind the network.
//!
//! A context provider that caches quorum public keys goes stale once Platform
//! starts signing with a quorum that formed after the cache was filled. From
//! then on every proof fails with [`ContextProviderError::InvalidQuorum`]. The
//! client is at fault, not the node, but a proof error is retryable, so the
//! request moves from node to node and bans each one until no address is left
//! and every request fails with "no available addresses".
//!
//! A provider that cannot fetch a missing key from inside the synchronous
//! [`ContextProvider::get_quorum_public_key`](dash_context_provider::ContextProvider::get_quorum_public_key)
//! (the wasm trusted context) registers a [`QuorumRefreshFn`] instead. When a
//! proof names a quorum the provider does not know, the SDK refreshes the keys
//! and asks again. A key that is still missing after that fails the request
//! without banning the node.

use super::Sdk;
use crate::Error;
use dash_context_provider::ContextProviderError;
use futures::future::{BoxFuture, FutureExt, Shared};
use rs_dapi_client::{ExecutionError, ExecutionResult};
use std::future::Future;
use std::panic::AssertUnwindSafe;
use std::sync::{Arc, Mutex, PoisonError};

/// Refreshes the quorum keys held by the SDK's context provider. See
/// [`SdkBuilder::with_quorum_refresher`](super::SdkBuilder::with_quorum_refresher).
pub type QuorumRefreshFn = Arc<dyn Fn() -> BoxFuture<'static, Result<(), String>> + Send + Sync>;

type RefreshFuture = Shared<BoxFuture<'static, Result<(), String>>>;

/// A [`QuorumRefreshFn`] that runs at most one refresh at a time.
pub(crate) struct QuorumRefresher {
    refresh: QuorumRefreshFn,
    /// The latest refresh. A request that misses a key while it is still
    /// running waits for it instead of starting another, so a burst of
    /// requests that all meet the new quorum costs a single refresh.
    latest: Mutex<Option<RefreshFuture>>,
}

impl QuorumRefresher {
    pub(crate) fn new(refresh: QuorumRefreshFn) -> Self {
        Self {
            refresh,
            latest: Mutex::new(None),
        }
    }

    async fn refresh(&self) -> Result<(), String> {
        let refresh = {
            let mut latest = self.latest.lock().unwrap_or_else(PoisonError::into_inner);
            match latest.as_ref() {
                Some(running) if running.peek().is_none() => running.clone(),
                _ => {
                    // `Shared` re-raises a panic on every later poll and `peek`,
                    // so a hook that panicked once would break every later miss.
                    let started = AssertUnwindSafe((self.refresh)())
                        .catch_unwind()
                        .map(|outcome| {
                            outcome.unwrap_or_else(|_| Err("quorum refresh panicked".to_string()))
                        })
                        .boxed()
                        .shared();
                    *latest = Some(started.clone());
                    started
                }
            }
        };
        refresh.await
    }
}

/// Whether `error` is a proof the context provider could not check because it
/// does not know the quorum that signed it.
fn is_missing_quorum_key(error: &Error) -> bool {
    matches!(
        error,
        Error::Proof(drive_proof_verifier::Error::ContextProviderError(
            ContextProviderError::InvalidQuorum(_)
        ))
    )
}

impl Sdk {
    /// Run `attempt`, and when its proof names a quorum the context provider
    /// does not know, refresh the quorum keys and run it once more.
    ///
    /// Without a registered [`QuorumRefreshFn`] this is just `attempt()`.
    pub(crate) async fn with_quorum_refresh<T, F, Fut>(
        &self,
        attempt: F,
    ) -> ExecutionResult<T, Error>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = ExecutionResult<T, Error>>,
    {
        let Some(refresher) = self.quorum_refresher.as_deref() else {
            return attempt().await;
        };
        let first = match attempt().await {
            Err(error) if is_missing_quorum_key(&error.inner) => error,
            result => return result,
        };

        // Ask again even when the refresh reports an error: it fetches the
        // current and previous quorum lists separately, and the one that did
        // arrive may hold the missing key.
        let refreshed = refresher.refresh().await;
        let earlier_requests = first.retries + 1;
        match attempt().await {
            // Every node answers with a proof from the same quorum, so moving on
            // to the next one cannot help and would ban a healthy node for the
            // client's missing key. Fail this request without retrying; the next
            // miss refreshes again.
            Err(error) if is_missing_quorum_key(&error.inner) => {
                let reason = match refreshed {
                    Ok(()) => "the refreshed quorum list does not have it yet".to_string(),
                    Err(refresh_error) => format!("refreshing quorum keys failed: {refresh_error}"),
                };
                Err(ExecutionError {
                    inner: Error::ContextProviderError(ContextProviderError::InvalidQuorum(
                        format!("{}; {reason}", error.inner),
                    )),
                    address: error.address,
                    retries: error.retries + earlier_requests,
                })
            }
            Err(mut error) => {
                error.retries += earlier_requests;
                Err(error)
            }
            Ok(mut response) => {
                response.retries += earlier_requests;
                Ok(response)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SdkBuilder;
    use rs_dapi_client::{CanRetry, ExecutionResponse};
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn missing_quorum() -> Error {
        Error::Proof(drive_proof_verifier::Error::ContextProviderError(
            ContextProviderError::InvalidQuorum("Quorum not found in cache".to_string()),
        ))
    }

    fn failed(inner: Error) -> ExecutionResult<u32, Error> {
        Err(ExecutionError {
            inner,
            address: None,
            retries: 0,
        })
    }

    fn succeeded(value: u32) -> ExecutionResult<u32, Error> {
        Ok(ExecutionResponse {
            inner: value,
            address: "http://127.0.0.1:1".parse().expect("address"),
            retries: 0,
        })
    }

    /// An SDK whose refresh hook counts its calls and returns `outcome`.
    fn sdk_with_refresher(calls: Arc<AtomicUsize>, outcome: Result<(), String>) -> Sdk {
        let refresher: QuorumRefreshFn = Arc::new(move || {
            calls.fetch_add(1, Ordering::SeqCst);
            let outcome = outcome.clone();
            async move { outcome }.boxed()
        });
        SdkBuilder::new_mock()
            .with_quorum_refresher(refresher)
            .build()
            .expect("mock sdk")
    }

    /// Runs `attempt` and reports how many times it was called.
    async fn run(
        sdk: &Sdk,
        attempt: impl Fn(usize) -> ExecutionResult<u32, Error>,
    ) -> (ExecutionResult<u32, Error>, usize) {
        let attempts = AtomicUsize::new(0);
        let result = sdk
            .with_quorum_refresh(|| {
                let n = attempts.fetch_add(1, Ordering::SeqCst);
                let result = attempt(n);
                async move { result }
            })
            .await;
        (result, attempts.load(Ordering::SeqCst))
    }

    #[tokio::test]
    async fn missing_quorum_refreshes_and_retries_once() {
        let refreshes = Arc::new(AtomicUsize::new(0));
        let sdk = sdk_with_refresher(Arc::clone(&refreshes), Ok(()));

        let (result, attempts) = run(&sdk, |n| {
            if n == 0 {
                failed(missing_quorum())
            } else {
                succeeded(7)
            }
        })
        .await;

        let response = result.expect("second attempt succeeds");
        assert_eq!(response.inner, 7);
        assert_eq!(response.retries, 1, "the first request counts as a retry");
        assert_eq!(attempts, 2);
        assert_eq!(refreshes.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn quorum_still_missing_after_refresh_fails_without_ban() {
        let refreshes = Arc::new(AtomicUsize::new(0));
        let sdk = sdk_with_refresher(Arc::clone(&refreshes), Ok(()));

        let (result, attempts) = run(&sdk, |_| failed(missing_quorum())).await;

        let error = result.expect_err("quorum is still unknown");
        assert!(
            !error.can_retry(),
            "every node signs with the same quorum; retrying would ban healthy nodes"
        );
        assert!(error.inner.to_string().contains("does not have it yet"));
        assert_eq!(attempts, 2);
        assert_eq!(refreshes.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn failed_refresh_fails_without_ban() {
        let refreshes = Arc::new(AtomicUsize::new(0));
        let sdk = sdk_with_refresher(
            Arc::clone(&refreshes),
            Err("quorum service down".to_string()),
        );

        let (result, attempts) = run(&sdk, |_| failed(missing_quorum())).await;

        let error = result.expect_err("refresh failed");
        assert!(
            !error.can_retry(),
            "retrying would ban a node for the client's stale keys"
        );
        assert!(error.inner.to_string().contains("quorum service down"));
        assert_eq!(attempts, 2);
        assert_eq!(refreshes.load(Ordering::SeqCst), 1);
    }

    /// The refresh fetches two lists; one failing does not mean the missing key
    /// did not arrive with the other.
    #[tokio::test]
    async fn failed_refresh_still_asks_again() {
        let refreshes = Arc::new(AtomicUsize::new(0));
        let sdk = sdk_with_refresher(
            Arc::clone(&refreshes),
            Err("previous quorums timed out".to_string()),
        );

        let (result, attempts) = run(&sdk, |n| {
            if n == 0 {
                failed(missing_quorum())
            } else {
                succeeded(7)
            }
        })
        .await;

        assert_eq!(result.expect("second attempt succeeds").inner, 7);
        assert_eq!(attempts, 2);
    }

    #[tokio::test]
    async fn panicking_refresh_does_not_break_later_ones() {
        let calls = Arc::new(AtomicUsize::new(0));
        let counted = Arc::clone(&calls);
        let refresher = QuorumRefresher::new(Arc::new(move || {
            let first = counted.fetch_add(1, Ordering::SeqCst) == 0;
            async move {
                if first {
                    panic!("refresh hook bug");
                }
                Ok(())
            }
            .boxed()
        }));

        assert_eq!(
            refresher.refresh().await,
            Err("quorum refresh panicked".to_string())
        );
        assert_eq!(refresher.refresh().await, Ok(()));
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn other_errors_do_not_refresh() {
        let refreshes = Arc::new(AtomicUsize::new(0));
        let sdk = sdk_with_refresher(Arc::clone(&refreshes), Ok(()));

        let (result, attempts) = run(&sdk, |_| {
            failed(Error::Proof(drive_proof_verifier::Error::EmptyVersion))
        })
        .await;

        assert!(result.is_err());
        assert_eq!(attempts, 1);
        assert_eq!(refreshes.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn without_refresher_missing_quorum_is_returned_as_is() {
        let sdk = SdkBuilder::new_mock().build().expect("mock sdk");

        let (result, attempts) = run(&sdk, |_| failed(missing_quorum())).await;

        assert!(is_missing_quorum_key(
            &result.expect_err("no refresher").inner
        ));
        assert_eq!(attempts, 1);
    }

    #[tokio::test]
    async fn concurrent_misses_share_one_refresh() {
        let calls = Arc::new(AtomicUsize::new(0));
        let (release, released) = futures::channel::oneshot::channel::<()>();
        let released = released.shared();
        let counted = Arc::clone(&calls);
        let refresher = QuorumRefresher::new(Arc::new(move || {
            counted.fetch_add(1, Ordering::SeqCst);
            let released = released.clone();
            async move {
                let _ = released.await;
                Ok(())
            }
            .boxed()
        }));

        let waiters = futures::future::join_all((0..8).map(|_| refresher.refresh()));
        let releaser = async {
            tokio::task::yield_now().await;
            release.send(()).expect("receiver alive");
        };
        let (results, ()) = futures::join!(waiters, releaser);

        assert!(results.iter().all(Result::is_ok));
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        // Once that refresh has finished, the next miss starts a new one.
        refresher.refresh().await.expect("second refresh");
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }
}
