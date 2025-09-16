use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};

use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tracing::{error, info, warn};

use dapi_grpc::core::v0::core_server::{Core as CoreTrait, CoreServer};
use dapi_grpc::platform::v0::platform_server::{Platform, PlatformServer};

use crate::clients::{CoreClient, DriveClient, TenderdashClient};
use crate::config::Config;
use crate::error::{DAPIResult, DapiError};
use crate::logging::{middleware::AccessLogLayer, AccessLogger};
use crate::protocol::{JsonRpcRequest, JsonRpcTranslator, RestTranslator};
use crate::services::{CoreServiceImpl, PlatformServiceImpl};
use crate::{clients::traits::TenderdashClientTrait, services::StreamingServiceImpl};

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
        // Create clients based on configuration
        // For now, let's use real clients by default
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

        // Create Dash Core RPC client
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

    /// Create a new DapiServer with mock clients for testing
    ///
    /// This method bypasses connection validation and uses mock clients,
    /// making it suitable for unit tests and environments where real
    /// services are not available.
    pub async fn new_with_mocks(
        config: Arc<Config>,
        access_logger: Option<AccessLogger>,
    ) -> DAPIResult<Self> {
        use crate::clients::mock::MockTenderdashClient;

        info!("Creating DAPI server with mock clients for testing");

        // Create real Drive client (it validates connection, but we can handle failure gracefully)
        // For testing, we might want to make this more flexible in the future
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

    /// Create a new DapiServer, falling back to mock clients if connection validation fails
    ///
    /// This method attempts to create real clients first, but if connection validation
    /// fails, it falls back to mock clients and logs a warning. This is useful for
    /// development environments where services may not always be available.
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

        // Streaming service and websocket service auto-starts when created, no need to start it manually

        // Start all servers concurrently
        let grpc_server = self.start_unified_grpc_server();
        let rest_server = self.start_rest_server();
        let jsonrpc_server = self.start_jsonrpc_server();
        let health_server = self.start_health_server();

        // Use tokio::select! to run all servers concurrently
        // If any server fails, the whole application should shut down
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

    async fn start_unified_grpc_server(&self) -> DAPIResult<()> {
        let addr = self.config.grpc_server_addr();
        info!(
            "Starting unified gRPC server on {} (Core + Platform services)",
            addr
        );

        let platform_service = self.platform_service.clone();
        let core_service = self.core_service.clone();

        const MAX_DECODING_BYTES: usize = 64 * 1024 * 1024; // 64 MiB
        const MAX_ENCODING_BYTES: usize = 32 * 1024 * 1024; // 32 MiB

        // NOTE: Compression (gzip) is intentionally DISABLED at rs-dapi level.
        // Envoy handles wire compression at the edge. Keeping it disabled here
        // avoids double-compression overhead.
        info!("gRPC compression: disabled (handled by Envoy)");

        dapi_grpc::tonic::transport::Server::builder()
            .tcp_keepalive(Some(Duration::from_secs(25))) // 25 seconds keepalive
            .timeout(std::time::Duration::from_secs(120)) // 2 minutes timeout
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

    async fn start_rest_server(&self) -> DAPIResult<()> {
        let addr = self.config.rest_gateway_addr();
        info!("Starting REST gateway server on {}", addr);

        let app_state = RestAppState {
            platform_service: Arc::try_unwrap(self.platform_service.clone())
                .unwrap_or_else(|arc| (*arc).clone()),
            core_service: Arc::try_unwrap(self.core_service.clone())
                .unwrap_or_else(|arc| (*arc).clone()),
            translator: self.rest_translator.clone(),
        };

        let mut app = Router::new()
            .route("/v1/platform/status", get(handle_rest_get_status))
            .route(
                "/v1/core/best-block-height",
                get(handle_rest_get_best_block_height),
            )
            .route(
                "/v1/core/transaction/{id}",
                get(handle_rest_get_transaction),
            )
            .route(
                "/v1/core/block/hash/{hash}",
                get(handle_rest_get_block_by_hash),
            )
            .route(
                "/v1/core/block/height/{height}",
                get(handle_rest_get_block_by_height),
            )
            .route(
                "/v1/core/transaction/broadcast",
                post(handle_rest_broadcast_transaction),
            )
            .with_state(app_state);

        // Add access logging middleware if available
        if let Some(ref access_logger) = self.access_logger {
            app = app.layer(
                ServiceBuilder::new()
                    .layer(AccessLogLayer::new(access_logger.clone()))
                    .layer(CorsLayer::permissive()),
            );
        } else {
            app = app.layer(CorsLayer::permissive());
        }

        let listener = TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }

    async fn start_jsonrpc_server(&self) -> DAPIResult<()> {
        let addr = self.config.json_rpc_addr();
        info!("Starting JSON-RPC server on {}", addr);

        let app_state = JsonRpcAppState {
            platform_service: Arc::try_unwrap(self.platform_service.clone())
                .unwrap_or_else(|arc| (*arc).clone()),
            core_service: Arc::try_unwrap(self.core_service.clone())
                .unwrap_or_else(|arc| (*arc).clone()),
            translator: self.jsonrpc_translator.clone(),
        };

        let mut app = Router::new()
            .route("/", post(handle_jsonrpc_request))
            .with_state(app_state);

        // Add access logging middleware if available
        if let Some(ref access_logger) = self.access_logger {
            app = app.layer(
                ServiceBuilder::new()
                    .layer(AccessLogLayer::new(access_logger.clone()))
                    .layer(CorsLayer::permissive()),
            );
        } else {
            app = app.layer(CorsLayer::permissive());
        }

        let listener = TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }

    async fn start_health_server(&self) -> DAPIResult<()> {
        let addr = self.config.health_check_addr();
        info!("Starting health check server on {}", addr);

        let mut app = Router::new()
            .route("/health", get(handle_health))
            .route("/health/ready", get(handle_ready))
            .route("/health/live", get(handle_live))
            .route("/metrics", get(handle_metrics));

        // Add access logging middleware if available
        if let Some(ref access_logger) = self.access_logger {
            app = app.layer(AccessLogLayer::new(access_logger.clone()));
        }

        let listener = TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }
}

#[derive(Clone)]
struct RestAppState {
    platform_service: PlatformServiceImpl,
    core_service: CoreServiceImpl,
    translator: Arc<RestTranslator>,
}

#[derive(Clone)]
struct JsonRpcAppState {
    platform_service: PlatformServiceImpl,
    core_service: CoreServiceImpl,
    translator: Arc<JsonRpcTranslator>,
}

// REST handlers
async fn handle_rest_get_status(
    State(state): State<RestAppState>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // Translate REST request to gRPC
    let grpc_request = match state.translator.translate_get_status().await {
        Ok(req) => req,
        Err(e) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": e.to_string()})),
            ));
        }
    };

    // Call the gRPC service
    let grpc_response = match state
        .platform_service
        .get_status(dapi_grpc::tonic::Request::new(grpc_request))
        .await
    {
        Ok(resp) => resp.into_inner(),
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            ));
        }
    };

    // Translate gRPC response back to REST
    match state
        .translator
        .translate_status_response(grpc_response)
        .await
    {
        Ok(json_response) => Ok(Json(json_response)),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )),
    }
}

