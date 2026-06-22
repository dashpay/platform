//! Failover behavior for gRPC `ResourceExhausted` (rate-limit / backpressure).
//!
//! Unlike a genuine node failure, a rate-limited node is *healthy* — it is just
//! throttled. The executor must therefore NOT ban it (banning would relocate
//! its load onto the survivors and cascade into `NoAvailableAddressesToRetry`).
//! Instead the retry must rotate to a *different* node, leaving the throttled
//! one in the live pool. The bounded retry count bounds an over-limit client.
//!
//! These tests drive the real `DapiClient::execute` retry loop through a fake
//! transport, complementing the unit tests in `src/`.

use std::sync::{Arc, Mutex};

use dapi_grpc::mock::Mockable;
use dapi_grpc::tonic::Status;
use rs_dapi_client::transport::{
    AppliedRequestSettings, BoxFuture, TransportClient, TransportError, TransportRequest,
};
use rs_dapi_client::{
    Address, AddressList, CanRetry, ConnectionPool, DapiClient, DapiClientError,
    DapiRequestExecutor, RequestSettings, Uri,
};

/// Transport client that only remembers which node it was created for.
struct FakeClient {
    uri: Uri,
}

impl TransportClient for FakeClient {
    fn with_uri(uri: Uri, _pool: &ConnectionPool) -> Result<Self, TransportError> {
        Ok(Self { uri })
    }

    fn with_uri_and_settings(
        uri: Uri,
        _settings: &AppliedRequestSettings,
        _pool: &ConnectionPool,
    ) -> Result<Self, TransportError> {
        Ok(Self { uri })
    }
}

#[derive(Debug)]
struct FakeResponse;

impl Mockable for FakeResponse {}

#[derive(Debug, Default)]
struct State {
    /// How many more attempts should be answered with RESOURCE_EXHAUSTED,
    /// simulating a throttled (but healthy) node.
    rate_limited_responses_left: usize,
    /// Nodes that answered RESOURCE_EXHAUSTED, in order.
    rate_limited_uris: Vec<Uri>,
}

/// Request that answers RESOURCE_EXHAUSTED for the first N attempts and
/// succeeds afterwards, recording which node served each throttled attempt.
#[derive(Clone, Debug)]
struct RateLimitedRequest {
    state: Arc<Mutex<State>>,
}

impl RateLimitedRequest {
    fn with_rate_limited_responses(count: usize) -> Self {
        Self {
            state: Arc::new(Mutex::new(State {
                rate_limited_responses_left: count,
                rate_limited_uris: Vec::new(),
            })),
        }
    }
}

impl Mockable for RateLimitedRequest {}

impl TransportRequest for RateLimitedRequest {
    type Client = FakeClient;
    type Response = FakeResponse;

    const SETTINGS_OVERRIDES: RequestSettings = RequestSettings::default();

    fn method_name(&self) -> &'static str {
        "fake_rate_limited_method"
    }

    fn execute_transport<'c>(
        self,
        client: &'c mut Self::Client,
        _settings: &AppliedRequestSettings,
    ) -> BoxFuture<'c, Result<Self::Response, TransportError>> {
        let result = {
            let mut state = self.state.lock().unwrap();
            if state.rate_limited_responses_left > 0 {
                state.rate_limited_responses_left -= 1;
                state.rate_limited_uris.push(client.uri.clone());
                Err(TransportError::Grpc(Status::resource_exhausted(
                    "rate limit exceeded",
                )))
            } else {
                Ok(FakeResponse)
            }
        };

        Box::pin(async move { result })
    }
}

/// A rate-limited node must NOT be banned, and the retry must rotate to a
/// different node, where the request succeeds.
#[tokio::test]
async fn rate_limited_node_is_not_banned_and_request_rotates_to_another() {
    // One throttled response, then success on the rotated-to node.
    let request = RateLimitedRequest::with_rate_limited_responses(1);
    let address_list: AddressList = "http://127.0.0.1:20001,http://127.0.0.1:20002"
        .parse()
        .expect("valid address list");
    let client = DapiClient::new(address_list, RequestSettings::default());

    let response = client
        .execute(request.clone(), RequestSettings::default())
        .await
        .expect("request should succeed on the rotated-to node");

    let throttled_uri = {
        let state = request.state.lock().unwrap();
        assert_eq!(
            state.rate_limited_uris.len(),
            1,
            "exactly one node should have answered RESOURCE_EXHAUSTED"
        );
        state.rate_limited_uris[0].clone()
    };

    assert_eq!(response.retries, 1);
    assert_ne!(
        response.address.uri(),
        &throttled_uri,
        "retry must rotate to a different node than the throttled one"
    );

    // The throttled node stays healthy in the pool — it must NOT be banned.
    let throttled = Address::try_from(throttled_uri).expect("valid address");
    assert!(
        !client.address_list().is_banned(&throttled),
        "a rate-limited node must NOT be banned"
    );
}

/// Sustained congestion across *every* node must never ban any of them and must
/// never empty the live pool. The loop is bounded by the retry budget and
/// surfaces the raw, still-retryable `ResourceExhausted`.
#[tokio::test]
async fn congestion_never_bans_nodes_nor_empties_pool() {
    let settings = RequestSettings {
        retries: Some(3),
        ..Default::default()
    };

    // Every attempt is throttled.
    let request = RateLimitedRequest::with_rate_limited_responses(usize::MAX);
    let address_list: AddressList = "http://127.0.0.1:20003,http://127.0.0.1:20004"
        .parse()
        .expect("valid address list");
    let client = DapiClient::new(address_list, settings);

    let error = client
        .execute(request.clone(), settings)
        .await
        .expect_err("request must fail when every node is throttled");

    // Bounded purely by the retry budget (no banning, no address exhaustion):
    // initial attempt + 3 retries = 4 attempts.
    assert_eq!(
        request.state.lock().unwrap().rate_limited_uris.len(),
        4,
        "attempts must be bounded by retry budget (retries + 1), not by banning"
    );

    // No node was banned, so the pool is never emptied by congestion.
    for info in client.address_list().ban_info() {
        assert!(!info.banned, "address {} must not be banned", info.uri);
        assert_eq!(info.ban_count, 0, "ban_count must stay 0 for {}", info.uri);
        assert!(
            info.banned_until.is_none(),
            "banned_until must stay None for {}",
            info.uri
        );
    }
    assert!(
        client.address_list().get_live_address().is_some(),
        "congestion must never empty the live pool"
    );

    // The raw ResourceExhausted surfaces and stays retryable — it never
    // collapses into the non-retryable NoAvailableAddressesToRetry.
    assert!(
        error.can_retry(),
        "surfaced ResourceExhausted must stay retryable"
    );
    match error.inner {
        DapiClientError::Transport(TransportError::Grpc(status)) => {
            assert_eq!(status.code(), dapi_grpc::tonic::Code::ResourceExhausted)
        }
        other => panic!("expected raw Transport(Grpc(ResourceExhausted)), got: {other:?}"),
    }
}
