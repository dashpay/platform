//! Failover behavior for gRPC `Unimplemented` during a mixed-version network
//! rollout: a node running an older build answers UNIMPLEMENTED for a method
//! that upgraded nodes already serve. The executor must ban that node and
//! retry the request on another address. If every node is unimplemented, the
//! error must still surface instead of retrying forever.

use std::sync::{Arc, Mutex};

use dapi_grpc::mock::Mockable;
use dapi_grpc::tonic::{Code, Status};
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
    /// How many more attempts should be answered with UNIMPLEMENTED,
    /// simulating nodes that run an older build.
    unimplemented_responses_left: usize,
    /// Nodes that answered UNIMPLEMENTED, in order.
    unimplemented_uris: Vec<Uri>,
}

/// Request that answers UNIMPLEMENTED for the first N attempts (each failing
/// node gets banned by the executor, so each attempt hits a different node)
/// and succeeds afterwards.
#[derive(Clone, Debug)]
struct MixedVersionRequest {
    state: Arc<Mutex<State>>,
}

impl MixedVersionRequest {
    fn with_unimplemented_responses(count: usize) -> Self {
        Self {
            state: Arc::new(Mutex::new(State {
                unimplemented_responses_left: count,
                unimplemented_uris: Vec::new(),
            })),
        }
    }
}

impl Mockable for MixedVersionRequest {}

impl TransportRequest for MixedVersionRequest {
    type Client = FakeClient;
    type Response = FakeResponse;

    const SETTINGS_OVERRIDES: RequestSettings = RequestSettings::default();

    fn method_name(&self) -> &'static str {
        "fake_shielded_method"
    }

    fn execute_transport<'c>(
        self,
        client: &'c mut Self::Client,
        _settings: &AppliedRequestSettings,
    ) -> BoxFuture<'c, Result<Self::Response, TransportError>> {
        let result = {
            let mut state = self.state.lock().unwrap();
            if state.unimplemented_responses_left > 0 {
                state.unimplemented_responses_left -= 1;
                state.unimplemented_uris.push(client.uri.clone());
                Err(TransportError::Grpc(Status::unimplemented(
                    "Operation is not implemented or not supported",
                )))
            } else {
                Ok(FakeResponse)
            }
        };

        Box::pin(async move { result })
    }
}

#[tokio::test]
async fn unimplemented_node_is_banned_and_request_retried_on_another() {
    // One of the two nodes still runs an older build without the method.
    let request = MixedVersionRequest::with_unimplemented_responses(1);
    let address_list: AddressList = "http://127.0.0.1:10001,http://127.0.0.1:10002"
        .parse()
        .expect("valid address list");
    let client = DapiClient::new(address_list, RequestSettings::default());

    let response = client
        .execute(request.clone(), RequestSettings::default())
        .await
        .expect("request should succeed on the upgraded node");

    let old_node_uri = {
        let state = request.state.lock().unwrap();
        assert_eq!(
            state.unimplemented_uris.len(),
            1,
            "exactly one node should have answered UNIMPLEMENTED"
        );
        state.unimplemented_uris[0].clone()
    };

    assert_eq!(response.retries, 1);
    assert_ne!(
        response.address.uri(),
        &old_node_uri,
        "retry must go to a different node"
    );

    let old_node = Address::try_from(old_node_uri).expect("valid address");
    assert!(
        client.address_list().is_banned(&old_node),
        "the node that answered UNIMPLEMENTED must be banned"
    );
}

#[tokio::test]
async fn unimplemented_on_all_nodes_still_surfaces_error() {
    // No node implements the method: every attempt answers UNIMPLEMENTED.
    let request = MixedVersionRequest::with_unimplemented_responses(usize::MAX);
    let address_list: AddressList = "http://127.0.0.1:10003,http://127.0.0.1:10004"
        .parse()
        .expect("valid address list");
    let client = DapiClient::new(address_list, RequestSettings::default());

    let error = client
        .execute(request.clone(), RequestSettings::default())
        .await
        .expect_err("request must fail when no node implements the method");

    // Both nodes were tried once each before the address list was exhausted.
    assert_eq!(request.state.lock().unwrap().unimplemented_uris.len(), 2);
    assert!(
        !error.can_retry(),
        "exhausted-addresses error must not be retried by callers"
    );

    match error.inner {
        DapiClientError::NoAvailableAddressesToRetry(transport_error) => match *transport_error {
            TransportError::Grpc(status) => assert_eq!(status.code(), Code::Unimplemented),
        },
        other => panic!("expected NoAvailableAddressesToRetry, got: {other:?}"),
    }
}

#[tokio::test]
async fn unimplemented_surfaces_retryable_when_pool_exceeds_retry_budget() {
    // The non-retryable `NoAvailableAddressesToRetry` collapse (above) only holds
    // when the live address list is exhausted before the retry budget, i.e.
    // `live_addresses <= settings.retries`. When the pool is LARGER than the retry
    // budget, the per-call retry cap trips first: the executor surfaces the raw
    // (still-retryable) `Unimplemented` after banning `retries + 1` nodes, leaving
    // the rest of the pool live. A caller honoring `CanRetry` re-enters and bans
    // more nodes each round (rs-sdk additionally caps via `total_retries`), so
    // termination is still bounded — this test pins the contract for that branch.
    let settings = RequestSettings {
        retries: Some(1),
        ..Default::default()
    };

    // Three live nodes, retry budget of 1 → at most retries + 1 = 2 attempts, so
    // the retry cap trips before all three addresses are banned.
    let request = MixedVersionRequest::with_unimplemented_responses(usize::MAX);
    let address_list: AddressList =
        "http://127.0.0.1:10005,http://127.0.0.1:10006,http://127.0.0.1:10007"
            .parse()
            .expect("valid address list");
    let client = DapiClient::new(address_list, settings);

    let error = client
        .execute(request.clone(), settings)
        .await
        .expect_err("request must fail when no node implements the method");

    // retries + 1 = 2 nodes were tried (and banned) before the retry cap tripped;
    // the third address is never reached.
    assert_eq!(
        request.state.lock().unwrap().unimplemented_uris.len(),
        2,
        "retry budget (1) caps attempts at 2 before the 3-node pool is exhausted"
    );

    // Because the retry cap — not address exhaustion — terminated the loop, the
    // raw Unimplemented surfaces and is still retryable (unlike the exhausted-pool
    // case, which collapses to the non-retryable NoAvailableAddressesToRetry).
    assert!(
        error.can_retry(),
        "with more live nodes than retries, the surfaced Unimplemented stays retryable"
    );
    match error.inner {
        DapiClientError::Transport(TransportError::Grpc(status)) => {
            assert_eq!(status.code(), Code::Unimplemented)
        }
        other => panic!("expected raw Transport(Grpc(Unimplemented)), got: {other:?}"),
    }
}
