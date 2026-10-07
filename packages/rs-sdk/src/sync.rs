pub use dash_async::{block_on, AsyncError};

use crate::error::Error;
use dash_context_provider::ContextProviderError;
use rs_dapi_client::{
    transport::sleep, update_address_ban_status, AddressList, CanRetry, ExecutionResult,
    RequestSettings,
};
use std::future::Future;
use std::time::Duration;

impl From<AsyncError> for crate::Error {
    fn from(error: AsyncError) -> Self {
        Self::ContextProviderError(error.into())
    }
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
/// - ensure returned value is [`ExecutionResult`].
/// - consider adding `.await` at the end of the closure.
pub async fn retry<Fut, FutureFactoryFn, R>(
    address_list: &AddressList,
    settings: RequestSettings,
    future_factory_fn: FutureFactoryFn,
) -> ExecutionResult<R, Error>
where
    Fut: Future<Output = ExecutionResult<R, Error>>,
    FutureFactoryFn: FnMut(RequestSettings) -> Fut,
    R: Send,
{
    retry_with_additional_error(
        address_list,
        settings,
        future_factory_fn,
        is_quorum_source_unavailable,
    )
    .await
}

/// Whether the response could not be checked because the context provider's
/// trusted quorum source gave no answer about the quorum that signed it. That
/// says nothing about the node, so it is not banned through the health ladder;
/// it is stepped over briefly, because another node may be signed by a quorum
/// the provider already holds.
fn is_quorum_source_unavailable(error: &Error) -> bool {
    matches!(
        error,
        Error::ContextProviderError(ContextProviderError::QuorumSourceUnavailable(_))
    )
}

/// Retry a failure that is not retryable on its own, such as an
/// operation-specific rejection or a response the client could not check,
/// only when its responding node can be excluded. This does not change the
/// error's global retry classification. Callers must restrict the predicate to
/// failures that are safe to repeat on another node.
pub(crate) async fn retry_with_additional_error<Fut, FutureFactoryFn, R, AdditionalError>(
    address_list: &AddressList,
    settings: RequestSettings,
    mut future_factory_fn: FutureFactoryFn,
    additional_error: AdditionalError,
) -> ExecutionResult<R, Error>
where
    Fut: Future<Output = ExecutionResult<R, Error>>,
    FutureFactoryFn: FnMut(RequestSettings) -> Fut,
    R: Send,
    AdditionalError: Fn(&Error) -> bool,
{
    let max_retries = settings.retries.unwrap_or_default();
    let mut total_retries: usize = 0;
    let mut current_settings = settings;

    // Store the last meaningful error (not "no available addresses")
    // so we can return it if we exhaust all addresses
    let mut last_meaningful_error: Option<rs_dapi_client::ExecutionError<Error>> = None;

    loop {
        let result = future_factory_fn(current_settings).await;

        // Ban or unban the address based on the result
        update_address_ban_status(address_list, &result, &current_settings.finalize());

        match result {
            Ok(response) => return Ok(response),
            Err(error) => {
                // Check if this is a "no available addresses" error and we have a previous meaningful error
                if error.is_no_available_addresses() {
                    if let Some(prev_error) = last_meaningful_error.take() {
                        tracing::error!(
                            retry = total_retries,
                            max_retries,
                            error = ?prev_error,
                            "no addresses available to retry"
                        );
                        // Wrap the last meaningful error in NoAvailableAddresses
                        return Err(rs_dapi_client::ExecutionError {
                            inner: Error::NoAvailableAddressesToRetry(Box::new(prev_error.inner)),
                            retries: total_retries,
                            address: prev_error.address,
                        });
                    }
                    // No previous error, return the "no available addresses" error as-is
                    return Err(error);
                }

                // Count requests sent in this attempt
                let requests_sent = error.retries + 1;
                total_retries += requests_sent;

                let retry_additional_error = additional_error(&error.inner);
                if !error.can_retry() && !retry_additional_error {
                    // Non-retryable error, return immediately
                    let mut final_error = error;
                    final_error.retries = total_retries;
                    return Err(final_error);
                }

                if total_retries > max_retries {
                    // Exceeded max retries
                    tracing::error!(
                        retry = total_retries,
                        max_retries,
                        error = ?error,
                        "no more retries left, giving up"
                    );
                    let mut final_error = error;
                    final_error.retries = total_retries;
                    return Err(final_error);
                }

                if retry_additional_error {
                    // This failure does not establish a health failure. Use a
                    // short flat exclusion, never the exponential health ladder.
                    // The same node would answer the same way (a node can retain
                    // rejected transaction hashes), so never send the request to
                    // it again, including when the caller disabled banning or
                    // the address is unknown. Only exclude it when a retry to
                    // another node is possible; a single-node client must
                    // remain usable after failure.
                    let excluded = current_settings.finalize().ban_failed_address
                        && error.address.as_ref().is_some_and(|address| {
                            address_list
                                .get_live_addresses()
                                .iter()
                                .any(|candidate| candidate != address)
                                && address_list.ban_for(
                                    address,
                                    Duration::from_secs(2),
                                    Some(error.to_string()),
                                )
                        });
                    if !excluded || address_list.get_live_addresses().is_empty() {
                        tracing::debug!(node = ?error.address, error = %error.inner, "failover stopped: no safely excluded alternative");
                        let mut final_error = error;
                        final_error.retries = total_retries;
                        return Err(final_error);
                    }
                }

                // Log retry decision (matches original `when()` callback)
                tracing::warn!(
                    retry = total_retries,
                    max_retries,
                    error = ?error,
                    "retrying request"
                );

                // Update settings for next retry - limit retries for lower layer
                current_settings.retries = Some(max_retries.saturating_sub(total_retries));

                // Small delay to avoid spamming (we use different server, so no real delay needed)
                // Log before sleep (matches original `notify()` callback)
                let delay = Duration::from_millis(10);
                tracing::warn!(duration = ?delay, error = ?error, "request failed, retrying");

                // Store this as the last meaningful error before retrying
                last_meaningful_error = Some(error);

                sleep(delay).await;
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use rs_dapi_client::ExecutionError;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    use crate::error::StaleNodeError;
    use rs_dapi_client::DapiClientError;

    async fn retry_test_function(
        settings: RequestSettings,
        counter: Arc<AtomicUsize>,
    ) -> ExecutionResult<(), Error> {
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
            inner: Error::StaleNode(StaleNodeError::Height {
                expected_height: 100,
                received_height: 50,
                tolerance_blocks: 1,
            }),
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

    /// Test that when we get "no available addresses" error, we return the last meaningful error
    /// wrapped in NoAvailableAddresses.
    #[tokio::test]
    async fn test_retry_returns_last_meaningful_error_on_no_addresses() {
        let call_count = Arc::new(AtomicUsize::new(0));
        let address_list = AddressList::default();

        let mut settings = RequestSettings::default();
        settings.retries = Some(5);

        let call_count_clone = call_count.clone();
        let closure = move |_settings: RequestSettings| {
            let count = call_count_clone.fetch_add(1, Ordering::Relaxed);
            async move {
                if count == 0 {
                    Err(ExecutionError {
                        inner: Error::StaleNode(StaleNodeError::Height {
                            expected_height: 100,
                            received_height: 50,
                            tolerance_blocks: 1,
                        }),
                        retries: 0,
                        address: Some("http://localhost:1".parse().unwrap()),
                    })
                } else {
                    Err(ExecutionError {
                        inner: Error::DapiClientError(DapiClientError::NoAvailableAddresses),
                        retries: 0,
                        address: None,
                    })
                }
            }
        };

        let result: ExecutionResult<(), Error> = retry(&address_list, settings, closure).await;

        let error = result.expect_err("should fail");
        match &error.inner {
            Error::NoAvailableAddressesToRetry(inner) => {
                assert!(
                    matches!(**inner, Error::StaleNode(_)),
                    "inner error should be StaleNode, got: {:?}",
                    inner
                );
            }
            _ => panic!(
                "expected NoAvailableAddresses error, got: {:?}",
                error.inner
            ),
        }
        assert_eq!(
            call_count.load(Ordering::Relaxed),
            2,
            "should have called twice"
        );
    }

    fn quorum_source_unavailable(address: &rs_dapi_client::Address) -> ExecutionError<Error> {
        ExecutionError {
            inner: Error::ContextProviderError(
                dash_context_provider::ContextProviderError::QuorumSourceUnavailable(
                    "quorum service unreachable".to_string(),
                ),
            ),
            retries: 0,
            address: Some(address.clone()),
        }
    }

    fn two_nodes() -> (
        AddressList,
        rs_dapi_client::Address,
        rs_dapi_client::Address,
    ) {
        let first: rs_dapi_client::Address = "http://127.0.0.1:1".parse().expect("address");
        let second: rs_dapi_client::Address = "http://127.0.0.1:2".parse().expect("address");
        let mut list = AddressList::new();
        list.add(first.clone());
        list.add(second.clone());
        (list, first, second)
    }

    /// When the client's trusted quorum source gave no answer, the client
    /// could not check the response, which says nothing about the node. The
    /// request moves on, because another node may be signed by a quorum the
    /// client already holds, and the node is only stepped over briefly.
    #[tokio::test]
    async fn should_fail_over_when_the_quorum_source_gave_no_answer() {
        let (list, first, second) = two_nodes();
        let calls = Arc::new(AtomicUsize::new(0));
        let counted = Arc::clone(&calls);
        let served = second.clone();
        let failing = first.clone();
        let closure = move |_settings: RequestSettings| {
            let call = counted.fetch_add(1, Ordering::SeqCst);
            let served = served.clone();
            let failing = failing.clone();
            async move {
                if call == 0 {
                    Err(quorum_source_unavailable(&failing))
                } else {
                    Ok(rs_dapi_client::ExecutionResponse {
                        inner: (),
                        retries: 0,
                        address: served,
                    })
                }
            }
        };

        let settings = RequestSettings {
            retries: Some(3),
            ..RequestSettings::default()
        };

        let response = retry(&list, settings, closure)
            .await
            .expect("another node answers");

        assert_eq!(response.address, second);
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert!(
            !list.get_live_addresses().contains(&first),
            "the node that answered is stepped over for a moment"
        );
        let excluded_until = list
            .ban_info()
            .into_iter()
            .find(|info| info.uri == first.uri().to_string())
            .and_then(|info| info.banned_until)
            .expect("the node is excluded for a while");
        assert!(
            excluded_until <= chrono::Utc::now() + chrono::Duration::seconds(2),
            "a short flat exclusion, not the health ladder"
        );
    }

    /// A single-node client must stay usable, and a client that turned
    /// banning off must not have its nodes excluded: the request fails with
    /// the quorum source error instead of failing over.
    #[test_case::test_case(false, None; "single node")]
    #[test_case::test_case(true, Some(false); "banning off")]
    #[tokio::test]
    async fn should_not_fail_over_a_quorum_source_failure_without_an_excludable_node(
        two_nodes_listed: bool,
        ban_failed_address: Option<bool>,
    ) {
        let (list, first, _) = if two_nodes_listed {
            two_nodes()
        } else {
            let first: rs_dapi_client::Address = "http://127.0.0.1:1".parse().expect("address");
            let mut list = AddressList::new();
            list.add(first.clone());
            (list, first.clone(), first)
        };
        let calls = Arc::new(AtomicUsize::new(0));
        let counted = Arc::clone(&calls);
        let failing = first.clone();
        let closure = move |_settings: RequestSettings| {
            counted.fetch_add(1, Ordering::SeqCst);
            let failing = failing.clone();
            async move {
                Err::<rs_dapi_client::ExecutionResponse<()>, _>(quorum_source_unavailable(&failing))
            }
        };
        let settings = RequestSettings {
            retries: Some(3),
            ban_failed_address,
            ..RequestSettings::default()
        };

        let error = retry(&list, settings, closure)
            .await
            .expect_err("no node can be stepped over");

        assert!(matches!(
            error.inner,
            Error::ContextProviderError(
                dash_context_provider::ContextProviderError::QuorumSourceUnavailable(_)
            )
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert!(
            list.get_live_addresses().contains(&first),
            "no node is excluded"
        );
    }

    /// Test that if we get "no available addresses" on the first call (no previous error),
    /// we still return it as-is (not wrapped).
    #[tokio::test]
    async fn test_retry_returns_no_addresses_if_no_previous_error() {
        let address_list = AddressList::default();

        let mut settings = RequestSettings::default();
        settings.retries = Some(5);

        let closure = move |_settings: RequestSettings| async move {
            Err(ExecutionError {
                inner: Error::DapiClientError(DapiClientError::NoAvailableAddresses),
                retries: 0,
                address: None,
            })
        };

        let result: ExecutionResult<(), Error> = retry(&address_list, settings, closure).await;

        let error = result.expect_err("should fail");
        assert!(
            matches!(
                error.inner,
                Error::DapiClientError(DapiClientError::NoAvailableAddresses)
            ),
            "should return 'no available addresses' when there's no previous meaningful error, got: {:?}",
            error.inner
        );
    }
}