async fn handle_rest_get_best_block_height(
    State(state): State<RestAppState>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    use dapi_grpc::core::v0::GetBestBlockHeightRequest;

    let grpc_response = match state
        .core_service
        .get_best_block_height(dapi_grpc::tonic::Request::new(GetBestBlockHeightRequest {}))
        .await
    {
        Ok(resp) => resp.into_inner(),
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            ));
        }
    };

    match state
        .translator
        .translate_best_block_height(grpc_response.height)
        .await
    {
        Ok(json) => Ok(Json(json)),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )),
    }
}

async fn handle_rest_get_transaction(
    State(state): State<RestAppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    use dapi_grpc::core::v0::GetTransactionRequest;

    let grpc_response = match state
        .core_service
        .get_transaction(dapi_grpc::tonic::Request::new(GetTransactionRequest { id }))
        .await
    {
        Ok(resp) => resp.into_inner(),
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            ));
        }
    };

    match state
        .translator
        .translate_transaction_response(grpc_response)
        .await
    {
        Ok(json) => Ok(Json(json)),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )),
    }
}

async fn handle_rest_get_block_by_hash(
    State(state): State<RestAppState>,
    Path(hash): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    use dapi_grpc::core::v0::GetBlockResponse;

    // Build request via translator
    let grpc_req = match state.translator.translate_get_block_by_hash(hash).await {
        Ok(r) => r,
        Err(e) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": e.to_string()})),
            ));
        }
    };

    // Call Core service
    let GetBlockResponse { block } = match state
        .core_service
        .get_block(dapi_grpc::tonic::Request::new(grpc_req))
        .await
    {
        Ok(resp) => resp.into_inner(),
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            ));
        }
    };

    // Translate response
    match state.translator.translate_block_response(block).await {
        Ok(json) => Ok(Json(json)),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )),
    }
}

