use std::sync::Arc;

use dapi_grpc::drive::v0::drive_internal_client::DriveInternalClient;
use dapi_grpc::platform::v0::{
    GetStatusRequest,
    get_status_response::{self, GetStatusResponseV0},
    platform_client::PlatformClient,
};

use tower::ServiceBuilder;
use tower_http::{
    LatencyUnit,
    trace::{
        DefaultMakeSpan, DefaultOnBodyChunk, DefaultOnEos, DefaultOnFailure, DefaultOnRequest,
        DefaultOnResponse, Trace, TraceLayer,
    },
};
use tracing::{Level, debug, error, info, trace, warn};

/// gRPC client factory for interacting with Dash Platform Drive
///
/// ## Cloning
///
///  This client is designed to be cloned cheaply. No need to use `Arc` or `Rc`.
#[derive(Clone)]
pub struct DriveClient {
    client: PlatformClient<DriveChannel>,
    internal_client: DriveInternalClient<DriveChannel>,
    // base url stored as an Arc for faster cloning
    base_url: Arc<String>,
}

impl std::fmt::Debug for DriveClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DriveClient")
            .field("base_url", &self.base_url)
            .finish()
    }
}

pub type DriveStatusResponse = GetStatusResponseV0;

pub type DriveChannel = Trace<
    tonic::transport::Channel,
    tower_http::classify::SharedClassifier<tower_http::classify::ServerErrorsAsFailures>,
    DefaultMakeSpan,
    DefaultOnRequest,
    DefaultOnResponse,
    DefaultOnBodyChunk,
>;

impl DriveClient {
    /// Create a new DriveClient with gRPC request tracing and connection reuse.
    ///
    /// This method attempts to validate the connection by making a test gRPC call to ensure
    /// the Drive service is reachable and responding correctly. If the Drive
    /// service cannot be reached, the error is logged and the client is still returned so the
    /// caller can operate in a degraded mode while health checks surface the issue.
    pub async fn new(uri: &str) -> Result<Self, tonic::Status> {
        info!("Creating Drive client for: {}", uri);
        let channel = Self::create_channel(uri)?;

        // Configure clients with larger message sizes.
        // Compression (gzip) is intentionally DISABLED at rs-dapi level; Envoy handles it.
        info!("Drive client compression: disabled (handled by Envoy)");
        const MAX_DECODING_BYTES: usize = 64 * 1024 * 1024; // 64 MiB
        // `waitForStateTransitionResult` re-sends the whole transaction to Drive inside a
        // `GetProofsRequest`, so the outbound cap must exceed the largest state transition
        // family cap (`max_contract_code_state_transition_size`, 32 MiB from protocol
        // version 17) plus the protobuf framing around it.
        const MAX_ENCODING_BYTES: usize = 34 * 1024 * 1024; // 34 MiB

        let client = Self {
            base_url: Arc::new(uri.to_string()),
            client: PlatformClient::new(channel.clone())
                .max_decoding_message_size(MAX_DECODING_BYTES)
                .max_encoding_message_size(MAX_ENCODING_BYTES),
            internal_client: DriveInternalClient::new(channel.clone())
                .max_decoding_message_size(MAX_DECODING_BYTES)
                .max_encoding_message_size(MAX_ENCODING_BYTES),
        };

        // Validate connection by making a test status call; log warnings but allow degraded operation.
        trace!("Validating Drive connection at: {}", uri);
        let test_request = GetStatusRequest { version: None };
        match client.get_drive_status(&test_request).await {
            Ok(_) => {
                debug!("Drive connection validated successfully");
            }
            Err(e) => {
                warn!(
                    error = %e,
                    "Failed to validate Drive connection; continuing with degraded health"
                );
            }
        }

        Ok(client)
    }

    /// Build a traced gRPC channel to Drive with error normalization.
    fn create_channel(uri: &str) -> Result<DriveChannel, tonic::Status> {
        let endpoint = dapi_grpc::tonic::transport::Endpoint::from_shared(uri.to_string())
            .map_err(|e| {
                error!("Invalid Drive service URI {}: {}", uri, e);
                tonic::Status::invalid_argument(format!("Invalid URI: {}", e))
            })?;

        let raw_channel = endpoint.connect_lazy();

        let channel: Trace<
            tonic::transport::Channel,
            tower_http::classify::SharedClassifier<tower_http::classify::ServerErrorsAsFailures>,
            DefaultMakeSpan,
            DefaultOnRequest,
            DefaultOnResponse,
            DefaultOnBodyChunk,
        > = ServiceBuilder::new()
            .layer(
                TraceLayer::new_for_http()
                    .make_span_with(DefaultMakeSpan::new().include_headers(true))
                    .on_request(DefaultOnRequest::new().level(Level::TRACE))
                    .on_response(
                        DefaultOnResponse::new()
                            .level(Level::INFO)
                            .latency_unit(LatencyUnit::Micros),
                    )
                    .on_failure(DefaultOnFailure::new().level(Level::WARN))
                    .on_eos(DefaultOnEos::new().level(Level::DEBUG))
                    .on_body_chunk(DefaultOnBodyChunk::new()),
            )
            .service(raw_channel);

        Ok(channel)
    }

