use std::sync::Arc;
use tracing::info;

use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
};
use serde_json::Value;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;

use crate::error::DAPIResult;
use crate::logging::middleware::AccessLogLayer;

use dapi_grpc::core::v0::core_server::Core;
use dapi_grpc::platform::v0::platform_server::Platform;

use super::DapiServer;
use super::state::RestAppState;

impl DapiServer {
    pub(super) async fn start_rest_server(&self) -> DAPIResult<()> {
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
}

async fn handle_rest_get_status(
    State(state): State<RestAppState>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let grpc_request = match state.translator.translate_get_status().await {
        Ok(req) => req,
        Err(e) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": e.to_string()})),
            ));
        }
    };

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

    let grpc_req = match state.translator.translate_get_block_by_hash(hash).await {
        Ok(r) => r,
        Err(e) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": e.to_string()})),
            ));
        }
    };

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

    let grpc_req = match state.translator.translate_get_block_by_height(height).await {
        Ok(r) => r,
        Err(e) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": e.to_string()})),
            ));
        }
    };

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
