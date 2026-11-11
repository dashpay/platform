//! A malformed request belongs to its caller; a valid frame still reaches the service.

#![cfg(all(feature = "core", feature = "server", not(target_arch = "wasm32")))]

use bytes::Bytes;
use dapi_grpc::core::v0::core_server::{Core, CoreServer};
use dapi_grpc::core::v0::{
    BlockHeadersWithChainLocksRequest, BlockHeadersWithChainLocksResponse,
    BroadcastTransactionRequest, BroadcastTransactionResponse, GetBestBlockHeightRequest,
    GetBestBlockHeightResponse, GetBlockRequest, GetBlockResponse, GetBlockchainStatusRequest,
    GetBlockchainStatusResponse, GetEstimatedTransactionFeeRequest,
    GetEstimatedTransactionFeeResponse, GetMasternodeStatusRequest, GetMasternodeStatusResponse,
    GetTransactionRequest, GetTransactionResponse, MasternodeListRequest, MasternodeListResponse,
    TransactionsWithProofsRequest, TransactionsWithProofsResponse,
};
use dapi_grpc::drive::v0::drive_internal_server::{DriveInternal, DriveInternalServer};
use dapi_grpc::drive::v0::{GetProofsRequest, GetProofsResponse};
use dapi_grpc::tonic::codegen::http::{HeaderMap, Request as HttpRequest};
use dapi_grpc::tonic::transport::Server;
use dapi_grpc::tonic::{Code, Request, Response, Status};
use dapi_grpc::Message;
use h2::client::SendRequest;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::task::JoinHandle;
use tokio_stream::{Iter, StreamExt};

type ResponseStream<T> = Iter<std::vec::IntoIter<Result<T, Status>>>;

#[derive(Clone, Default)]
struct RecordingService(Arc<AtomicUsize>);

impl RecordingService {
    fn response<T: Default>(&self) -> Result<Response<T>, Status> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(Response::new(T::default()))
    }

    fn stream<T: Default>(&self) -> Result<Response<ResponseStream<T>>, Status> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(Response::new(tokio_stream::iter(vec![Ok(T::default())])))
    }
}

#[dapi_grpc::tonic::async_trait]
impl Core for RecordingService {
    async fn get_blockchain_status(
        &self,
        _: Request<GetBlockchainStatusRequest>,
    ) -> Result<Response<GetBlockchainStatusResponse>, Status> {
        self.response()
    }

    async fn get_masternode_status(
        &self,
        _: Request<GetMasternodeStatusRequest>,
    ) -> Result<Response<GetMasternodeStatusResponse>, Status> {
        self.response()
    }

    async fn get_block(
        &self,
        _: Request<GetBlockRequest>,
    ) -> Result<Response<GetBlockResponse>, Status> {
        self.response()
    }

    async fn get_best_block_height(
        &self,
        _: Request<GetBestBlockHeightRequest>,
    ) -> Result<Response<GetBestBlockHeightResponse>, Status> {
        self.response()
    }

    async fn broadcast_transaction(
        &self,
        _: Request<BroadcastTransactionRequest>,
    ) -> Result<Response<BroadcastTransactionResponse>, Status> {
        self.response()
    }

    async fn get_transaction(
        &self,
        _: Request<GetTransactionRequest>,
    ) -> Result<Response<GetTransactionResponse>, Status> {
        self.response()
    }

    async fn get_estimated_transaction_fee(
        &self,
        _: Request<GetEstimatedTransactionFeeRequest>,
    ) -> Result<Response<GetEstimatedTransactionFeeResponse>, Status> {
        self.response()
    }

    type subscribeToBlockHeadersWithChainLocksStream =
        ResponseStream<BlockHeadersWithChainLocksResponse>;
    type subscribeToTransactionsWithProofsStream = ResponseStream<TransactionsWithProofsResponse>;
    type subscribeToMasternodeListStream = ResponseStream<MasternodeListResponse>;

    async fn subscribe_to_block_headers_with_chain_locks(
        &self,
        _: Request<BlockHeadersWithChainLocksRequest>,
    ) -> Result<Response<Self::subscribeToBlockHeadersWithChainLocksStream>, Status> {
        self.stream()
    }