async fn handle_rest_get_block_by_height(
    State(state): State<RestAppState>,
    Path(height): Path<u32>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    use dapi_grpc::core::v0::GetBlockResponse;

    // Build request via translator
    let grpc_req = match state.translator.translate_get_block_by_height(height).await {
        Ok(r) => r,
        Err(e) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": e.to_string()})),
            ));
        }
    };

    // Call Core service
    let GetBlockResponse { block } = match state
        .core_service
        .get_block(dapi_grpc::tonic::Request::new(grpc_req))
        .await
    {
        Ok(resp) => resp.into_inner(),
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            ));
        }
    };

    // Translate response
    match state.translator.translate_block_response(block).await {
        Ok(json) => Ok(Json(json)),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )),
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct BroadcastTxBody {
    transaction: String,
    #[serde(default)]
    allow_high_fees: Option<bool>,
    #[serde(default)]
    bypass_limits: Option<bool>,
}

async fn handle_rest_broadcast_transaction(
    State(state): State<RestAppState>,
    axum::Json(body): axum::Json<BroadcastTxBody>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    use dapi_grpc::core::v0::BroadcastTransactionRequest;

    let tx_bytes = match hex::decode(&body.transaction) {
        Ok(b) => b,
        Err(e) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": format!("invalid hex transaction: {}", e)})),
            ));
        }
    };

    let req = BroadcastTransactionRequest {
        transaction: tx_bytes,
        allow_high_fees: body.allow_high_fees.unwrap_or(false),
        bypass_limits: body.bypass_limits.unwrap_or(false),
    };

    let grpc_response = match state
        .core_service
        .broadcast_transaction(dapi_grpc::tonic::Request::new(req))
        .await
    {
        Ok(resp) => resp.into_inner(),
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            ));
        }
    };

    Ok(Json(serde_json::json!({
        "transactionId": grpc_response.transaction_id
    })))
}

