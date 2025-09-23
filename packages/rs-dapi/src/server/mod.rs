mod grpc;
mod health;
mod jsonrpc;
mod rest;
mod state;

use std::sync::Arc;
use tracing::{error, info, warn};

use crate::clients::{CoreClient, DriveClient, TenderdashClient, traits::TenderdashClientTrait};
use crate::config::Config;
use crate::error::{DAPIResult, DapiError};
use crate::logging::AccessLogger;
use crate::protocol::{JsonRpcTranslator, RestTranslator};
use crate::services::{CoreServiceImpl, PlatformServiceImpl, StreamingServiceImpl};

pub struct DapiServer {
    config: Arc<Config>,
    core_service: Arc<CoreServiceImpl>,
    platform_service: Arc<PlatformServiceImpl>,
    rest_translator: Arc<RestTranslator>,
    jsonrpc_translator: Arc<JsonRpcTranslator>,
    access_logger: Option<AccessLogger>,
}

impl DapiServer {
    pub async fn new(config: Arc<Config>, access_logger: Option<AccessLogger>) -> DAPIResult<Self> {
        let drive_client = DriveClient::new(&config.dapi.drive.uri)
            .await
            .map_err(|e| DapiError::Client(format!("Failed to create Drive client: {}", e)))?;

        let tenderdash_client: Arc<dyn TenderdashClientTrait> = Arc::new(
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

        let rest_translator = Arc::new(RestTranslator::new());
        let jsonrpc_translator = Arc::new(JsonRpcTranslator::new());

        Ok(Self {
            config,
            platform_service: Arc::new(platform_service),
            core_service: Arc::new(core_service),
            rest_translator,
            jsonrpc_translator,
            access_logger,
        })
    }

    pub async fn new_with_mocks(
        config: Arc<Config>,
        access_logger: Option<AccessLogger>,
    ) -> DAPIResult<Self> {
        use crate::clients::mock::MockTenderdashClient;

        info!("Creating DAPI server with mock clients for testing");

        let drive_client = DriveClient::new("http://localhost:3005")
            .await
            .map_err(|e| DapiError::Client(format!("Mock Drive client creation failed: {}", e)))?;

        let tenderdash_client: Arc<dyn TenderdashClientTrait> =
            Arc::new(MockTenderdashClient::new());

        let core_client = CoreClient::new(
            config.dapi.core.rpc_url.clone(),
            config.dapi.core.rpc_user.clone(),
            config.dapi.core.rpc_pass.clone().into(),
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
            CoreServiceImpl::new(streaming_service.clone(), config.clone(), core_client).await;

        let rest_translator = Arc::new(RestTranslator::new());
        let jsonrpc_translator = Arc::new(JsonRpcTranslator::new());

        Ok(Self {
            config,
            platform_service: Arc::new(platform_service),
            core_service: Arc::new(core_service),
            rest_translator,
            jsonrpc_translator,
            access_logger,
        })
    }

    pub async fn new_with_fallback(
        config: Arc<Config>,
        access_logger: Option<AccessLogger>,
    ) -> DAPIResult<Self> {
        match Self::new(config.clone(), access_logger.clone()).await {
            Ok(server) => {
                info!("DAPI server created with real clients");
                Ok(server)
            }
            Err(DapiError::ServerUnavailable(_uri, msg)) => {
                warn!(
                    "Upstream server unavailable, falling back to mock clients: {}",
                    msg
                );
                Self::new_with_mocks(config, access_logger).await
            }
            Err(DapiError::Client(msg)) if msg.contains("Failed to connect") => {
                warn!(
                    "Client connection failed, falling back to mock clients: {}",
                    msg
                );
                Self::new_with_mocks(config, access_logger).await
            }
            Err(DapiError::Transport(_)) => {
                warn!("Transport error occurred, falling back to mock clients");
                Self::new_with_mocks(config, access_logger).await
            }
            Err(e) => Err(e),
        }
    }

    pub async fn run(self) -> DAPIResult<()> {
        info!("Starting DAPI server...");

        let grpc_server = self.start_unified_grpc_server();
        let rest_server = self.start_rest_server();
        let jsonrpc_server = self.start_jsonrpc_server();
        let health_server = self.start_health_server();

        tokio::select! {
            result = grpc_server => {
                error!("gRPC server stopped: {:?}", result);
                result
            },
            result = rest_server => {
                error!("REST server stopped: {:?}", result);
                result
            },
            result = jsonrpc_server => {
                error!("JSON-RPC server stopped: {:?}", result);
                result
            },
            result = health_server => {
                error!("Health check server stopped: {:?}", result);
                result
            },
        }
    }
}
