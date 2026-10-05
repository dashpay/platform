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
//! and asks again. A key that is still missing after that is not held against
//! the node either: the quorum may be newer than the refreshed list, or the
//! endpoint may have named one that does not exist (the quorum hash is looked
//! up before the signature is checked). [`crate::sync::retry`] moves such a
//! request on to endpoints that have not answered it that way, within the
//! retry budget, and never bans one for it.

use super::Sdk;
use crate::Error;
use dash_context_provider::ContextProviderError;
use futures::future::{BoxFuture, FutureExt, Shared};
use rs_dapi_client::{ExecutionError, ExecutionResult};
use std::future::Future;
use std::panic::AssertUnwindSafe;
use std::sync::{Arc, Mutex, PoisonError};
use web_time::{Duration, Instant};

/// Refreshes the quorum keys held by the SDK's context provider. See
/// [`SdkBuilder::with_quorum_refresher`](super::SdkBuilder::with_quorum_refresher).
pub type QuorumRefreshFn = Arc<dyn Fn() -> BoxFuture<'static, Result<(), String>> + Send + Sync>;

type RefreshFuture = Shared<BoxFuture<'static, Result<(), String>>>;

/// How long a refresh answers every miss after it starts. Quorums rotate far
/// less often than this, and a node is never banned for naming an unknown
/// quorum, so without the window a node that keeps doing so would cost a
/// refresh on every request it serves.
///
/// Measured on a monotonic clock, so a wall-clock change can neither stretch
/// the reuse of a stale refresh nor cut the window short.
const REFRESH_COOLDOWN: Duration = Duration::from_secs(10);

/// A [`QuorumRefreshFn`] that runs at most one refresh at a time and at most
/// one per [`REFRESH_COOLDOWN`].
pub(crate) struct QuorumRefresher {
    refresh: QuorumRefreshFn,
    /// The latest refresh and when it started. A miss while it is running, or
    /// within the cooldown after it started, gets its result instead of
    /// starting another, so a burst of requests that all meet the new quorum
    /// costs a single refresh.
    latest: Mutex<Option<(RefreshFuture, Instant)>>,
}

impl QuorumRefresher {
    pub(crate) fn new(refresh: QuorumRefreshFn) -> Self {
        Self {
            refresh,
            latest: Mutex::new(None),
        }
    }

    async fn refresh(&self) -> Result<(), String> {
        // The guard is released before the refresh is awaited, and the hook
        // only runs when the refresh is first polled (see `start`), so the hook
        // never runs while the lock is held.
        let refresh = {
            let mut latest = self.latest.lock().unwrap_or_else(PoisonError::into_inner);
            let now = Instant::now();
            match latest.as_ref() {
                Some((refresh, started))
                    if refresh.peek().is_none()
                        || now.duration_since(*started) < REFRESH_COOLDOWN =>
                {
                    refresh.clone()
                }
                _ => {
                    let started = self.start();
                    *latest = Some((started.clone(), now));
                    started
                }
            }
        };
        refresh.await
    }

    /// A refresh that calls the hook when it is first polled.
    ///
    /// The returned future is published under the lock but polled after it is
    /// released, so a hook that blocks, or re-enters the refresher, cannot
    /// stall or deadlock other callers. Only the published future ever calls
    /// the hook: the wasm hook starts its fetch while building its future, so a
    /// hook future built and then discarded would still refresh.
    ///
    /// `Shared` re-raises a panic on every later poll and `peek`, so a hook
    /// that panicked once would break every later miss. The hook can panic
    /// while building its future as well as while it runs; both become an
    /// `Err`.
    fn start(&self) -> RefreshFuture {
        let hook = Arc::clone(&self.refresh);
        async move {
            match std::panic::catch_unwind(AssertUnwindSafe(|| hook())) {
                Ok(refresh) => AssertUnwindSafe(refresh)
                    .catch_unwind()
                    .await
                    .unwrap_or_else(|_| Err(refresh_panicked())),
                Err(_) => Err(refresh_panicked()),
            }
        }
        .boxed()
        .shared()
    }
}

fn refresh_panicked() -> String {
    "quorum refresh panicked".to_string()
}

/// Starts the message of the error a request fails with when the quorum its
/// proof names is still unknown after a refresh. See [`is_unresolved_quorum`].
const UNRESOLVED_QUORUM: &str = "quorum still unknown after refreshing quorum keys";