// JSON-RPC handlers
async fn handle_jsonrpc_request(
    State(state): State<JsonRpcAppState>,
    Json(json_rpc): Json<JsonRpcRequest>,
) -> Json<Value> {
    let id = json_rpc.id.clone();

    // Translate JSON-RPC request
    let (call, request_id) = match state.translator.translate_request(json_rpc).await {
        Ok((req, id)) => (req, id),
        Err(e) => {
            let error_response = state.translator.error_response(e, id);
            return Json(serde_json::to_value(error_response).unwrap_or_default());
        }
    };

    use crate::protocol::JsonRpcCall;
    match call {
        JsonRpcCall::PlatformGetStatus(grpc_request) => {
            let grpc_response = match state
                .platform_service
                .get_status(dapi_grpc::tonic::Request::new(grpc_request))
                .await
            {
                Ok(resp) => resp.into_inner(),
                Err(e) => {
                    let dapi_error =
                        crate::error::DapiError::Internal(format!("gRPC error: {}", e));
                    let error_response = state.translator.error_response(dapi_error, request_id);
                    return Json(serde_json::to_value(error_response).unwrap_or_default());
                }
            };

            match state
                .translator
                .translate_response(grpc_response, request_id)
                .await
            {
                Ok(json_rpc_response) => {
                    Json(serde_json::to_value(json_rpc_response).unwrap_or_default())
                }
                Err(e) => {
                    let error_response = state.translator.error_response(e, id);
                    Json(serde_json::to_value(error_response).unwrap_or_default())
                }
            }
        }
        JsonRpcCall::CoreBroadcastTransaction(req_broadcast) => {
            let result = state
                .core_service
                .broadcast_transaction(dapi_grpc::tonic::Request::new(req_broadcast))
                .await;
            match result {
                Ok(resp) => {
                    let txid = resp.into_inner().transaction_id;
                    let ok = state
                        .translator
                        .ok_response(serde_json::json!(txid), request_id);
                    Json(serde_json::to_value(ok).unwrap_or_default())
                }
                Err(e) => {
                    let dapi_error =
                        crate::error::DapiError::Internal(format!("Core gRPC error: {}", e));
                    let error_response = state.translator.error_response(dapi_error, request_id);
                    Json(serde_json::to_value(error_response).unwrap_or_default())
                }
            }
        }
        JsonRpcCall::CoreGetBestBlockHash => {
            use dapi_grpc::core::v0::GetBlockchainStatusRequest;
            let resp = match state
                .core_service
                .get_blockchain_status(dapi_grpc::tonic::Request::new(
                    GetBlockchainStatusRequest {},
                ))
                .await
            {
                Ok(r) => r.into_inner(),
                Err(e) => {
                    let dapi_error =
                        crate::error::DapiError::Internal(format!("Core gRPC error: {}", e));
                    let error_response = state.translator.error_response(dapi_error, request_id);
                    return Json(serde_json::to_value(error_response).unwrap_or_default());
                }
            };
            let best_block_hash_hex = resp
                .chain
                .map(|c| hex::encode(c.best_block_hash))
                .unwrap_or_default();
            let ok = state
                .translator
                .ok_response(serde_json::json!(best_block_hash_hex), request_id);
            Json(serde_json::to_value(ok).unwrap_or_default())
        }
        JsonRpcCall::CoreGetBlockHash { height } => {
            // Use underlying core client via service
            let result = state.core_service.core_client.get_block_hash(height).await;
            match result {
                Ok(hash) => {
                    let ok = state
                        .translator
                        .ok_response(serde_json::json!(hash.to_string()), request_id);
                    Json(serde_json::to_value(ok).unwrap_or_default())
                }
                Err(e) => {
                    let dapi_error =
                        crate::error::DapiError::Internal(format!("Core RPC error: {}", e));
                    let error_response = state.translator.error_response(dapi_error, request_id);
                    Json(serde_json::to_value(error_response).unwrap_or_default())
                }
            }
        }
    }
}

// Health check handlers
async fn handle_health() -> Json<Value> {
    Json(serde_json::json!({
        "status": "ok",
        "timestamp": chrono::Utc::now().timestamp(),
        "version": env!("CARGO_PKG_VERSION")
    }))
}

async fn handle_ready() -> Json<Value> {
    Json(serde_json::json!({
        "status": "ready",
        "timestamp": chrono::Utc::now().timestamp()
    }))
}

async fn handle_live() -> Json<Value> {
    Json(serde_json::json!({
        "status": "alive",
        "timestamp": chrono::Utc::now().timestamp()
    }))
}

async fn handle_metrics() -> axum::response::Response {
    let (body, content_type) = crate::metrics::gather_prometheus();
    axum::response::Response::builder()
        .status(200)
        .header(axum::http::header::CONTENT_TYPE, content_type)
        .body(axum::body::Body::from(body))
        .unwrap_or_else(|_| axum::response::Response::new(axum::body::Body::from("")))
}
