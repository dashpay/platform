use anyhow::Result;
use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde_json::Value;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tracing::{error, info};

use dapi_grpc::platform::v0::platform_server::{Platform, PlatformServer};

use crate::clients::traits::{DriveClientTrait, TenderdashClientTrait};
use crate::clients::{DriveClient, TenderdashClient};
use crate::config::Config;
use crate::protocol::{JsonRpcRequest, JsonRpcTranslator, RestTranslator};
use crate::services::PlatformServiceImpl;

pub struct DapiServer {
    config: Config,
    platform_service: Arc<PlatformServiceImpl>,
    rest_translator: Arc<RestTranslator>,
    jsonrpc_translator: Arc<JsonRpcTranslator>,
}

impl DapiServer {
    pub async fn new(config: Config) -> Result<Self> {
        // Create clients based on configuration
        // For now, let's use real clients by default
        let drive_client: Arc<dyn DriveClientTrait> =
            Arc::new(DriveClient::new(&config.dapi.drive.uri));

        let tenderdash_client: Arc<dyn TenderdashClientTrait> =
            Arc::new(TenderdashClient::new(&config.dapi.tenderdash.uri));

        let platform_service = Arc::new(PlatformServiceImpl::new(
            drive_client,
            tenderdash_client,
            config.clone(),
        ));

        let rest_translator = Arc::new(RestTranslator::new());
        let jsonrpc_translator = Arc::new(JsonRpcTranslator::new());

        Ok(Self {
            config,
            platform_service,
            rest_translator,
            jsonrpc_translator,
        })
    }
    pub async fn run(self) -> Result<()> {
        // For minimal proof-of-concept, just start the gRPC server
        tracing::info!("Starting DAPI server...");
        self.start_grpc_api_server().await
    }

    async fn start_grpc_api_server(&self) -> Result<()> {
        let addr = self.config.grpc_api_addr();
        info!("Starting gRPC API server on {}", addr);

        let platform_service = self.platform_service.clone();

        dapi_grpc::tonic::transport::Server::builder()
            .add_service(PlatformServer::new((*platform_service).clone()))
            .serve(addr)
            .await?;

        Ok(())
    }

    async fn start_rest_server(&self) -> Result<()> {
        let addr = self.config.rest_gateway_addr();
        info!("Starting REST gateway server on {}", addr);

        let app_state = RestAppState {
            platform_service: self.platform_service.clone(),
            translator: self.rest_translator.clone(),
        };

        let app = Router::new()
            .route("/v1/platform/status", get(handle_rest_get_status))
            .layer(ServiceBuilder::new().layer(CorsLayer::permissive()))
            .with_state(app_state);

        let listener = TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }

    async fn start_jsonrpc_server(&self) -> Result<()> {
        let addr = self.config.json_rpc_addr();
        info!("Starting JSON-RPC server on {}", addr);

        let app_state = JsonRpcAppState {
            platform_service: self.platform_service.clone(),
            translator: self.jsonrpc_translator.clone(),
        };

        let app = Router::new()
            .route("/", post(handle_jsonrpc_request))
            .layer(ServiceBuilder::new().layer(CorsLayer::permissive()))
            .with_state(app_state);

        let listener = TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }

    async fn start_health_server(&self) -> Result<()> {
        let addr = self.config.health_check_addr();
        info!("Starting health check server on {}", addr);

        let app = Router::new()
            .route("/health", get(handle_health))
            .route("/health/ready", get(handle_ready))
            .route("/health/live", get(handle_live))
            .route("/metrics", get(handle_metrics));

        let listener = TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }
}

#[derive(Clone)]
struct RestAppState {
    platform_service: Arc<PlatformServiceImpl>,
    translator: Arc<RestTranslator>,
}

#[derive(Clone)]
struct JsonRpcAppState {
    platform_service: Arc<PlatformServiceImpl>,
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

// JSON-RPC handlers
async fn handle_jsonrpc_request(
    State(state): State<JsonRpcAppState>,
    Json(json_rpc): Json<JsonRpcRequest>,
) -> Json<Value> {
    let id = json_rpc.id.clone();

    // Translate JSON-RPC request to gRPC
    let (grpc_request, request_id) = match state.translator.translate_request(json_rpc).await {
        Ok((req, id)) => (req, id),
        Err(e) => {
            let error_response = state.translator.error_response(e, id);
            return Json(serde_json::to_value(error_response).unwrap_or_default());
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
            let dapi_error = crate::errors::DapiError::Internal(format!("gRPC error: {}", e));
            let error_response = state.translator.error_response(dapi_error, request_id);
            return Json(serde_json::to_value(error_response).unwrap_or_default());
        }
    };

    // Translate gRPC response back to JSON-RPC
    match state
        .translator
        .translate_response(grpc_response, request_id)
        .await
    {
        Ok(json_rpc_response) => Json(serde_json::to_value(json_rpc_response).unwrap_or_default()),
        Err(e) => {
            let error_response = state.translator.error_response(e, id);
            Json(serde_json::to_value(error_response).unwrap_or_default())
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

async fn handle_metrics() -> Json<Value> {
    Json(serde_json::json!({
        "requests_total": 0,
        "requests_per_second": 0,
        "memory_usage_bytes": 0,
        "uptime_seconds": 0
    }))
}
