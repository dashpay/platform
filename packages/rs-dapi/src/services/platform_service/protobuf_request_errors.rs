//! Test the public request decoder and real Drive forwarding without background connections.

use super::{MAX_PENDING_STATE_TRANSITION_WAITS, PlatformServiceImpl, ShieldedProofFailureBudget};
use crate::cache::LruResponseCache;
use crate::clients::TenderdashClient;
use crate::clients::drive_client::{DriveChannel, DriveClient};
use crate::config::Config;
use crate::services::streaming_service::SubscriberManager;
use crate::sync::Workers;
use bytes::{BufMut, Bytes};
use dapi_grpc::Message;
use dapi_grpc::mock::Mockable;
use dapi_grpc::platform::v0::get_data_contracts_request::GetDataContractsRequestV0;
use dapi_grpc::platform::v0::get_data_contracts_response::get_data_contracts_response_v0::Result as ContractsResult;
use dapi_grpc::platform::v0::get_data_contracts_response::{
    DataContractEntry, DataContracts, GetDataContractsResponseV0, Version,
};
use dapi_grpc::platform::v0::platform_client::PlatformClient;
use dapi_grpc::platform::v0::platform_server::PlatformServer;
use dapi_grpc::platform::v0::{GetDataContractsRequest, GetDataContractsResponse};
use dapi_grpc::tonic::client::Grpc;
use dapi_grpc::tonic::codec::{BufferSettings, Codec, EncodeBuf, Encoder};
use dapi_grpc::tonic::codegen::http::{
    HeaderMap, HeaderValue, Response as HttpResponse, uri::PathAndQuery,
};
use dapi_grpc::tonic::transport::{Channel, Endpoint, Server};
use dapi_grpc::tonic::{Code, Request, Response, Status};
use dapi_grpc::tonic_prost::{ProstCodec, ProstDecoder};
use hyper_util::rt::TokioIo;
use rs_dapi_client::transport::{
    AppliedRequestSettings, BoxFuture, TransportClient, TransportError, TransportRequest,
};
use rs_dapi_client::{
    AddressList, CanRetry, ConnectionPool, DapiClient, DapiClientError, DapiRequestExecutor,
    RequestSettings, Uri,
};
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::task::JoinHandle;
use tokio_stream::StreamExt;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;

#[derive(Clone, Copy)]
enum DriveReply {
    Valid,
    Corrupt,
    Fault(Code),
}

struct Harness {
    channel: Channel,
    seen: Arc<Mutex<Vec<GetDataContractsRequest>>>,
    server: JoinHandle<()>,
    drive: JoinHandle<()>,
}

impl Drop for Harness {
    fn drop(&mut self) {
        self.server.abort();
        self.drive.abort();
    }
}

fn channel(io: tokio::io::DuplexStream, uri: &'static str) -> Channel {
    let mut io = Some(io);
    Endpoint::from_static(uri)
        .timeout(Duration::from_secs(5))
        .connect_with_connector_lazy(tower::service_fn(move |_: Uri| {
            let io = io.take();
            async move {
                io.map(TokioIo::new)
                    .ok_or_else(|| std::io::Error::other("single-use fixture channel"))
            }
        }))
}

fn frame(payload: &[u8]) -> Bytes {
    let mut bytes = vec![0];
    bytes.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    bytes.extend_from_slice(payload);
    Bytes::from(bytes)
}

