mod grpc;
mod jsonrpc;
mod metrics;
mod state;

use futures::FutureExt;
use std::sync::Arc;
use tracing::{error, info};

use crate::clients::{CoreClient, DriveClient, TenderdashClient};
use crate::config::Config;
use crate::error::{DAPIResult, DapiError};
use crate::logging::AccessLogger;
use crate::protocol::JsonRpcTranslator;
use crate::services::{CoreServiceImpl, PlatformServiceImpl, StreamingServiceImpl};

pub struct DapiServer {
    config: Arc<Config>,
    core_service: CoreServiceImpl,
    platform_service: PlatformServiceImpl,
    jsonrpc_translator: JsonRpcTranslator,
    access_logger: Option<AccessLogger>,
}

impl DapiServer {
    /// Construct the DAPI server by wiring clients, services, and translators from config.
    /// Establishes Drive, Tenderdash, and Core connections while building streaming support.
    /// Returns an error with context when dependencies cannot be initialized.
    pub async fn new(config: Arc<Config>, access_logger: Option<AccessLogger>) -> DAPIResult<Self> {
        let drive_client = DriveClient::new(&config.dapi.drive.uri)
            .await
            .map_err(|e| DapiError::Client(format!("Failed to create Drive client: {}", e)))?;

        let tenderdash_client = Arc::new(
            TenderdashClient::with_websocket(
                &config.dapi.tenderdash.uri,
                &config.dapi.tenderdash.websocket_uri,
            )
            .await?,
        );

        let core_client = CoreClient::new(
            config.dapi.core.rpc_url.clone(),
            config.dapi.core.rpc_user.clone(),
            config.dapi.core.rpc_pass.clone().into(),
            config.dapi.core.cache_bytes,
        )
        .map_err(|e| DapiError::Client(format!("Failed to create Core RPC client: {}", e)))?;

        let streaming_service = Arc::new(StreamingServiceImpl::new(
            drive_client.clone(),
            tenderdash_client.clone(),
            core_client.clone(),
            config.clone(),
        )?);

        let platform_service = PlatformServiceImpl::new(
            drive_client.clone(),
            tenderdash_client.clone(),
            config.clone(),
            streaming_service.subscriber_manager.clone(),
        )
        .await;

        let core_service =
            CoreServiceImpl::new(streaming_service, config.clone(), core_client).await;

        let jsonrpc_translator = JsonRpcTranslator::new();

        Ok(Self {
            config,
            platform_service,
            core_service,
            jsonrpc_translator,
            access_logger,
        })
    }

    /// Run all configured server endpoints and await until one terminates.
    /// gRPC, JSON-RPC, and optional metrics servers are started concurrently.
    /// The first server to exit determines the result returned to the caller.
    pub async fn run(self) -> DAPIResult<()> {
        info!("Starting DAPI server...");

        let grpc_server = self.start_unified_grpc_server();
        let jsonrpc_server = self.start_jsonrpc_server();

        let metrics_server = if self.config.metrics_enabled() {
            self.start_metrics_server().boxed()
        } else {
            futures::future::pending().map(|_: ()| Ok(())).boxed() // Never completes
        };

        // when any of the servers stop, log and return its result
        tokio::select! {
            result = grpc_server => {
                error!("gRPC server stopped: {:?}", result);
                result
            },
            result = jsonrpc_server => {
                error!("JSON-RPC server stopped: {:?}", result);
                result
            },
            result = metrics_server => {
                error!("Metrics server stopped: {:?}", result);
                result
            },
        }
    }
}