    /// Call the Drive `getStatus` endpoint and map the response into simplified structs.
    pub async fn get_drive_status(
        &self,
        request: &GetStatusRequest,
    ) -> Result<DriveStatusResponse, tonic::Status> {
        trace!("Making get_status gRPC call to Drive");
        // Make gRPC call to Drive with timing
        let drive_response = self.get_client().get_status(*request).await?.into_inner();

        // Convert Drive's GetStatusResponse to our DriveStatusResponse format
        if let Some(get_status_response::Version::V0(v0)) = drive_response.version {
            Ok(v0)
        } else {
            Err(tonic::Status::internal(
                "Drive returned unexpected response format",
            ))
        }
    }

    /// Return a clone of the public Platform gRPC client.
    pub fn get_client(&self) -> PlatformClient<DriveChannel> {
        self.client.clone()
    }

    /// Return a clone of the internal Drive gRPC client.
    pub fn get_internal_client(&self) -> DriveInternalClient<DriveChannel> {
        self.internal_client.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_drive_client_tracing_integration() {
        // Test that DriveClient can be created with tracing interceptor
        // Note: This should succeed even if no server is running; connectivity validation logs a warning.
        match DriveClient::new("http://localhost:1443").await {
            Ok(client) => {
                // If connection succeeds, verify the structure
                assert_eq!(client.base_url.to_string(), "http://localhost:1443");
            }
            Err(_) => {
                // Expected when no server is running - this is okay for unit tests
                // The important thing is that the method signature and error handling work
            }
        }

        // Note: In a real integration test with a running Drive instance,
        // you would see tracing logs like:
        // [TRACE] Sending gRPC request
        // [TRACE] gRPC request successful (status: OK, duration: 45ms)
        //
        // The interceptor and log_grpc_result function automatically log:
        // - Request method and timing
        // - Response status and duration
        // - Error classification (technical vs service errors)
    }

    mod message_size_boundary {
        use super::*;
        use dapi_grpc::drive::v0::drive_internal_server::{DriveInternal, DriveInternalServer};
        use dapi_grpc::drive::v0::{GetProofsRequest, GetProofsResponse};
        use dapi_grpc::tonic::transport::Server;
        use dapi_grpc::tonic::transport::server::TcpIncoming;
        use dapi_grpc::tonic::{Request, Response, Status};
        use dpp::version::PlatformVersion;
        use tokio::net::TcpListener;

        /// A `getProofs` server that answers with an empty response and records the size of
        /// the transaction it received.
        struct RecordingDriveInternal {
            received_len: std::sync::Mutex<Option<usize>>,
        }

        #[dapi_grpc::tonic::async_trait]
        impl DriveInternal for RecordingDriveInternal {
            async fn get_proofs(
                &self,
                request: Request<GetProofsRequest>,
            ) -> Result<Response<GetProofsResponse>, Status> {
                *self.received_len.lock().expect("lock") =
                    Some(request.into_inner().state_transition.len());
                Ok(Response::new(GetProofsResponse {
                    proof: None,
                    metadata: None,
                }))
            }
        }

        /// The rs-dapi to Drive client re-sends the whole transaction inside a
        /// `GetProofsRequest`, so its outbound cap decides whether a proof can ever be fetched
        /// for a transition at the family cap. A local server behind the configured client
        /// receives one at the cap intact, and one above the client's own cap never reaches
        /// the server: the encoder refuses it and the stream is reset on the client side
        /// (tonic reports the refusal as a transport error, not as a status the server sent).
        #[tokio::test]
        async fn get_proofs_request_carrying_a_family_cap_transaction_is_sent_intact() {
            let family_cap = PlatformVersion::latest()
                .system_limits
                .max_contract_code_state_transition_size
                .expect("the latest version bounds contract code envelopes")
                as usize;

            let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
            let address = listener.local_addr().expect("local address");
            let service = Arc::new(RecordingDriveInternal {
                received_len: std::sync::Mutex::new(None),
            });
            let server_service = Arc::clone(&service);
            let server = tokio::spawn(async move {
                Server::builder()
                    .add_service(
                        DriveInternalServer::from_arc(server_service)
                            .max_decoding_message_size(64 * 1024 * 1024),
                    )
                    .serve_with_incoming(TcpIncoming::from(listener))
                    .await
                    .expect("test server");
            });

            let client = DriveClient::new(&format!("http://{address}"))
                .await
                .expect("client");
            let mut internal_client = client.get_internal_client();

            internal_client
                .get_proofs(GetProofsRequest {
                    state_transition: vec![7u8; family_cap],
                })
                .await
                .expect("a transaction at the family cap must reach Drive");
            assert_eq!(
                *service.received_len.lock().expect("lock"),
                Some(family_cap)
            );

            internal_client
                .get_proofs(GetProofsRequest {
                    state_transition: vec![7u8; 34 * 1024 * 1024 + 1],
                })
                .await
                .expect_err("a transaction above the client's outbound cap is refused");
            assert_eq!(
                *service.received_len.lock().expect("lock"),
                Some(family_cap),
                "the oversized request must be refused before the server sees it"
            );

            server.abort();
        }
    }
}