impl Harness {
    async fn new(reply: DriveReply) -> Self {
        let (drive_io, fake_io) = tokio::io::duplex(64 * 1024);
        let seen = Arc::new(Mutex::new(Vec::new()));
        let requests = Arc::clone(&seen);
        let drive = tokio::spawn(async move {
            let mut connection = h2::server::handshake(fake_io)
                .await
                .expect("fake Drive handshake");
            while let Some(request) = connection.accept().await {
                let (request, mut response) = request.expect("fake Drive request");
                let seen = Arc::clone(&requests);
                tokio::spawn(async move {
                    assert!(request.uri().path().ends_with("/getDataContracts"));
                    let mut body = request.into_body();
                    let mut bytes = Vec::new();
                    while let Some(data) = body.data().await {
                        let data = data.expect("forwarded request bytes");
                        body.flow_control()
                            .release_capacity(data.len())
                            .expect("request capacity");
                        bytes.extend_from_slice(&data);
                    }
                    let request = GetDataContractsRequest::decode(&bytes[5..])
                        .expect("DAPI forwards valid protobuf");
                    seen.lock().expect("recording requests").push(request);
                    let mut headers = HttpResponse::builder()
                        .status(200)
                        .header("content-type", "application/grpc");
                    if let DriveReply::Fault(code) = reply {
                        headers = headers.header("grpc-status", (code as i32).to_string());
                        response
                            .send_response(headers.body(()).expect("fault response"), true)
                            .expect("send node fault");
                        return;
                    }
                    let mut stream = response
                        .send_response(headers.body(()).expect("response"), false)
                        .expect("send headers");
                    let payload = match reply {
                        DriveReply::Corrupt => vec![0],
                        DriveReply::Valid => GetDataContractsResponse {
                            version: Some(Version::V0(GetDataContractsResponseV0 {
                                result: Some(ContractsResult::DataContracts(DataContracts {
                                    data_contract_entries: vec![DataContractEntry {
                                        identifier: vec![1; 32],
                                        data_contract: None,
                                    }],
                                })),
                                metadata: None,
                            })),
                        }
                        .encode_to_vec(),
                        DriveReply::Fault(_) => unreachable!(),
                    };
                    stream
                        .send_data(frame(&payload), false)
                        .expect("send response bytes");
                    let mut trailers = HeaderMap::new();
                    trailers.insert("grpc-status", HeaderValue::from_static("0"));
                    // A corrupt response may make the receiver cancel before its trailers.
                    let _ = stream.send_trailers(trailers);
                });
            }
        });

        let traced: DriveChannel = ServiceBuilder::new()
            .layer(TraceLayer::new_for_http())
            .service(channel(drive_io, "http://drive.in-memory"));
        let config = Arc::new(Config::default());
        let tenderdash_client = Arc::new(TenderdashClient::without_test_connection());
        let service = PlatformServiceImpl {
            drive_client: DriveClient::from_test_channel(traced),
            websocket_client: tenderdash_client.websocket_client(),
            tenderdash_client,
            config: Arc::clone(&config),
            platform_cache: LruResponseCache::with_capacity(
                "protobuf-request-test",
                config.dapi.platform_cache_bytes,
            ),
            subscriber_manager: Arc::new(SubscriberManager::new()),
            state_transition_wait_permits: Arc::new(Semaphore::new(
                MAX_PENDING_STATE_TRANSITION_WAITS,
            )),
            shielded_proof_failure_budget: Arc::new(ShieldedProofFailureBudget::default()),
            workers: Workers::new(),
        };
        let (client_io, server_io) = tokio::io::duplex(64 * 1024);
        let server = tokio::spawn(async move {
            let incoming = tokio_stream::iter([Ok::<_, std::io::Error>(server_io)])
                .chain(tokio_stream::pending());
            Server::builder()
                .add_service(PlatformServer::new(service).max_decoding_message_size(128 * 1024))
                .serve_with_incoming(incoming)
                .await
                .expect("serve actual DAPI service");
        });
        Self {
            channel: channel(client_io, "http://dapi.in-memory"),
            seen,
            server,
            drive,
        }
    }

    async fn raw_request(
        &self,
        payload: &[u8],
    ) -> Result<Response<GetDataContractsResponse>, Status> {
        let mut client = Grpc::new(self.channel.clone());
        client.ready().await.expect("ready in-memory DAPI channel");
        client
            .unary(
                Request::new(Bytes::copy_from_slice(payload)),
                PathAndQuery::from_static("/org.dash.platform.dapi.v0.Platform/getDataContracts"),
                PayloadCodec::<GetDataContractsResponse>(PhantomData),
            )
            .await
    }

    async fn valid_request(&self) -> Result<Response<GetDataContractsResponse>, Status> {
        PlatformClient::new(self.channel.clone())
            .get_data_contracts(GetDataContractsRequest::from(GetDataContractsRequestV0 {
                ids: vec![vec![1; 32]],
                prove: false,
            }))
            .await
    }
}

struct PayloadCodec<T>(PhantomData<T>);
struct PayloadEncoder;

impl Encoder for PayloadEncoder {
    type Item = Bytes;
    type Error = Status;

    fn encode(&mut self, bytes: Bytes, buffer: &mut EncodeBuf<'_>) -> Result<(), Status> {
        buffer.put_slice(&bytes);
        Ok(())
    }
}

impl<T: Message + Default + Send + 'static> Codec for PayloadCodec<T> {
    type Encode = Bytes;
    type Decode = T;
    type Encoder = PayloadEncoder;
    type Decoder = ProstDecoder<T>;

    fn encoder(&mut self) -> PayloadEncoder {
        PayloadEncoder
    }
    fn decoder(&mut self) -> ProstDecoder<T> {
        ProstCodec::<T, T>::raw_decoder(BufferSettings::default())
    }
}

#[tokio::test]
async fn should_refuse_caller_protobuf_at_dapi_before_forwarding_to_drive() {
    let harness = Harness::new(DriveReply::Valid).await;
    let mut wrong = Vec::new();
    for payload in [&[0][..], &[0x80], &[0x0a, 0x02, 0x08], &[0x08, 0x01]] {
        let status = harness
            .raw_request(payload)
            .await
            .expect_err("malformed request");
        assert!(
            status
                .message()
                .starts_with("failed to decode Protobuf message: ")
        );
        if status.code() != Code::InvalidArgument {
            wrong.push(format!(
                "{payload:02x?}: {:?}: {}",
                status.code(),
                status.message()
            ));
        }
    }
    assert!(
        harness.seen.lock().expect("recorded requests").is_empty(),
        "malformed caller bytes never reach Drive"
    );
    let response = harness
        .valid_request()
        .await
        .expect("valid request reaches fake Drive")
        .into_inner();
    assert!(response.version.is_some());
    assert_eq!(harness.seen.lock().expect("recorded requests").len(), 1);
    assert!(
        wrong.is_empty(),
        "public DAPI caller protobuf statuses: {wrong:#?}"
    );
}

