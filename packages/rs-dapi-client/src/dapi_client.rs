//! [DapiClient] definition.

use dapi_grpc::mock::Mockable;
use dapi_grpc::tonic::async_trait;
#[cfg(not(target_arch = "wasm32"))]
use dapi_grpc::tonic::transport::Certificate;
use rand::{rngs::SmallRng, Rng, SeedableRng};
use std::fmt::{Debug, Display};
use std::time::Duration;
use tracing::Instrument;

use crate::address_list::AddressListError;
use crate::connection_pool::ConnectionPool;
use crate::request_settings::AppliedRequestSettings;
use crate::transport::{self, TransportError};
use crate::{
    transport::{TransportClient, TransportRequest},
    AddressList, CanRetry, DapiRequestExecutor, ExecutionError, ExecutionResponse, ExecutionResult,
    RequestSettings,
};

/// Base delay between retries. Genuine (non-rate-limited) failures keep this
/// flat delay — unchanged from the pre-existing behavior.
const RETRY_BASE_DELAY_MS: u64 = 10;

/// Upper bound for the rate-limit backoff window. Caps the capped-exponential
/// growth so a throttled fleet gets breathing room without stalling the call
/// for too long.
const RATE_LIMIT_MAX_DELAY_MS: u64 = 500;

/// Capped-exponential backoff window for a rate-limited retry: `base × 2^attempt`,
/// saturating at [`RATE_LIMIT_MAX_DELAY_MS`].
///
/// Pure (no jitter) so the growth/cap can be reasoned about and tested directly;
/// jitter is applied on top by [`retry_delay`]. `attempt` is the 0-based retry
/// index (0 for the first retry).
fn rate_limit_backoff_window(attempt: u32) -> Duration {
    // `base << attempt` == `base * 2^attempt`; `checked_shl` guards against a
    // huge `attempt` (e.g. a caller configuring an enormous retry budget).
    let window_ms = RETRY_BASE_DELAY_MS
        .checked_shl(attempt)
        .unwrap_or(u64::MAX)
        .min(RATE_LIMIT_MAX_DELAY_MS);
    Duration::from_millis(window_ms)
}

/// Delay to wait before the next retry attempt.
///
/// * Genuine failures keep the pre-existing flat [`RETRY_BASE_DELAY_MS`] delay.
/// * Rate-limit (`ResourceExhausted`) retries use **capped exponential backoff
///   with full jitter**: the window grows as `base × 2^attempt`, saturating at
///   [`RATE_LIMIT_MAX_DELAY_MS`], and the actual sleep is a uniform-random
///   fraction of that window. Jitter de-correlates clients so a congested fleet
///   is not retried in lockstep (a synchronized retry storm); the growth + cap
///   bound the per-call amplification.
///
/// `attempt` is the 0-based retry index (0 for the first retry).
//
// TODO(rate-limit): when the server attaches `google.rpc.RetryInfo` to the
// `ResourceExhausted` status, honor its `retry_delay` as the floor for this
// sleep. Extracting it needs decoding `grpc-status-details-bin`, so it is left
// as a follow-up rather than plumbed here.
fn retry_delay(rate_limited: bool, attempt: u32) -> Duration {
    if !rate_limited {
        return Duration::from_millis(RETRY_BASE_DELAY_MS);
    }

    let window = rate_limit_backoff_window(attempt);
    // Full jitter: sleep a uniform-random fraction of the window, i.e. a value
    // in `[0, window)`.
    let jitter: f64 = SmallRng::from_entropy().gen();
    window.mul_f64(jitter)
}

/// General DAPI request error type.
#[derive(Debug, thiserror::Error, Clone)]
#[cfg_attr(feature = "mocks", derive(serde::Serialize, serde::Deserialize))]
pub enum DapiClientError {
    /// The error happened on transport layer
    #[error("transport error: {0}")]
    Transport(
        #[cfg_attr(feature = "mocks", serde(with = "dapi_grpc::mock::serde_mockable"))]
        TransportError,
    ),
    /// There are no valid DAPI addresses to use.
    #[error("no available addresses to use")]
    NoAvailableAddresses,
    /// All available addresses have been exhausted (banned due to errors).
    /// Contains the last meaningful error that caused addresses to be banned.
    #[error("no available addresses to retry, last error: {0}")]
    NoAvailableAddressesToRetry(
        #[cfg_attr(feature = "mocks", serde(with = "dapi_grpc::mock::serde_mockable"))]
        Box<TransportError>,
    ),
    /// [AddressListError] errors
    #[error("address list error: {0}")]
    AddressList(AddressListError),

    #[cfg(feature = "mocks")]
    #[error("mock error: {0}")]
    /// Error happened in mock client
    Mock(#[from] crate::mock::MockError),
}

impl CanRetry for DapiClientError {
    fn can_retry(&self) -> bool {
        use DapiClientError::*;
        match self {
            NoAvailableAddresses => false,
            NoAvailableAddressesToRetry(_) => false,
            Transport(transport_error) => transport_error.can_retry(),
            AddressList(_) => false,
            #[cfg(feature = "mocks")]
            Mock(_) => false,
        }
    }

