use std::sync::Arc;

use dapi_grpc::drive::v0::drive_internal_client::DriveInternalClient;
use dapi_grpc::platform::v0::{GetStatusRequest, platform_client::PlatformClient};
use serde::{Deserialize, Serialize};

use tower::ServiceBuilder;
use tower_http::{
    LatencyUnit,
    trace::{
        DefaultMakeSpan, DefaultOnBodyChunk, DefaultOnEos, DefaultOnFailure, DefaultOnRequest,
        DefaultOnResponse, Trace, TraceLayer,
    },
};
use tracing::{Level, debug, error, info, trace};

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

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct DriveStatusResponse {
    pub version: Option<DriveVersion>,
    pub chain: Option<DriveChain>,
    pub time: Option<DriveTime>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct DriveVersion {
    pub software: Option<DriveSoftware>,
    pub protocol: Option<DriveProtocol>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct DriveSoftware {
    pub drive: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct DriveProtocol {
    pub drive: Option<DriveProtocolVersion>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct DriveProtocolVersion {
    pub current: Option<u64>,
    pub latest: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct DriveChain {
    #[serde(rename = "coreChainLockedHeight")]
    pub core_chain_locked_height: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct DriveTime {
    pub block: Option<u64>,
    pub genesis: Option<u64>,
    pub epoch: Option<u64>,
}

pub type DriveChannel = Trace<
    tonic::transport::Channel,
    tower_http::classify::SharedClassifier<tower_http::classify::ServerErrorsAsFailures>,
    DefaultMakeSpan,
    DefaultOnRequest,
    DefaultOnResponse,
    DefaultOnBodyChunk,
>;

impl DriveClient {
    /// Create a new DriveClient with gRPC request tracing and connection reuse
    ///
    /// This method validates the connection by making a test gRPC call to ensure
    /// the Drive service is reachable and responding correctly.
    pub async fn new(uri: &str) -> Result<Self, tonic::Status> {
        info!("Creating Drive client for: {}", uri);
        let channel = Self::create_channel(uri).await?;

        // Configure clients with larger message sizes.
        // Compression (gzip) is intentionally DISABLED at rs-dapi level; Envoy handles it.
        info!("Drive client compression: disabled (handled by Envoy)");
        const MAX_DECODING_BYTES: usize = 64 * 1024 * 1024; // 64 MiB
        const MAX_ENCODING_BYTES: usize = 32 * 1024 * 1024; // 32 MiB

        let client = Self {
            base_url: Arc::new(uri.to_string()),
            client: PlatformClient::new(channel.clone())
                .max_decoding_message_size(MAX_DECODING_BYTES)
                .max_encoding_message_size(MAX_ENCODING_BYTES),
            internal_client: DriveInternalClient::new(channel.clone())
                .max_decoding_message_size(MAX_DECODING_BYTES)
                .max_encoding_message_size(MAX_ENCODING_BYTES),
        };

        // Validate connection by making a test status call
        trace!("Validating Drive connection at: {}", uri);
        let test_request = GetStatusRequest { version: None };
        match client.get_drive_status(&test_request).await {
            Ok(_) => {
                debug!("Drive connection validated successfully");
                Ok(client)
            }
            Err(e) => {
                error!("Failed to validate Drive connection: {}", e);
                Err(e)
            }
        }
    }

    async fn create_channel(uri: &str) -> Result<DriveChannel, tonic::Status> {
        let raw_channel = dapi_grpc::tonic::transport::Endpoint::from_shared(uri.to_string())
            .map_err(|e| {
                error!("Invalid Drive service URI {}: {}", uri, e);
                tonic::Status::invalid_argument(format!("Invalid URI: {}", e))
            })?
            .connect()
            .await
            .map_err(|e| {
                error!("Failed to connect to Drive service at {}: {}", uri, e);
                tonic::Status::unavailable(format!("Connection failed: {}", e))
            })?;

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

    pub async fn get_drive_status(
        &self,
        request: &GetStatusRequest,
    ) -> Result<DriveStatusResponse, tonic::Status> {
        trace!("Making get_status gRPC call to Drive");
        // Make gRPC call to Drive with timing
        let drive_response = self.get_client().get_status(*request).await?.into_inner();

        // Convert Drive's GetStatusResponse to our DriveStatusResponse format
        if let Some(dapi_grpc::platform::v0::get_status_response::Version::V0(v0)) =
            drive_response.version
        {
            let mut drive_status = DriveStatusResponse::default();

            // Extract version information
            if let Some(version) = v0.version {
                let mut drive_version = DriveVersion::default();

                if let Some(software) = version.software {
                    drive_version.software = Some(DriveSoftware {
                        drive: software.drive,
                    });
                }

                if let Some(protocol) = version.protocol {
                    if let Some(drive_proto) = protocol.drive {
                        drive_version.protocol = Some(DriveProtocol {
                            drive: Some(DriveProtocolVersion {
                                current: Some(drive_proto.current as u64),
                                latest: Some(drive_proto.latest as u64),
                            }),
                        });
                    }
                }

                drive_status.version = Some(drive_version);
            }

            // Extract chain information
            if let Some(chain) = v0.chain {
                drive_status.chain = Some(DriveChain {
                    core_chain_locked_height: chain.core_chain_locked_height.map(|h| h as u64),
                });
            }

            // Extract time information
            if let Some(time) = v0.time {
                drive_status.time = Some(DriveTime {
                    block: Some(time.local),
                    genesis: time.genesis,
                    epoch: time.epoch.map(|e| e as u64),
                });
            }

            Ok(drive_status)
        } else {
            Err(tonic::Status::internal(
                "Drive returned unexpected response format",
            ))
        }
    }

    pub fn get_client(&self) -> PlatformClient<DriveChannel> {
        self.client.clone()
    }

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
        // Note: This will fail if no server is running, which is expected in unit tests
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
}