#[tokio::test]
async fn should_keep_corrupt_drive_response_internal_through_actual_dapi_forwarding() {
    let harness = Harness::new(DriveReply::Corrupt).await;
    let status = harness
        .valid_request()
        .await
        .expect_err("corrupt Drive response");
    assert_eq!(status.code(), Code::Internal);
    assert!(
        status
            .message()
            .starts_with("failed to decode Protobuf message: ")
    );
    assert!(
        status.can_retry(),
        "a corrupt node response is still a node failure"
    );
    assert_eq!(harness.seen.lock().expect("recorded requests").len(), 1);
    assert_node_failover(status).await;
}

#[tokio::test]
async fn should_preserve_actual_dapi_request_cap_and_node_faults() {
    let harness = Harness::new(DriveReply::Valid).await;
    let status = harness
        .raw_request(&vec![0; 128 * 1024 + 1])
        .await
        .expect_err("oversized frame");
    assert_eq!(status.code(), Code::OutOfRange);
    assert!(harness.seen.lock().expect("recorded requests").is_empty());
    for code in [Code::Internal, Code::DataLoss, Code::Unavailable] {
        let harness = Harness::new(DriveReply::Fault(code)).await;
        let status = harness.valid_request().await.expect_err("Drive fault");
        assert_eq!(status.code(), code);
        assert_node_failover(status).await;
    }
}

#[derive(Debug)]
struct PoolNode {
    uri: Uri,
}

impl TransportClient for PoolNode {
    fn with_uri(uri: Uri, _: &ConnectionPool) -> Result<Self, TransportError> {
        Ok(Self { uri })
    }
    fn with_uri_and_settings(
        uri: Uri,
        _: &AppliedRequestSettings,
        pool: &ConnectionPool,
    ) -> Result<Self, TransportError> {
        Self::with_uri(uri, pool)
    }
}

/// Replay the status delivered by the wire fixture through the existing executor policy.
#[derive(Clone, Debug)]
struct DeliveredStatus {
    status: Status,
    attempts: Arc<Mutex<Vec<Uri>>>,
}

impl Mockable for DeliveredStatus {}

impl TransportRequest for DeliveredStatus {
    type Client = PoolNode;
    type Response = GetDataContractsResponse;
    const SETTINGS_OVERRIDES: RequestSettings = RequestSettings::default();

    fn method_name(&self) -> &'static str {
        "get_data_contracts"
    }
    fn execute_transport<'c>(
        self,
        client: &'c mut PoolNode,
        _: &AppliedRequestSettings,
    ) -> BoxFuture<'c, Result<Self::Response, TransportError>> {
        let first = {
            let mut attempts = self.attempts.lock().expect("request attempts");
            attempts.push(client.uri.clone());
            attempts.len() == 1
        };
        Box::pin(async move {
            if first {
                Err(TransportError::Grpc(self.status))
            } else {
                Ok(GetDataContractsResponse::default())
            }
        })
    }
}

fn executor() -> DapiClient {
    let mut addresses = AddressList::with_settings(Duration::from_secs(3600));
    for port in [21001, 21002] {
        addresses.add(
            format!("http://127.0.0.1:{port}")
                .parse()
                .expect("synthetic address"),
        );
    }
    DapiClient::new(
        addresses,
        RequestSettings {
            retries: Some(1),
            ..RequestSettings::default()
        },
    )
}

async fn assert_node_failover(status: Status) {
    let request = DeliveredStatus {
        status,
        attempts: Default::default(),
    };
    let client = executor();
    let response = client
        .execute(request.clone(), RequestSettings::default())
        .await
        .expect("next node answers");
    let attempts = request.attempts.lock().expect("attempts").clone();
    assert_eq!(attempts.len(), 2);
    assert_ne!(attempts[0], attempts[1]);
    let failed = attempts[0].clone().try_into().expect("failed address");
    assert!(client.address_list().is_banned(&failed));
    assert_eq!(response.retries, 1);
    assert_eq!(response.address.uri(), &attempts[1]);
}

#[tokio::test]
async fn should_keep_nodes_available_for_served_malformed_caller_requests() {
    let harness = Harness::new(DriveReply::Valid).await;
    let status = harness
        .raw_request(&[0])
        .await
        .expect_err("malformed caller request");
    let request = DeliveredStatus {
        status,
        attempts: Default::default(),
    };
    let client = executor();
    let error = client
        .execute(request.clone(), RequestSettings::default())
        .await
        .expect_err("caller error stops");
    let DapiClientError::Transport(TransportError::Grpc(status)) = error.inner else {
        panic!("expected served gRPC refusal");
    };
    assert_eq!(status.code(), Code::InvalidArgument);
    assert_eq!(error.retries, 0);
    assert_eq!(request.attempts.lock().expect("attempts").len(), 1);
    assert!(
        client
            .address_list()
            .ban_info()
            .iter()
            .all(|ban| !ban.banned)
    );
}
