//! Regression through tonic itself: a node that sends the response headers
//! of a unary call and then never sends its body.
//!
//! tonic enforces the `grpc-timeout` header only until the response headers
//! arrive, so such a call never completes on its own. The executor's attempt
//! deadline must cut it with `DeadlineExceeded` and fail over to another
//! node. The server runs in memory over a duplex stream, so the tests need no
//! network access.

#[allow(dead_code)]
mod common;

use std::fmt::Debug;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use common::FakeClient;
use dapi_grpc::mock::Mockable;
use dapi_grpc::platform::v0::{GetStatusRequest, GetStatusResponse};
use dapi_grpc::tonic::transport::{Channel, Endpoint};
use dapi_grpc::tonic::{Code, IntoRequest};
use hyper_util::rt::TokioIo;
use rs_dapi_client::transport::{
    AppliedRequestSettings, BoxFuture, PlatformGrpcClient, TransportError, TransportRequest,
};
use rs_dapi_client::{
    Address, AddressList, DapiClient, DapiClientError, DapiRequestExecutor, RequestSettings, Uri,
};

const ATTEMPT_TIMEOUT: Duration = Duration::from_millis(200);

/// A gRPC client whose channel runs over an in-memory duplex stream to an
/// HTTP/2 server that answers every request with `200` response headers and
/// then never sends a body or trailers.
fn body_stalling_client() -> PlatformGrpcClient {
    let (client_io, server_io) = tokio::io::duplex(64 * 1024);
    tokio::spawn(async move {
        let mut connection = h2::server::handshake(server_io)
            .await
            .expect("h2 handshake");
        // Keep every response stream open (and the connection polled) so the
        // body stays pending instead of the stream being reset.
        let mut open_streams = Vec::new();
        while let Some(accepted) = connection.accept().await {
            let (_request, mut respond) = accepted.expect("accept request");
            let headers = http::Response::builder()
                .status(200)
                .header("content-type", "application/grpc")
                .body(())
                .expect("response headers");
            open_streams.push(
                respond
                    .send_response(headers, false)
                    .expect("send response headers"),
            );
        }
    });

    let mut io = Some(client_io);
    let channel: Channel = Endpoint::from_static("http://body-stalling.in-memory")
        .connect_with_connector_lazy(tower::service_fn(move |_: Uri| {
            let io = io.take();
            async move {
                io.map(TokioIo::new)
                    .ok_or_else(|| std::io::Error::other("the in-memory stream is single-use"))
            }
        }));
    PlatformGrpcClient::new(channel)
}

/// `getStatus` request whose first attempted node is served by the
/// body-stalling tonic channel; every other node answers at once.
#[derive(Clone)]
struct StatusRequest {
    stalling_client: PlatformGrpcClient,
    stalled_node: Arc<Mutex<Option<Uri>>>,
}

impl StatusRequest {
    fn new() -> Self {
        Self {
            stalling_client: body_stalling_client(),
            stalled_node: Default::default(),
        }
    }

    fn stalled_node(&self) -> Option<Uri> {
        self.stalled_node.lock().expect("stalled node lock").clone()
    }
}

impl Debug for StatusRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StatusRequest")
            .field("stalled_node", &self.stalled_node)
            .finish()
    }
}

impl Mockable for StatusRequest {}

impl TransportRequest for StatusRequest {
    type Client = FakeClient;
    type Response = GetStatusResponse;

    const SETTINGS_OVERRIDES: RequestSettings = RequestSettings::default();

    fn method_name(&self) -> &'static str {
        "get_status"
    }

    fn execute_transport<'c>(
        self,
        client: &'c mut Self::Client,
        settings: &AppliedRequestSettings,
    ) -> BoxFuture<'c, Result<Self::Response, TransportError>> {
        let is_stalled_node = {
            let mut stalled = self.stalled_node.lock().expect("stalled node lock");
            *stalled.get_or_insert_with(|| client.uri.clone()) == client.uri
        };
        if !is_stalled_node {
            return Box::pin(async { Ok(GetStatusResponse::default()) });
        }
        // The same shape as the production transport: the request timeout
        // travels only as the `grpc-timeout` header.
        let mut grpc = self.stalling_client;
        let mut request = GetStatusRequest::default().into_request();
        request.set_timeout(settings.timeout);
        Box::pin(async move {
            grpc.get_status(request)
                .await
                .map(|response| response.into_inner())
                .map_err(TransportError::Grpc)
        })
    }
}

fn settings(retries: usize) -> RequestSettings {
    RequestSettings {
        timeout: Some(ATTEMPT_TIMEOUT),
        retries: Some(retries),
        ..RequestSettings::default()
    }
}

/// The premise: with only the `grpc-timeout` header, a unary call whose body
/// never arrives is still pending long after that timeout.
#[tokio::test]
async fn should_leave_a_body_stalled_call_pending_with_only_the_grpc_timeout_header() {
    let mut client = body_stalling_client();
    let mut request = GetStatusRequest::default().into_request();
    request.set_timeout(ATTEMPT_TIMEOUT);

    let outcome = tokio::time::timeout(Duration::from_secs(2), client.get_status(request)).await;

    assert!(
        outcome.is_err(),
        "tonic must still be waiting for the body 2 s after a 200 ms grpc-timeout, got {outcome:?}"
    );
}

#[tokio::test]
async fn should_cut_a_body_stalled_call_with_deadline_exceeded() {
    let request = StatusRequest::new();
    let client = DapiClient::new(
        "http://127.0.0.1:20001"
            .parse()
            .expect("valid address list"),
        settings(0),
    );

    let error = client
        .execute(request, RequestSettings::default())
        .await
        .expect_err("the only node never completes its response");

    match error.inner {
        DapiClientError::Transport(TransportError::Grpc(status)) => {
            assert_eq!(
                status.code(),
                Code::DeadlineExceeded,
                "unexpected status: {status:?}"
            )
        }
        other => panic!("expected a gRPC DeadlineExceeded, got {other:?}"),
    }
}

#[tokio::test]
async fn should_fail_over_when_a_node_stalls_its_response_body() {
    let request = StatusRequest::new();
    let address_list: AddressList = "http://127.0.0.1:20001,http://127.0.0.1:20002"
        .parse()
        .expect("valid address list");
    let client = DapiClient::new(address_list, settings(5));

    let started = tokio::time::Instant::now();
    let response = client
        .execute(request.clone(), RequestSettings::default())
        .await
        .expect("the other node must answer");
    let elapsed = started.elapsed();

    let stalled_uri = request.stalled_node().expect("a node was tried first");
    assert_eq!(response.retries, 1);
    assert_ne!(response.address.uri(), &stalled_uri);
    assert!(
        elapsed >= ATTEMPT_TIMEOUT && elapsed < Duration::from_secs(2),
        "the stalled attempt must end at its deadline, took {elapsed:?}"
    );
    let stalled_node = Address::try_from(stalled_uri).expect("valid address");
    assert!(
        client.address_list().is_banned(&stalled_node),
        "the node that stalled its response body must be banned"
    );
}
