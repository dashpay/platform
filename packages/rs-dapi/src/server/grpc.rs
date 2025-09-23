use std::sync::Arc;
use std::time::Duration;
use tracing::info;

use dapi_grpc::core::v0::core_server::CoreServer;
use dapi_grpc::platform::v0::platform_server::PlatformServer;

use crate::error::DAPIResult;

use super::DapiServer;

impl DapiServer {
    pub(super) async fn start_unified_grpc_server(&self) -> DAPIResult<()> {
        let addr = self.config.grpc_server_addr();
        info!(
            "Starting unified gRPC server on {} (Core + Platform services)",
            addr
        );

        let platform_service = self.platform_service.clone();
        let core_service = self.core_service.clone();

        const MAX_DECODING_BYTES: usize = 64 * 1024 * 1024; // 64 MiB
        const MAX_ENCODING_BYTES: usize = 32 * 1024 * 1024; // 32 MiB

        info!("gRPC compression: disabled (handled by Envoy)");

        dapi_grpc::tonic::transport::Server::builder()
            .tcp_keepalive(Some(Duration::from_secs(25)))
            .timeout(Duration::from_secs(120))
            .add_service(
                PlatformServer::new(
                    Arc::try_unwrap(platform_service).unwrap_or_else(|arc| (*arc).clone()),
                )
                .max_decoding_message_size(MAX_DECODING_BYTES)
                .max_encoding_message_size(MAX_ENCODING_BYTES),
            )
            .add_service(
                CoreServer::new(Arc::try_unwrap(core_service).unwrap_or_else(|arc| (*arc).clone()))
                    .max_decoding_message_size(MAX_DECODING_BYTES)
                    .max_encoding_message_size(MAX_ENCODING_BYTES),
            )
            .serve(addr)
            .await?;

        Ok(())
    }
}
