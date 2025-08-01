// JSON-RPC to gRPC translator

use crate::errors::{DapiError, DapiResult};
use dapi_grpc::platform::v0::{GetStatusRequest, GetStatusResponse};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: Option<Value>,
    pub id: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub result: Option<Value>,
    pub error: Option<JsonRpcError>,
    pub id: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    pub data: Option<Value>,
}

#[derive(Debug, Default)]
pub struct JsonRpcTranslator;

impl JsonRpcTranslator {
    pub fn new() -> Self {
        Self
    }

    // Convert JSON-RPC request to gRPC request
    pub async fn translate_request(
        &self,
        json_rpc: JsonRpcRequest,
    ) -> DapiResult<(GetStatusRequest, Option<Value>)> {
        match json_rpc.method.as_str() {
            "getStatus" => {
                use dapi_grpc::platform::v0::get_status_request::GetStatusRequestV0;

                let request_v0 = GetStatusRequestV0 {};
                let grpc_request = GetStatusRequest {
                    version: Some(dapi_grpc::platform::v0::get_status_request::Version::V0(
                        request_v0,
                    )),
                };

                Ok((grpc_request, json_rpc.id))
            }
            _ => Err(DapiError::InvalidArgument(format!(
                "Unknown method: {}",
                json_rpc.method
            ))),
        }
    }

    // Convert gRPC response back to JSON-RPC response
    pub async fn translate_response(
        &self,
        response: GetStatusResponse,
        id: Option<Value>,
    ) -> DapiResult<JsonRpcResponse> {
        let result = serde_json::to_value(&response)
            .map_err(|e| DapiError::Internal(format!("Failed to serialize response: {}", e)))?;

        Ok(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(result),
            error: None,
            id,
        })
    }

    // Convert error to JSON-RPC error response
    pub fn error_response(&self, error: DapiError, id: Option<Value>) -> JsonRpcResponse {
        let (code, message) = match &error {
            DapiError::InvalidArgument(_) => (-32602, "Invalid params"),
            DapiError::NotFound(_) => (-32601, "Method not found"),
            DapiError::ServiceUnavailable(_) => (-32003, "Service unavailable"),
            _ => (-32603, "Internal error"),
        };

        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.to_string(),
                data: Some(Value::String(error.to_string())),
            }),
            id,
        }
    }
}