    fn is_no_available_addresses(&self) -> bool {
        matches!(
            self,
            DapiClientError::NoAvailableAddresses | DapiClientError::NoAvailableAddressesToRetry(_)
        )
    }

    fn is_rate_limited(&self) -> bool {
        use DapiClientError::*;
        match self {
            NoAvailableAddresses => false,
            NoAvailableAddressesToRetry(_) => false,
            Transport(transport_error) => transport_error.is_rate_limited(),
            AddressList(_) => false,
            #[cfg(feature = "mocks")]
            Mock(_) => false,
        }
    }
}

/// Serialization of [DapiClientError].
///
/// We need to do manual serialization because of the generic type parameter which doesn't support serde derive.
impl Mockable for DapiClientError {
    #[cfg(feature = "mocks")]
    fn mock_serialize(&self) -> Option<Vec<u8>> {
        Some(serde_json::to_vec(self).expect("serialize DAPI client error"))
    }

    #[cfg(feature = "mocks")]
    fn mock_deserialize(data: &[u8]) -> Option<Self> {
        Some(serde_json::from_slice(data).expect("deserialize DAPI client error"))
    }
}

/// Access point to DAPI.
#[derive(Debug, Clone)]
pub struct DapiClient {
    address_list: AddressList,
    settings: RequestSettings,
    pool: ConnectionPool,
    #[cfg(not(target_arch = "wasm32"))]
    /// Certificate Authority certificate to use for verifying the server's certificate.
    pub ca_certificate: Option<Certificate>,
    #[cfg(feature = "dump")]
    pub(crate) dump_dir: Option<std::path::PathBuf>,
}

impl DapiClient {
    /// Initialize new [DapiClient] and optionally override default settings.
    pub fn new(address_list: AddressList, settings: RequestSettings) -> Self {
        // multiply by 3 as we need to store core and platform addresses, and we want some spare capacity just in case
        let address_count = 3 * address_list.len();

        Self {
            address_list,
            settings,
            pool: ConnectionPool::new(address_count),
            #[cfg(feature = "dump")]
            dump_dir: None,
            #[cfg(not(target_arch = "wasm32"))]
            ca_certificate: None,
        }
    }

    /// Set CA certificate to use when verifying the server's certificate.
    ///
    /// # Arguments
    ///
    /// * `pem_ca_cert` - CA certificate in PEM format.
    ///
    /// # Returns
    /// [DapiClient] with CA certificate set.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn with_ca_certificate(mut self, ca_cert: Certificate) -> Self {
        self.ca_certificate = Some(ca_cert);

        self
    }

    /// Return the [DapiClient] address list.
    pub fn address_list(&self) -> &AddressList {
        &self.address_list
    }

    /// Get all non-banned addresses from the address list.
    ///
    /// Returns a vector of addresses that are not currently banned or whose ban period has expired.
    /// This is useful for diagnostics, monitoring, or when you need to know which DAPI nodes are
    /// currently available for making requests.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use rs_dapi_client::{DapiClient, AddressList, RequestSettings};
    ///
    /// let address_list = "http://127.0.0.1:3000,http://127.0.0.1:3001".parse().unwrap();
    /// let client = DapiClient::new(address_list, RequestSettings::default());
    ///
    /// // Get all currently available (non-banned) addresses
    /// let live_addresses = client.get_live_addresses();
    /// println!("Available DAPI nodes: {}", live_addresses.len());
    /// ```
    pub fn get_live_addresses(&self) -> Vec<crate::Address> {
        self.address_list.get_live_addresses()
    }
}

