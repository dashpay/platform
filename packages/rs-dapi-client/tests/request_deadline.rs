//! Client-side attempt deadline: an attempt whose response never completes
//! (a node that stalls mid-response, or a half-open connection) must fail with
//! `DeadlineExceeded` after `timeout + connect_timeout` and be retried on
//! another node, instead of hanging the request forever.
//!
//! The tests run on tokio's paused clock, so the deadlines elapse instantly.

#[allow(dead_code)]
mod common;

use std::fmt::Debug;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use common::{FakeClient, FakeResponse};
use dapi_grpc::mock::Mockable;
use dapi_grpc::tonic::Code;
use rs_dapi_client::transport::{
    AppliedRequestSettings, BoxFuture, TransportError, TransportRequest,
};
use rs_dapi_client::{
    Address, AddressList, DapiClient, DapiClientError, DapiRequestExecutor, RequestSettings, Uri,
};

/// How a fake node answers one attempt.
#[derive(Clone, Copy)]
enum Answer {
    /// Respond successfully after this much time.
    After(Duration),
    /// Never finish the response.
    Never,
}

/// Fake request whose `answer` closure decides, per node, how the attempt
/// behaves over time. `hit_uris` records every node the executor tried.
#[derive(Clone)]
struct DelayedRequest {
    answer: Arc<dyn Fn(&Uri) -> Answer + Send + Sync>,
    hit_uris: Arc<Mutex<Vec<Uri>>>,
}

impl DelayedRequest {
    fn new(answer: impl Fn(&Uri) -> Answer + Send + Sync + 'static) -> Self {
        Self {
            answer: Arc::new(answer),
            hit_uris: Default::default(),
        }
    }

    fn hits(&self) -> Vec<Uri> {
        self.hit_uris.lock().unwrap().clone()
    }
}

impl Debug for DelayedRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DelayedRequest")
            .field("hit_uris", &self.hit_uris)
            .finish()
    }
}

impl Mockable for DelayedRequest {}

impl TransportRequest for DelayedRequest {
    type Client = FakeClient;
    type Response = FakeResponse;

    const SETTINGS_OVERRIDES: RequestSettings = RequestSettings::default();

    fn method_name(&self) -> &'static str {
        "delayed_fake_method"
    }

    fn execute_transport<'c>(
        self,
        client: &'c mut Self::Client,
        _settings: &AppliedRequestSettings,
    ) -> BoxFuture<'c, Result<Self::Response, TransportError>> {
        let uri = client.uri.clone();
        self.hit_uris.lock().unwrap().push(uri.clone());
        match (self.answer)(&uri) {
            Answer::After(delay) => Box::pin(async move {
                tokio::time::sleep(delay).await;
                Ok(FakeResponse)
            }),
            Answer::Never => Box::pin(futures::future::pending()),
        }
    }
}

fn two_nodes() -> AddressList {
    "http://127.0.0.1:10001,http://127.0.0.1:10002"
        .parse()
        .expect("valid address list")
}

#[tokio::test(start_paused = true)]
async fn should_retry_on_another_node_when_an_attempt_misses_its_deadline() {
    // The first node the executor picks stalls forever; every other node
    // answers at once.
    let stalled: Arc<Mutex<Option<Uri>>> = Default::default();
    let stalled_c = stalled.clone();
    let request = DelayedRequest::new(move |uri| {
        let mut stalled = stalled_c.lock().unwrap();
        let target = stalled.get_or_insert_with(|| uri.clone());
        if *target == *uri {
            Answer::Never
        } else {
            Answer::After(Duration::ZERO)
        }
    });
    let client = DapiClient::new(two_nodes(), RequestSettings::default());

    let started = tokio::time::Instant::now();
    let response = client
        .execute(request.clone(), RequestSettings::default())
        .await
        .expect("the retry on the healthy node must succeed");

    let stalled_uri = stalled.lock().unwrap().clone().expect("a node stalled");
    assert_eq!(response.retries, 1);
    assert_ne!(response.address.uri(), &stalled_uri);
    assert_eq!(request.hits().len(), 2);
    assert!(
        started.elapsed() >= Duration::from_secs(10),
        "the stalled attempt must run until the default 10 s deadline, not be cut earlier"
    );
    let stalled_node = Address::try_from(stalled_uri).expect("valid address");
    assert!(
        client.address_list().is_banned(&stalled_node),
        "the node that missed the deadline must be banned"
    );
}

#[tokio::test(start_paused = true)]
async fn should_fail_with_deadline_exceeded_when_every_node_stalls() {
    let request = DelayedRequest::new(|_| Answer::Never);
    let client = DapiClient::new(two_nodes(), RequestSettings::default());

    let error = client
        .execute(request.clone(), RequestSettings::default())
        .await
        .expect_err("no node ever completes a response");

    // Both nodes were tried once and banned, then the address list ran dry.
    assert_eq!(request.hits().len(), 2);
    match error.inner {
        DapiClientError::NoAvailableAddressesToRetry(last) => {
            let TransportError::Grpc(status) = *last;
            assert_eq!(status.code(), Code::DeadlineExceeded);
        }
        other => panic!("expected NoAvailableAddressesToRetry, got {other:?}"),
    }
}

#[tokio::test(start_paused = true)]
async fn should_cut_a_response_that_outlasts_timeout_plus_connect_timeout() {
    // The response would complete, but only after the attempt deadline:
    // headers-only timeouts let this through; the attempt deadline must not.
    let request = DelayedRequest::new(|_| Answer::After(Duration::from_secs(14)));
    let settings = RequestSettings {
        timeout: Some(Duration::from_secs(10)),
        connect_timeout: Some(Duration::from_secs(3)),
        retries: Some(0),
        ..RequestSettings::default()
    };
    let client = DapiClient::new(two_nodes(), settings);

    let error = client
        .execute(request.clone(), RequestSettings::default())
        .await
        .expect_err("a 14 s response exceeds the 13 s attempt deadline");

    match error.inner {
        DapiClientError::Transport(TransportError::Grpc(status)) => {
            assert_eq!(status.code(), Code::DeadlineExceeded)
        }
        other => panic!("expected a gRPC DeadlineExceeded, got {other:?}"),
    }
}

#[tokio::test(start_paused = true)]
async fn should_not_cut_an_attempt_when_timeout_is_zero() {
    // Zero means "no limit", for the `grpc-timeout` header and the attempt alike.
    let request = DelayedRequest::new(|_| Answer::After(Duration::from_secs(60)));
    let settings = RequestSettings {
        timeout: Some(Duration::ZERO),
        ..RequestSettings::default()
    };
    let client = DapiClient::new(two_nodes(), RequestSettings::default());

    let response = client
        .execute(request.clone(), settings)
        .await
        .expect("a slow response must complete when the timeout is disabled");

    assert_eq!(response.retries, 0);
    assert_eq!(request.hits().len(), 1);
}