/// Whether `error` is the one [`Sdk::with_quorum_refresh`] returns when the
/// quorum a proof names is still unknown after a refresh.
///
/// It is not retryable, so `update_address_ban_status` never bans the node for
/// it, but [`crate::sync::retry`] still moves the request on to another
/// endpoint: one endpoint naming an unknown quorum says nothing about the rest.
pub(crate) fn is_unresolved_quorum(error: &Error) -> bool {
    matches!(
        error,
        Error::ContextProviderError(ContextProviderError::InvalidQuorum(message))
            if message.starts_with(UNRESOLVED_QUORUM)
    )
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
            // Either the quorum is newer than the refreshed list, or this
            // endpoint named one that does not exist. Neither shows the node is
            // unhealthy, so the error is not retryable and never bans it;
            // `sync::retry` tries other endpoints instead (`is_unresolved_quorum`).
            Err(error) if is_missing_quorum_key(&error.inner) => {
                let reason = match refreshed {
                    Ok(()) => "the refreshed quorum list does not have it yet".to_string(),
                    Err(refresh_error) => format!("refreshing quorum keys failed: {refresh_error}"),
                };
                Err(ExecutionError {
                    inner: Error::ContextProviderError(ContextProviderError::InvalidQuorum(
                        format!("{UNRESOLVED_QUORUM}: {}; {reason}", error.inner),
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

    impl QuorumRefresher {
        /// Let the next miss start a new refresh, as if the cooldown had passed.
        fn expire_cooldown(&self) {
            if let Some((_, started)) = self.latest.lock().unwrap().as_mut() {
                *started -= REFRESH_COOLDOWN;
            }
        }
    }

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
            "a retryable error would ban the node for the client's missing key"
        );
        assert!(is_unresolved_quorum(&error.inner));
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
        refresher.expire_cooldown();
        assert_eq!(refresher.refresh().await, Ok(()));
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn hook_panicking_before_its_future_does_not_escape() {
        let calls = Arc::new(AtomicUsize::new(0));
        let counted = Arc::clone(&calls);
        let refresher = QuorumRefresher::new(Arc::new(move || {
            if counted.fetch_add(1, Ordering::SeqCst) == 0 {
                panic!("refresh hook bug");
            }
            async { Ok(()) }.boxed()
        }));

        assert_eq!(
            refresher.refresh().await,
            Err("quorum refresh panicked".to_string())
        );
        refresher.expire_cooldown();
        assert_eq!(refresher.refresh().await, Ok(()));
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    /// The hook runs only once the shared refresh is polled, after the lock is
    /// released, so it can block or re-enter the refresher without stalling
    /// other callers.
    #[tokio::test]
    async fn hook_runs_with_the_lock_released() {
        let lock_was_free = Arc::new(Mutex::new(None));
        let observed = Arc::clone(&lock_was_free);
        let refresher = Arc::new_cyclic(|this: &std::sync::Weak<QuorumRefresher>| {
            let this = this.clone();
            QuorumRefresher::new(Arc::new(move || {
                let refresher = this.upgrade().expect("refresher alive");
                *observed.lock().unwrap() = Some(refresher.latest.try_lock().is_ok());
                async { Ok(()) }.boxed()
            }))
        });

        refresher.refresh().await.expect("refresh");

        assert_eq!(*lock_was_free.lock().unwrap(), Some(true));
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
        // `calls` counts hook calls, the point where the wasm hook already
        // starts its fetch; `runs` counts refreshes that actually ran.
        let calls = Arc::new(AtomicUsize::new(0));
        let runs = Arc::new(AtomicUsize::new(0));
        let (release, released) = futures::channel::oneshot::channel::<()>();
        let released = released.shared();
        let counted = Arc::clone(&calls);
        let ran = Arc::clone(&runs);
        let refresher = QuorumRefresher::new(Arc::new(move || {
            counted.fetch_add(1, Ordering::SeqCst);
            let released = released.clone();
            let ran = Arc::clone(&ran);
            async move {
                ran.fetch_add(1, Ordering::SeqCst);
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
        assert_eq!(calls.load(Ordering::SeqCst), 1, "the hook must start once");
        assert_eq!(runs.load(Ordering::SeqCst), 1);

        // Within the cooldown a finished refresh answers later misses too.
        refresher.refresh().await.expect("cached refresh");
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        // After it, the next miss starts a new one.
        refresher.expire_cooldown();
        refresher.refresh().await.expect("second refresh");
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    fn failed_at(inner: Error, address: &rs_dapi_client::Address) -> ExecutionResult<u32, Error> {
        Err(ExecutionError {
            inner,
            address: Some(address.clone()),
            retries: 0,
        })
    }

    fn address(port: u16) -> rs_dapi_client::Address {
        format!("http://127.0.0.1:{port}").parse().expect("address")
    }

    fn assert_nobody_banned(list: &rs_dapi_client::AddressList) {
        for info in list.ban_info() {
            assert!(!info.banned, "{} must not be banned", info.uri);
            assert_eq!(info.ban_count, 0, "{} must not be banned", info.uri);
        }
        assert_eq!(list.get_live_addresses().len(), list.len());
    }

    /// Request through the SDK's retry loop, with the request served by
    /// whichever endpoint the address list selects. Endpoints in `fabricating`
    /// answer with a proof naming a quorum nobody knows; the rest succeed.
    /// Returns the result and the number of requests sent.
    async fn request(
        sdk: &Sdk,
        list: &rs_dapi_client::AddressList,
        fabricating: &[rs_dapi_client::Address],
    ) -> (ExecutionResult<u32, Error>, usize) {
        let requests = AtomicUsize::new(0);
        let settings = rs_dapi_client::RequestSettings {
            retries: Some(5),
            ..Default::default()
        };
        let result = crate::sync::retry(list, settings, |_| {
            sdk.with_quorum_refresh(|| {
                requests.fetch_add(1, Ordering::SeqCst);
                let served = list.get_live_address().expect("a live address");
                let result = if fabricating.contains(&served) {
                    failed_at(missing_quorum(), &served)
                } else {
                    Ok(ExecutionResponse {
                        inner: 7,
                        address: served,
                        retries: 0,
                    })
                };
                async move { result }
            })
        })
        .await;
        (result, requests.load(Ordering::SeqCst))
    }

    /// The quorum hash is read before the signature is checked, so any
    /// endpoint can name a quorum that does not exist. Endpoints doing that
    /// must not keep the request from a healthy one, even when only one
    /// endpoint at a time receives traffic, and nobody is banned for it.
    #[tokio::test]
    async fn unknown_quorums_from_some_endpoints_fail_over_to_a_healthy_one() {
        let refreshes = Arc::new(AtomicUsize::new(0));
        let sdk = sdk_with_refresher(Arc::clone(&refreshes), Ok(()));
        let fabricating = [address(1), address(2)];
        let healthy = address(3);

        // Selection is random; repeat so every serving order is covered.
        for _ in 0..20 {
            let mut list = rs_dapi_client::AddressList::new().with_active_set_size(1);
            for endpoint in fabricating.iter().chain([&healthy]) {
                list.add(endpoint.clone());
            }

            let (result, _) = request(&sdk, &list, &fabricating).await;

            let response = result.expect("the healthy endpoint answers");
            assert_eq!(response.address, healthy);
            assert_nobody_banned(&list);
        }
        assert_eq!(
            refreshes.load(Ordering::SeqCst),
            1,
            "later misses reuse the refresh within the cooldown"
        );
    }

    /// When every endpoint names a quorum the refreshed keys lack, as after a
    /// rotation the quorum service has not caught up with, the request stops
    /// within its retry budget, once no endpoint is left to ask, and bans none.
    #[tokio::test]
    async fn unknown_quorum_from_every_endpoint_fails_within_budget_without_bans() {
        let sdk = sdk_with_refresher(Arc::new(AtomicUsize::new(0)), Ok(()));
        let endpoints = [address(1), address(2), address(3)];
        let mut list = rs_dapi_client::AddressList::new();
        for endpoint in &endpoints {
            list.add(endpoint.clone());
        }

        let (result, requests) = request(&sdk, &list, &endpoints).await;

        let error = result.expect_err("no endpoint has a known quorum");
        assert!(is_unresolved_quorum(&error.inner));
        assert!(requests <= 6, "{requests} requests exceed retries + 1");
        assert_nobody_banned(&list);

        // A single endpoint is asked only once around the refresh.
        let mut single = rs_dapi_client::AddressList::new();
        single.add(address(4));
        let (result, requests) = request(&sdk, &single, &[address(4)]).await;
        assert!(is_unresolved_quorum(
            &result.expect_err("unknown quorum").inner
        ));
        assert_eq!(requests, 2);
        assert_nobody_banned(&single);
    }
}