/// Ban address in case of retryable error or unban it
/// if it was banned, and the request was successful.
pub fn update_address_ban_status<R, E>(
    address_list: &AddressList,
    result: &ExecutionResult<R, E>,
    applied_settings: &AppliedRequestSettings,
) where
    E: CanRetry + Display + Debug,
{
    match &result {
        Ok(response) => {
            // Unban the address if it was banned and node responded successfully this time
            if address_list.is_banned(&response.address) {
                if address_list.unban(&response.address) {
                    tracing::debug!(address = ?response.address, "unban successfully responded address {}", response.address);
                } else {
                    // The address might be already removed from the list
                    // by background process (i.e., SML update), and it's fine.
                    tracing::debug!(
                        address = ?response.address,
                        "unable to unban address {} because it's not in the list anymore",
                        response.address
                    );
                }
            }
        }
        Err(error) => {
            if error.can_retry() {
                if error.is_rate_limited() {
                    // ResourceExhausted is congestion/backpressure, not endpoint
                    // ill-health. Banning a healthy-but-throttled node shifts its
                    // load onto the survivors and cascades into
                    // NoAvailableAddressesToRetry. Keep it in the pool; the retry
                    // loop rotates to a different node via explicit exclusion. The
                    // bounded retry count bounds an over-limit client.
                    tracing::debug!(
                        ?error,
                        address = ?error.address,
                        "not banning rate-limited (ResourceExhausted) address; will rotate to another node"
                    );
                } else if let Some(address) = error.address.as_ref() {
                    if applied_settings.ban_failed_address {
                        if address_list.ban_with_reason(address, Some(error.to_string())) {
                            tracing::warn!(
                                ?address,
                                ?error,
                                "ban address {address} due to error: {error}"
                            );
                        } else {
                            // The address might be already removed from the list
                            // by background process (i.e., SML update), and it's fine.
                            tracing::debug!(
                                ?address,
                                ?error,
                                "unable to ban address {address} because it's not in the list anymore"
                            );
                        }
                    } else {
                        tracing::debug!(
                            ?error,
                            ?address,
                            "we should ban the address {address} due to the error but banning is disabled"
                        );
                    }
                } else {
                    tracing::debug!(
                        ?error,
                        "we should ban an address due to the error but address is absent"
                    );
                }
            }
        }
    };
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::*;

    fn mock_address() -> crate::Address {
        "http://127.0.0.1:3000".parse().expect("valid address")
    }

    fn make_applied_settings(ban: bool) -> AppliedRequestSettings {
        AppliedRequestSettings {
            connect_timeout: None,
            timeout: Duration::from_secs(10),
            retries: 5,
            ban_failed_address: ban,
            max_decoding_message_size: None,
            #[cfg(not(target_arch = "wasm32"))]
            ca_certificate: None,
        }
    }

    #[test]
    fn test_can_retry_no_available_addresses() {
        let err = DapiClientError::NoAvailableAddresses;
        assert!(!err.can_retry());
    }

    #[test]
    fn test_can_retry_no_available_addresses_to_retry() {
        let transport_err = TransportError::Grpc(dapi_grpc::tonic::Status::unavailable("gone"));
        let err = DapiClientError::NoAvailableAddressesToRetry(Box::new(transport_err));
        assert!(!err.can_retry());
    }

    #[test]
    fn test_can_retry_transport_retryable() {
        let transport_err =
            TransportError::Grpc(dapi_grpc::tonic::Status::unavailable("temporary"));
        let err = DapiClientError::Transport(transport_err);
        assert!(err.can_retry());
    }

    #[test]
    fn test_can_retry_transport_non_retryable() {
        let transport_err = TransportError::Grpc(dapi_grpc::tonic::Status::not_found("permanent"));
        let err = DapiClientError::Transport(transport_err);
        assert!(!err.can_retry());
    }

    #[test]
    fn test_can_retry_address_list_error() {
        let err =
            DapiClientError::AddressList(AddressListError::InvalidAddressUri("bad".to_string()));
        assert!(!err.can_retry());
    }

    #[test]
    fn test_dapi_client_error_is_rate_limited() {
        // Transport ResourceExhausted → rate-limited (but still retryable).
        let rate_limited = DapiClientError::Transport(TransportError::Grpc(
            dapi_grpc::tonic::Status::resource_exhausted("429"),
        ));
        assert!(rate_limited.is_rate_limited());
        assert!(rate_limited.can_retry());

        // Other transport codes are not rate-limited.
        let unavailable = DapiClientError::Transport(TransportError::Grpc(
            dapi_grpc::tonic::Status::unavailable("down"),
        ));
        assert!(!unavailable.is_rate_limited());

        // Non-transport variants are never rate-limited.
        assert!(!DapiClientError::NoAvailableAddresses.is_rate_limited());
        let to_retry = DapiClientError::NoAvailableAddressesToRetry(Box::new(
            TransportError::Grpc(dapi_grpc::tonic::Status::resource_exhausted("429")),
        ));
        assert!(!to_retry.is_rate_limited());
        assert!(
            !DapiClientError::AddressList(AddressListError::InvalidAddressUri("bad".to_string()))
                .is_rate_limited()
        );
    }

    #[test]
    fn test_execution_error_is_rate_limited_delegates() {
        let err: ExecutionError<DapiClientError> = ExecutionError {
            inner: DapiClientError::Transport(TransportError::Grpc(
                dapi_grpc::tonic::Status::resource_exhausted("429"),
            )),
            retries: 0,
            address: Some(mock_address()),
        };
        assert!(err.is_rate_limited());

        let err: ExecutionError<DapiClientError> = ExecutionError {
            inner: DapiClientError::Transport(TransportError::Grpc(
                dapi_grpc::tonic::Status::internal("boom"),
            )),
            retries: 0,
            address: Some(mock_address()),
        };
        assert!(!err.is_rate_limited());
    }

    /// Property: the load-bearing `is_rate_limited() ⇒ can_retry()` invariant
    /// holds for *every* gRPC status code across *every* in-crate `CanRetry`
    /// implementor (`tonic::Status`, `TransportError`, `DapiClientError`,
    /// `ExecutionError`).
    ///
    /// The rotate-don't-ban behavior depends on this: a rate-limited error that
    /// is NOT retryable would skip the retry loop, silently killing rotation
    /// (see the `CanRetry` docs and the `debug_assert!` in `execute`). This test
    /// pins the invariant so any future widening of `is_rate_limited` (or
    /// narrowing of `can_retry`) that breaks it fails loudly here.
    #[test]
    fn test_rate_limit_implies_retryable_invariant() {
        use dapi_grpc::tonic::Code::*;

        // The full gRPC code set — keep exhaustive so a newly classified code
        // can never silently dodge the invariant check.
        let all_codes = [
            Ok,
            Cancelled,
            Unknown,
            InvalidArgument,
            DeadlineExceeded,
            NotFound,
            AlreadyExists,
            PermissionDenied,
            ResourceExhausted,
            FailedPrecondition,
            Aborted,
            OutOfRange,
            Unimplemented,
            Internal,
            Unavailable,
            DataLoss,
            Unauthenticated,
        ];

        let addr = mock_address();

        for code in all_codes {
            // `tonic::Status` is not `Clone`, so build a fresh one per layer.
            let status = dapi_grpc::tonic::Status::new(code, "x");
            assert!(
                !status.is_rate_limited() || status.can_retry(),
                "tonic::Status {code:?}: is_rate_limited() must imply can_retry()"
            );

            let te = TransportError::Grpc(dapi_grpc::tonic::Status::new(code, "x"));
            assert!(
                !te.is_rate_limited() || te.can_retry(),
                "TransportError {code:?}: is_rate_limited() must imply can_retry()"
            );

            let dce = DapiClientError::Transport(TransportError::Grpc(
                dapi_grpc::tonic::Status::new(code, "x"),
            ));
            assert!(
                !dce.is_rate_limited() || dce.can_retry(),
                "DapiClientError {code:?}: is_rate_limited() must imply can_retry()"
            );

            let ee: ExecutionError<DapiClientError> = ExecutionError {
                inner: DapiClientError::Transport(TransportError::Grpc(
                    dapi_grpc::tonic::Status::new(code, "x"),
                )),
                retries: 0,
                address: Some(addr.clone()),
            };
            assert!(
                !ee.is_rate_limited() || ee.can_retry(),
                "ExecutionError {code:?}: is_rate_limited() must imply can_retry()"
            );
        }

        // Guard the non-vacuous case: ResourceExhausted must actually be BOTH
        // rate-limited AND retryable, otherwise the invariant could hold only
        // because nothing is ever rate-limited (the feature would be dead).
        let re = dapi_grpc::tonic::Status::resource_exhausted("429");
        assert!(
            re.is_rate_limited() && re.can_retry(),
            "ResourceExhausted must be rate-limited AND retryable"
        );
    }

    #[test]
    fn test_retry_delay_non_rate_limited_is_flat() {
        // Genuine failures keep the pre-existing flat base delay, regardless of
        // the attempt index — unchanged behavior.
        let base = Duration::from_millis(RETRY_BASE_DELAY_MS);
        for attempt in 0..8u32 {
            assert_eq!(retry_delay(false, attempt), base);
        }
    }

    #[test]
    fn test_rate_limit_backoff_window_grows_and_caps() {
        // base × 2^attempt up to the cap.
        assert_eq!(rate_limit_backoff_window(0), Duration::from_millis(10));
        assert_eq!(rate_limit_backoff_window(1), Duration::from_millis(20));
        assert_eq!(rate_limit_backoff_window(2), Duration::from_millis(40));
        assert_eq!(rate_limit_backoff_window(3), Duration::from_millis(80));
        assert_eq!(rate_limit_backoff_window(4), Duration::from_millis(160));
        assert_eq!(rate_limit_backoff_window(5), Duration::from_millis(320));
        // 10 × 2^6 = 640 → capped at 500.
        assert_eq!(
            rate_limit_backoff_window(6),
            Duration::from_millis(RATE_LIMIT_MAX_DELAY_MS)
        );
        // Monotonic non-decreasing and never above the cap.
        let mut prev = Duration::ZERO;
        for attempt in 0..8u32 {
            let w = rate_limit_backoff_window(attempt);
            assert!(w >= prev, "window must be non-decreasing");
            assert!(w <= Duration::from_millis(RATE_LIMIT_MAX_DELAY_MS));
            prev = w;
        }
        // A pathologically large attempt must not panic and stays capped.
        assert_eq!(
            rate_limit_backoff_window(u32::MAX),
            Duration::from_millis(RATE_LIMIT_MAX_DELAY_MS)
        );
    }

    #[test]
    fn test_retry_delay_rate_limited_is_jittered_within_window() {
        // Use a wide window (attempt 5 → 320ms) so jitter has room to vary.
        let attempt = 5u32;
        let window = rate_limit_backoff_window(attempt);

        let samples: Vec<Duration> = (0..128).map(|_| retry_delay(true, attempt)).collect();

        // Full jitter: every sample is within [0, window).
        for d in &samples {
            assert!(
                *d < window,
                "jittered delay {d:?} must be strictly below the window {window:?}"
            );
        }
        // Jitter is actually applied: across many samples we see more than one
        // distinct value (probability of all-equal is astronomically small).
        let first = samples[0];
        assert!(
            samples.iter().any(|d| *d != first),
            "rate-limit delay must be randomized (jitter), got all equal"
        );
    }

    #[cfg(feature = "mocks")]
    #[test]
    fn test_can_retry_mock_error() {
        let err = DapiClientError::Mock(crate::mock::MockError::MockExpectationNotFound(
            "test".to_string(),
        ));
        assert!(!err.can_retry());
    }

    #[test]
    fn test_is_no_available_addresses() {
        assert!(DapiClientError::NoAvailableAddresses.is_no_available_addresses());

        let transport_err = TransportError::Grpc(dapi_grpc::tonic::Status::unavailable("gone"));
        assert!(
            DapiClientError::NoAvailableAddressesToRetry(Box::new(transport_err))
                .is_no_available_addresses()
        );

        let transport_err =
            TransportError::Grpc(dapi_grpc::tonic::Status::unavailable("temporary"));
        assert!(!DapiClientError::Transport(transport_err).is_no_available_addresses());
    }

    #[test]
    fn test_update_address_ban_status_success_unbans() {
        let mut address_list = AddressList::new();
        let addr = mock_address();
        address_list.add(addr.clone());
        address_list.ban(&addr);
        assert!(address_list.is_banned(&addr));

        let result: ExecutionResult<i32, DapiClientError> = Ok(ExecutionResponse {
            inner: 42,
            retries: 0,
            address: addr.clone(),
        });

        update_address_ban_status(&address_list, &result, &make_applied_settings(true));

        assert!(!address_list.is_banned(&addr));
    }

    #[test]
    fn test_update_address_ban_status_success_on_unbanned_is_noop() {
        let mut address_list = AddressList::new();
        let addr = mock_address();
        address_list.add(addr.clone());

        let result: ExecutionResult<i32, DapiClientError> = Ok(ExecutionResponse {
            inner: 42,
            retries: 0,
            address: addr.clone(),
        });

        // Should not panic or change anything
        update_address_ban_status(&address_list, &result, &make_applied_settings(true));
        assert!(!address_list.is_banned(&addr));
    }

    #[test]
    fn test_update_address_ban_status_retryable_error_bans_address() {
        let mut address_list = AddressList::new();
        let addr = mock_address();
        address_list.add(addr.clone());

        let transport_err =
            TransportError::Grpc(dapi_grpc::tonic::Status::unavailable("temporary"));
        let result: ExecutionResult<i32, DapiClientError> = Err(ExecutionError {
            inner: DapiClientError::Transport(transport_err),
            retries: 0,
            address: Some(addr.clone()),
        });

        update_address_ban_status(&address_list, &result, &make_applied_settings(true));
        assert!(address_list.is_banned(&addr));

        // The ban reason must be propagated from the error via this call path,
        // not just the ban itself.
        let info = address_list.ban_info();
        assert_eq!(info.len(), 1);
        let reason = info[0].reason.as_deref().expect("ban reason recorded");
        assert!(
            reason.contains("temporary"),
            "ban reason should carry the underlying error, got: {reason}"
        );
    }

    /// Invariant 1: the genuine-failure health-ban ladder is unchanged.
    ///
    /// A genuine failure (Unavailable / Internal) must still ban the node with
    /// the `60s × e^ban_count` exponential window, `ban_count` incrementing on
    /// each ban — byte-for-byte the pre-existing behavior.
    #[test]
    fn test_invariant_genuine_failure_still_bans_with_unchanged_ladder() {
        let mut address_list = AddressList::new(); // base ban period = 60s
        let addr = mock_address();
        address_list.add(addr.clone());

        let base_secs = 60.0_f64; // DEFAULT_BASE_BAN_PERIOD

        for expected_count in 1..=3usize {
            // Alternate genuine-failure codes to prove the ladder is not
            // code-specific (both are non-rate-limited, retryable failures).
            let status = if expected_count % 2 == 1 {
                dapi_grpc::tonic::Status::unavailable("node down")
            } else {
                dapi_grpc::tonic::Status::internal("boom")
            };

            let before = chrono::Utc::now();
            let result: ExecutionResult<i32, DapiClientError> = Err(ExecutionError {
                inner: DapiClientError::Transport(TransportError::Grpc(status)),
                retries: 0,
                address: Some(addr.clone()),
            });
            update_address_ban_status(&address_list, &result, &make_applied_settings(true));
            let after = chrono::Utc::now();

            let info = address_list.ban_info();
            let entry = info
                .iter()
                .find(|i| i.uri.contains("3000"))
                .expect("address present in ban info");

            assert!(entry.banned, "genuine failure must ban the node");
            assert_eq!(
                entry.ban_count, expected_count,
                "ban_count must increment on each genuine failure"
            );

            // Window must equal base × e^(ban_count - 1). Since
            // banned_until = t_call + period with before <= t_call <= after:
            //   (banned_until - before) >= period   and
            //   (banned_until - after)  <= period.
            let period = base_secs * ((expected_count - 1) as f64).exp();
            let banned_until = entry.banned_until.expect("banned_until is set");
            let lower = (banned_until - before).num_milliseconds() as f64 / 1000.0;
            let upper = (banned_until - after).num_milliseconds() as f64 / 1000.0;
            assert!(
                lower >= period - 0.05,
                "ban #{expected_count} window lower bound {lower}s < expected {period}s",
            );
            assert!(
                upper <= period + 0.05,
                "ban #{expected_count} window upper bound {upper}s > expected {period}s",
            );
        }
    }

    /// Invariant 2: congestion can never empty the live pool.
    ///
    /// Repeated `ResourceExhausted` across *every* address must never ban any of
    /// them — `banned_until` stays `None`, `ban_count` stays 0 — and
    /// `get_live_address` keeps returning `Some`.
    #[test]
    fn test_invariant_rate_limit_never_bans_and_pool_stays_live() {
        let mut address_list = AddressList::new();
        let addresses: Vec<crate::Address> = (3000..3003)
            .map(|p| {
                format!("http://127.0.0.1:{p}")
                    .parse()
                    .expect("valid address")
            })
            .collect();
        for addr in &addresses {
            address_list.add(addr.clone());
        }

        // Hammer every address with ResourceExhausted across many rounds.
        for _ in 0..10 {
            for addr in &addresses {
                let result: ExecutionResult<i32, DapiClientError> = Err(ExecutionError {
                    inner: DapiClientError::Transport(TransportError::Grpc(
                        dapi_grpc::tonic::Status::resource_exhausted("429"),
                    )),
                    retries: 0,
                    address: Some(addr.clone()),
                });
                update_address_ban_status(&address_list, &result, &make_applied_settings(true));
            }
        }

        for info in address_list.ban_info() {
            assert!(!info.banned, "address {} must not be banned", info.uri);
            assert_eq!(info.ban_count, 0, "ban_count must stay 0 for {}", info.uri);
            assert!(
                info.banned_until.is_none(),
                "banned_until must stay None for {}",
                info.uri
            );
        }
        // The pool is never emptied by congestion.
        assert!(address_list.get_live_address().is_some());
    }

    #[test]
    fn test_update_address_ban_status_retryable_error_ban_disabled() {
        let mut address_list = AddressList::new();
        let addr = mock_address();
        address_list.add(addr.clone());

        let transport_err =
            TransportError::Grpc(dapi_grpc::tonic::Status::unavailable("temporary"));
        let result: ExecutionResult<i32, DapiClientError> = Err(ExecutionError {
            inner: DapiClientError::Transport(transport_err),
            retries: 0,
            address: Some(addr.clone()),
        });

        update_address_ban_status(&address_list, &result, &make_applied_settings(false));
        // With ban disabled, the address should NOT be banned
        assert!(!address_list.is_banned(&addr));
    }

    #[test]
    fn test_update_address_ban_status_non_retryable_error_does_not_ban() {
        let mut address_list = AddressList::new();
        let addr = mock_address();
        address_list.add(addr.clone());

        let result: ExecutionResult<i32, DapiClientError> = Err(ExecutionError {
            inner: DapiClientError::NoAvailableAddresses,
            retries: 0,
            address: Some(addr.clone()),
        });

        update_address_ban_status(&address_list, &result, &make_applied_settings(true));
        assert!(!address_list.is_banned(&addr));
    }

    #[test]
    fn test_update_address_ban_status_retryable_error_no_address() {
        let address_list = AddressList::new();

        let transport_err =
            TransportError::Grpc(dapi_grpc::tonic::Status::unavailable("temporary"));
        let result: ExecutionResult<i32, DapiClientError> = Err(ExecutionError {
            inner: DapiClientError::Transport(transport_err),
            retries: 0,
            address: None,
        });

        // Should not panic when address is None
        update_address_ban_status(&address_list, &result, &make_applied_settings(true));
    }

    #[test]
    fn test_update_address_ban_status_unban_removed_address() {
        let mut address_list = AddressList::new();
        let addr = mock_address();
        address_list.add(addr.clone());
        address_list.ban(&addr);

        // Remove the address
        address_list.remove(&addr);

        let result: ExecutionResult<i32, DapiClientError> = Ok(ExecutionResponse {
            inner: 42,
            retries: 0,
            address: addr.clone(),
        });

        // Should not panic when trying to unban a removed address
        update_address_ban_status(&address_list, &result, &make_applied_settings(true));
    }

    #[test]
    fn test_update_address_ban_status_ban_removed_address() {
        let address_list = AddressList::new();
        let addr = mock_address();

        let transport_err =
            TransportError::Grpc(dapi_grpc::tonic::Status::unavailable("temporary"));
        let result: ExecutionResult<i32, DapiClientError> = Err(ExecutionError {
            inner: DapiClientError::Transport(transport_err),
            retries: 0,
            address: Some(addr),
        });

        // Should not panic when trying to ban an address not in the list
        update_address_ban_status(&address_list, &result, &make_applied_settings(true));
    }

    #[test]
    fn test_dapi_client_new() {
        let address_list: AddressList = "http://127.0.0.1:3000,http://127.0.0.1:3001"
            .parse()
            .unwrap();
        let client = DapiClient::new(address_list, RequestSettings::default());
        assert_eq!(client.address_list().len(), 2);
    }

    #[test]
    fn test_dapi_client_get_live_addresses() {
        let address_list: AddressList = "http://127.0.0.1:3000,http://127.0.0.1:3001"
            .parse()
            .unwrap();
        let client = DapiClient::new(address_list, RequestSettings::default());
        let live = client.get_live_addresses();
        assert_eq!(live.len(), 2);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_dapi_client_with_ca_certificate() {
        let address_list: AddressList = "http://127.0.0.1:3000".parse().unwrap();
        let client = DapiClient::new(address_list, RequestSettings::default());
        let cert = dapi_grpc::tonic::transport::Certificate::from_pem("fake-pem-data");
        let client = client.with_ca_certificate(cert);
        assert!(client.ca_certificate.is_some());
    }

    #[cfg(feature = "mocks")]
    #[test]
    fn test_dapi_client_error_mock_serialize_deserialize() {
        use dapi_grpc::mock::Mockable;

        let err = DapiClientError::NoAvailableAddresses;
        let serialized = err.mock_serialize().expect("should serialize");
        let deserialized =
            DapiClientError::mock_deserialize(&serialized).expect("should deserialize");
        assert!(matches!(
            deserialized,
            DapiClientError::NoAvailableAddresses
        ));
    }

    #[cfg(feature = "mocks")]
    #[test]
    fn test_dapi_client_error_transport_mock_roundtrip() {
        use dapi_grpc::mock::Mockable;

        let transport_err = TransportError::Grpc(dapi_grpc::tonic::Status::unavailable("test"));
        let err = DapiClientError::Transport(transport_err);
        let serialized = err.mock_serialize().expect("should serialize");
        let deserialized =
            DapiClientError::mock_deserialize(&serialized).expect("should deserialize");
        assert!(matches!(deserialized, DapiClientError::Transport(_)));
    }

    #[test]
    fn test_dapi_client_error_display() {
        let err = DapiClientError::NoAvailableAddresses;
        let display = format!("{}", err);
        assert!(display.contains("no available addresses"));

        let transport_err = TransportError::Grpc(dapi_grpc::tonic::Status::unavailable("gone"));
        let err = DapiClientError::NoAvailableAddressesToRetry(Box::new(transport_err));
        let display = format!("{}", err);
        assert!(display.contains("no available addresses to retry"));

        let err =
            DapiClientError::AddressList(AddressListError::InvalidAddressUri("bad".to_string()));
        let display = format!("{}", err);
        assert!(display.contains("address list error"));
    }
}

#[async_trait]
impl DapiRequestExecutor for DapiClient {
    /// Execute the [DapiRequest](crate::DapiRequest).
    async fn execute<R>(
        &self,
        request: R,
        settings: RequestSettings,
    ) -> ExecutionResult<R::Response, DapiClientError>
    where
        R: TransportRequest + Mockable,
        R::Response: Mockable,
        TransportError: Mockable,
    {
        // Join settings of different sources to get final version of the settings for this execution:
        let applied_settings = self
            .settings
            .override_by(R::SETTINGS_OVERRIDES)
            .override_by(settings)
            .finalize();
        #[cfg(not(target_arch = "wasm32"))]
        let applied_settings = applied_settings.with_ca_certificate(self.ca_certificate.clone());

        // Save dump dir for later use
        #[cfg(feature = "dump")]
        let dump_dir = self.dump_dir.clone();
        #[cfg(feature = "dump")]
        let dump_request = request.clone();

        let max_retries = applied_settings.retries;

        let mut retries: usize = 0;
        // Track the last transport error for when all addresses get exhausted
        let mut last_transport_error: Option<TransportError> = None;
        // Addresses already tried in this execution. We exclude them when
        // selecting the next node so a retry rotates to a *different* node —
        // crucial for rate-limited (ResourceExhausted) nodes, which stay in
        // the pool (not banned) and would otherwise be picked again.
        let mut tried: Vec<crate::Address> = Vec::new();

        let result: ExecutionResult<R::Response, DapiClientError> = async {
            loop {
                // Select the next node, rotating off every node already tried in
                // this execution so a retry goes to a *different* node.
                // Rate-limited (ResourceExhausted) nodes stay in the pool (never
                // banned), so without this exclusion a retry could re-pick the
                // very node that just throttled us.
                //
                // Fallback order once every live node has been tried:
                //   1. exclude only the node we *just* tried, so a small pool
                //      (e.g. two nodes) keeps alternating instead of re-hitting
                //      the just-throttled node at random;
                //   2. if that is still empty (single-node pool, or the
                //      just-tried node is the only live one), reuse it — there is
                //      no alternative. The backoff below keeps this from
                //      hammering the throttled node.
                let just_tried = tried.last().cloned();
                let Some(address) = self
                    .address_list
                    .get_live_address_excluding(&tried)
                    .or_else(|| {
                        just_tried.as_ref().and_then(|last| {
                            self.address_list
                                .get_live_address_excluding(std::slice::from_ref(last))
                        })
                    })
                    .or_else(|| self.address_list.get_live_address())
                else {
                    // No available addresses - wrap with last meaningful error if we have one
                    let error = if let Some(transport_error) = last_transport_error.take() {
                        tracing::debug!(
                            "no addresses available, returning last transport error"
                        );
                        DapiClientError::NoAvailableAddressesToRetry(Box::new(
                            transport_error,
                        ))
                    } else {
                        DapiClientError::NoAvailableAddresses
                    };

                    return Err(ExecutionError {
                        inner: error,
                        retries,
                        address: None,
                    });
                };

                // Rec 3 — explicit trace event so the resolved DAPI endpoint
                // appears in flat plain-text log output (not just the span context).
                tracing::trace!(
                    target: "dapi_client::dispatch",
                    ?address,
                    method = request.method_name(),
                    request_type = request.request_name(),
                    "dispatching request to DAPI endpoint"
                );
                tracing::trace!(
                    ?request,
                    "calling {} with {} request",
                    request.method_name(),
                    request.request_name(),
                );

                let transport_request = request.clone();
                let response_name = request.response_name();

                // Try to create transport client
                let transport_client_result = R::Client::with_uri_and_settings(
                    address.uri().clone(),
                    &applied_settings,
                    &self.pool,
                );

                let mut transport_client = match transport_client_result {
                    Ok(client) => client,
                    Err(transport_error) => {
                        let can_retry_error = transport_error.can_retry();

                        // Clone error before moving it
                        let cloned_error = transport_error.clone();

                        let execution_error = ExecutionError {
                            inner: DapiClientError::Transport(transport_error),
                            retries,
                            address: Some(address.clone()),
                        };

                        update_address_ban_status::<R::Response, DapiClientError>(
                            &self.address_list,
                            &Err(execution_error.clone()),
                            &applied_settings,
                        );

                        if can_retry_error && retries < max_retries {
                            let rate_limited = cloned_error.is_rate_limited();
                            // Store last transport error
                            last_transport_error = Some(cloned_error);

                            // Rotate off this node on the next attempt.
                            tried.push(address.clone());

                            let delay = retry_delay(rate_limited, retries as u32);
                            retries += 1;
                            tracing::warn!(
                                error = ?execution_error,
                                delay_ms = delay.as_millis() as u64,
                                "retrying error after backoff"
                            );
                            transport::sleep(delay).await;
                            continue;
                        }

                        return Err(execution_error);
                    }
                };

                // Execute the transport request
                let result = transport_request
                    .execute_transport(&mut transport_client, &applied_settings)
                    .instrument(tracing::trace_span!(
                        "execute_request",
                        ?address,
                        settings = ?applied_settings,
                        method = request.method_name(),
                    ))
                    .await;

                let execution_result = match result {
                    Ok(response) => {
                        tracing::trace!(response = ?response, "received {} response", response_name);
                        Ok(ExecutionResponse {
                            inner: response,
                            retries,
                            address: address.clone(),
                        })
                    }
                    Err(transport_error) => {
                        tracing::debug!(error = ?transport_error, "received error: {transport_error}");
                        Err(ExecutionError {
                            inner: DapiClientError::Transport(transport_error),
                            retries,
                            address: Some(address.clone()),
                        })
                    }
                };

                update_address_ban_status::<R::Response, DapiClientError>(
                    &self.address_list,
                    &execution_result,
                    &applied_settings,
                );

                match execution_result {
                    Ok(response) => return Ok(response),
                    Err(error) => {
                        // Invariant lock (load-bearing): a rate-limited error MUST
                        // be retryable, otherwise the rotation below never fires
                        // and the rotate-don't-ban behavior silently dies. See the
                        // `CanRetry` docs and the `test_rate_limit_implies_retryable_invariant`
                        // property test.
                        debug_assert!(
                            !error.is_rate_limited() || error.can_retry(),
                            "is_rate_limited() must imply can_retry(); rate-limit rotation depends on it"
                        );
                        if error.can_retry() && retries < max_retries {
                            let rate_limited = error.is_rate_limited();
                            // Store last transport error
                            if let DapiClientError::Transport(ref te) = error.inner {
                                last_transport_error = Some(te.clone());
                            }

                            // Rotate off this node on the next attempt.
                            tried.push(address.clone());

                            let delay = retry_delay(rate_limited, retries as u32);
                            retries += 1;
                            tracing::warn!(
                                ?error,
                                delay_ms = delay.as_millis() as u64,
                                "retrying error after backoff"
                            );
                            transport::sleep(delay).await;
                            continue;
                        }

                        return Err(error);
                    }
                }
            }
        }
        .instrument(tracing::info_span!("request routine"))
        .await;

        if let Err(error) = &result {
            if !error.can_retry() {
                tracing::error!(?error, "request failed");
            }
        }

        // Dump request and response to disk if dump_dir is set:
        #[cfg(feature = "dump")]
        Self::dump_request_response(&dump_request, &result, dump_dir);

        result
    }
}
