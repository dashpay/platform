use std::time::Duration;
use tracing::info;

use dapi_grpc::core::v0::core_server::CoreServer;
use dapi_grpc::platform::v0::platform_server::PlatformServer;
use tower::layer::util::{Identity, Stack};
use tower::util::Either;

use crate::error::DAPIResult;
use crate::logging::AccessLogLayer;
use crate::metrics::MetricsLayer;

use super::DapiServer;

impl DapiServer {
    /// Start the unified gRPC server that exposes both Platform and Core services.
    /// Configures timeouts, message limits, optional access logging, and then awaits completion.
    /// Returns when the server stops serving.
    pub(super) async fn start_unified_grpc_server(&self) -> DAPIResult<()> {
        let addr = self.config.grpc_server_addr()?;
        info!(
            "Starting unified gRPC server on {} (Core + Platform services)",
            addr
        );

        let platform_service = self.platform_service.clone();
        let core_service = self.core_service.clone();

        const MAX_DECODING_BYTES: usize = 64 * 1024 * 1024; // 64 MiB
        const MAX_ENCODING_BYTES: usize = 32 * 1024 * 1024; // 32 MiB

        info!("gRPC compression: disabled (handled by Envoy)");

        let builder = dapi_grpc::tonic::transport::Server::builder()
            .tcp_keepalive(Some(Duration::from_secs(25)))
            .timeout(Duration::from_secs(120));

        let metrics_layer = MetricsLayer::new();
        let access_layer = if let Some(ref access_logger) = self.access_logger {
            Either::Left(AccessLogLayer::new(access_logger.clone()))
        } else {
            Either::Right(Identity::new())
        };

        let combined_layer = Stack::new(access_layer, metrics_layer);
        let mut builder = builder.layer(combined_layer);

        builder
            .add_service(
                PlatformServer::new(platform_service)
                    .max_decoding_message_size(MAX_DECODING_BYTES)
                    .max_encoding_message_size(MAX_ENCODING_BYTES),
            )
            .add_service(
                CoreServer::new(core_service)
                    .max_decoding_message_size(MAX_DECODING_BYTES)
                    .max_encoding_message_size(MAX_ENCODING_BYTES),
            )
            .serve(addr)
            .await?;

        Ok(())
    }
}
