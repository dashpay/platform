use axum::{Router, extract::State, response::Json, routing::post};
use serde_json::Value;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tracing::info;

use crate::error::{DAPIResult, DapiError};
use crate::logging::middleware::AccessLogLayer;
use crate::protocol::{JsonRpcCall, JsonRpcRequest};

use dapi_grpc::core::v0::core_server::Core;
use dapi_grpc::platform::v0::platform_server::Platform;

use super::DapiServer;
use super::state::JsonRpcAppState;

impl DapiServer {
    /// Start the JSON-RPC HTTP server, configuring state, CORS, and access logging.
    /// Extracts shared services for request handling and binds the listener on the configured address.
    /// Returns when the server stops serving.
    pub(super) async fn start_jsonrpc_server(&self) -> DAPIResult<()> {
        let addr = self.config.json_rpc_addr()?;
        info!("Starting JSON-RPC server on {}", addr);

        let app_state = JsonRpcAppState {
            platform_service: self.platform_service.clone(),
            core_service: self.core_service.clone(),
            translator: self.jsonrpc_translator.clone(),
        };

        let mut app = Router::new()
            .route("/", post(handle_jsonrpc_request))
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

/// Handle a JSON-RPC request by translating it and delegating to the appropriate gRPC service.
/// Maps service responses and errors back into JSON-RPC payloads while preserving request ids.
/// Returns JSON suitable for Axum's response wrapper.
async fn handle_jsonrpc_request(
    State(state): State<JsonRpcAppState>,
    Json(json_rpc): Json<JsonRpcRequest>,
) -> Json<Value> {
    let id = json_rpc.id.clone();

    let call = match state.translator.translate_request(json_rpc).await {
        Ok(req) => req,
        Err(e) => {
            let error_response = state.translator.error_response(e, id.clone());
            return Json(serde_json::to_value(error_response).unwrap_or_default());
        }
    };

    match call {
        JsonRpcCall::PlatformGetStatus(grpc_request) => {
            let grpc_response = match state
                .platform_service
                .get_status(dapi_grpc::tonic::Request::new(grpc_request))
                .await
            {
                Ok(resp) => resp.into_inner(),
                Err(e) => {
                    let error_response = state.translator.error_response(e, id.clone());
                    return Json(serde_json::to_value(error_response).unwrap_or_default());
                }
            };

            match state
                .translator
                .translate_response(grpc_response, id.clone())
                .await
            {
                Ok(json_rpc_response) => {
                    Json(serde_json::to_value(json_rpc_response).unwrap_or_default())
                }
                Err(e) => {
                    let error_response = state.translator.error_response(e, id.clone());
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
                        .ok_response(serde_json::json!(txid), id.clone());
                    Json(serde_json::to_value(ok).unwrap_or_default())
                }
                Err(e) => {
                    let error_response = state.translator.error_response(e, id.clone());
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
                    let error_response = state.translator.error_response(e, id.clone());
                    return Json(serde_json::to_value(error_response).unwrap_or_default());
                }
            };
            let best_block_hash_hex = resp
                .chain
                .map(|c| hex::encode(c.best_block_hash))
                .unwrap_or_default();
            let ok = state
                .translator
                .ok_response(serde_json::json!(best_block_hash_hex), id.clone());
            Json(serde_json::to_value(ok).unwrap_or_default())
        }
        JsonRpcCall::CoreGetBlockHash { height } => {
            let result = state.core_service.core_client.get_block_hash(height).await;
            match result {
                Ok(hash) => {
                    let ok = state
                        .translator
                        .ok_response(serde_json::json!(hash.to_string()), id.clone());
                    Json(serde_json::to_value(ok).unwrap_or_default())
                }
                Err(e) => {
                    let error_response = state.translator.error_response(e, id.clone());
                    Json(serde_json::to_value(error_response).unwrap_or_default())
                }
            }
        }
    }
}