    async fn subscribe_to_transactions_with_proofs(
        &self,
        _: Request<TransactionsWithProofsRequest>,
    ) -> Result<Response<Self::subscribeToTransactionsWithProofsStream>, Status> {
        self.stream()
    }

    async fn subscribe_to_masternode_list(
        &self,
        _: Request<MasternodeListRequest>,
    ) -> Result<Response<Self::subscribeToMasternodeListStream>, Status> {
        self.stream()
    }
}

#[dapi_grpc::tonic::async_trait]
impl DriveInternal for RecordingService {
    async fn get_proofs(
        &self,
        _: Request<GetProofsRequest>,
    ) -> Result<Response<GetProofsResponse>, Status> {
        self.response()
    }
}

struct Node {
    sender: SendRequest<Bytes>,
    calls: Arc<AtomicUsize>,
    server: JoinHandle<()>,
    connection: JoinHandle<()>,
}

impl Drop for Node {
    fn drop(&mut self) {
        self.server.abort();
        self.connection.abort();
    }
}

struct WireResponse {
    status: Status,
    data: Vec<u8>,
    headers: HeaderMap,
}

impl Node {
    async fn new() -> Self {
        let (client_io, server_io) = tokio::io::duplex(64 * 1024);
        let service = RecordingService::default();
        let calls = Arc::clone(&service.0);
        let server = tokio::spawn(async move {
            let incoming = tokio_stream::iter([Ok::<_, std::io::Error>(server_io)])
                .chain(tokio_stream::pending());
            Server::builder()
                .add_service(CoreServer::new(service.clone()).max_decoding_message_size(512 * 1024))
                .add_service(DriveInternalServer::new(service))
                .serve_with_incoming(incoming)
                .await
                .expect("serve in-memory generated servers");
        });
        let (sender, connection) = h2::client::handshake(client_io)
            .await
            .expect("HTTP/2 handshake");
        let connection = tokio::spawn(async move {
            connection.await.expect("in-memory HTTP/2 connection");
        });
        Self {
            sender,
            calls,
            server,
            connection,
        }
    }

    async fn request(&self, path: &str, frame: Vec<u8>, encoding: Option<&str>) -> WireResponse {
        let mut request = HttpRequest::builder()
            .method("POST")
            .uri(format!("http://generated.in-memory{path}"))
            .header("content-type", "application/grpc")
            .header("te", "trailers");
        if let Some(encoding) = encoding {
            request = request.header("grpc-encoding", encoding);
        }
        let request = request.body(()).expect("wire request");
        let mut sender = self
            .sender
            .clone()
            .ready()
            .await
            .expect("ready HTTP/2 client");
        let (response, mut body) = sender
            .send_request(request, frame.is_empty())
            .expect("send headers");
        if !frame.is_empty() {
            body.send_data(Bytes::from(frame), true)
                .expect("send framed payload");
        }
        let response = response.await.expect("wire response");
        let headers = response.headers().clone();
        let mut body = response.into_body();
        let mut data = Vec::new();
        while let Some(chunk) = body.data().await {
            let chunk = chunk.expect("response bytes");
            body.flow_control()
                .release_capacity(chunk.len())
                .expect("release response capacity");
            data.extend_from_slice(&chunk);
        }
        let trailers = body.trailers().await.expect("response trailers");
        let status = Status::from_header_map(&headers)
            .or_else(|| trailers.as_ref().and_then(Status::from_header_map))
            .expect("gRPC status");
        WireResponse {
            status,
            data,
            headers,
        }
    }
}

fn frame(payload: &[u8]) -> Vec<u8> {
    let mut frame = vec![0];
    frame.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    frame.extend_from_slice(payload);
    frame
}

const CORE_UNARY: &str = "/org.dash.platform.dapi.v0.Core/getTransaction";
const CORE_STREAM: &str = "/org.dash.platform.dapi.v0.Core/subscribeToTransactionsWithProofs";
const DRIVE_UNARY: &str = "/org.dash.platform.drive.v0.DriveInternal/getProofs";

#[tokio::test]
async fn should_refuse_malformed_caller_protobuf_without_invoking_generated_services() {
    let node = Node::new().await;
    let mut wrong = Vec::new();
    for path in [CORE_UNARY, CORE_STREAM, DRIVE_UNARY] {
        for payload in [&[0][..], &[0x80], &[0x0a, 0x02, 0x08]] {
            let response = node.request(path, frame(payload), None).await;
            if response.status.code() != Code::InvalidArgument {
                wrong.push(format!(
                    "{path} {payload:02x?}: {:?}: {}",
                    response.status.code(),
                    response.status.message()
                ));
            }
            assert!(response
                .status
                .message()
                .starts_with("failed to decode Protobuf message: "));
            assert!(
                response.data.is_empty(),
                "a refused stream must emit no item"
            );
            assert!(
                response.headers.contains_key("grpc-status"),
                "a decode refusal is trailers-only"
            );
        }
    }
    let invalid_utf8 = node
        .request(CORE_UNARY, frame(&[0x0a, 0x01, 0xff]), None)
        .await;
    assert!(invalid_utf8
        .status
        .message()
        .starts_with("failed to decode Protobuf message: "));
    assert!(invalid_utf8.data.is_empty());
    assert!(invalid_utf8.headers.contains_key("grpc-status"));
    if invalid_utf8.status.code() != Code::InvalidArgument {
        wrong.push(format!("invalid UTF-8: {:?}", invalid_utf8.status.code()));
    }
    assert_eq!(
        node.calls.load(Ordering::SeqCst),
        0,
        "no malformed request reaches a handler"
    );

    let valid = node
        .request(
            CORE_UNARY,
            frame(
                &GetTransactionRequest {
                    id: "transaction".to_owned(),
                }
                .encode_to_vec(),
            ),
            None,
        )
        .await;
    assert_eq!(
        valid.status.code(),
        Code::Ok,
        "the same route accepts valid protobuf"
    );
    assert_eq!(node.calls.load(Ordering::SeqCst), 1);
    assert!(wrong.is_empty(), "caller protobuf statuses: {wrong:#?}");
}

#[tokio::test]
async fn should_preserve_valid_unknown_fields_and_stream_termination() {
    let node = Node::new().await;
    let mut request = GetTransactionRequest {
        id: "transaction".to_owned(),
    }
    .encode_to_vec();
    request.extend_from_slice(&[0xf8, 0x07, 0x01]);
    let response = node.request(CORE_UNARY, frame(&request), None).await;
    assert_eq!(response.status.code(), Code::Ok);
    assert_eq!(
        GetTransactionResponse::decode(&response.data[5..]).expect("valid response"),
        GetTransactionResponse::default()
    );

    let response = node
        .request(
            CORE_STREAM,
            frame(&TransactionsWithProofsRequest::default().encode_to_vec()),
            None,
        )
        .await;
    assert_eq!(response.status.code(), Code::Ok);
    assert_eq!(
        response.data.len(),
        5,
        "one empty protobuf item and clean stream completion"
    );
    let response = node
        .request(
            DRIVE_UNARY,
            frame(&GetProofsRequest::default().encode_to_vec()),
            None,
        )
        .await;
    assert_eq!(response.status.code(), Code::Ok);
    assert_eq!(
        GetProofsResponse::decode(&response.data[5..]).expect("valid response"),
        GetProofsResponse::default()
    );
    assert_eq!(node.calls.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn should_preserve_framing_compression_and_message_limit_errors() {
    let node = Node::new().await;
    let cases = [
        (vec![1, 0, 0, 0, 0], None, Code::Internal),
        (vec![2, 0, 0, 0, 0], None, Code::Internal),
        (vec![0, 0, 0, 0, 2, 0], None, Code::Internal),
        (vec![], None, Code::Internal),
        (frame(&[]), Some("unsupported"), Code::Unimplemented),
        (frame(&[]), Some("gzip"), Code::Unimplemented),
        (vec![0, 0, 8, 0, 1], None, Code::OutOfRange),
    ];
    for (frame, encoding, expected) in cases {
        let response = node.request(CORE_UNARY, frame, encoding).await;
        assert_eq!(
            response.status.code(),
            expected,
            "{}",
            response.status.message()
        );
        assert!(response.data.is_empty());
    }
    assert_eq!(node.calls.load(Ordering::SeqCst), 0);
}
