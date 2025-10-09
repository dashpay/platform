mod error;
mod params;
mod types;

use dapi_grpc::core::v0::BroadcastTransactionRequest;
use dapi_grpc::platform::v0::{GetStatusRequest, GetStatusResponse};
use serde_json::Value;

use crate::error::{DapiError, DapiResult};

pub use types::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};

#[derive(Debug, Default, Clone)]
pub struct JsonRpcTranslator;

/// Supported JSON-RPC calls handled by the gateway
#[derive(Debug)]
pub enum JsonRpcCall {
    PlatformGetStatus(GetStatusRequest),
    CoreGetBestBlockHash,
    CoreGetBlockHash { height: u32 },
    CoreBroadcastTransaction(BroadcastTransactionRequest),
}

impl JsonRpcTranslator {
    /// Create a new translator that maps between JSON-RPC payloads and gRPC requests.
    pub fn new() -> Self {
        Self
    }

    /// Interpret an incoming JSON-RPC request and produce the corresponding gRPC call marker.
    /// Validates parameters and converts them into typed messages or structured errors.
    pub async fn translate_request(&self, json_rpc: JsonRpcRequest) -> DapiResult<JsonRpcCall> {
        if json_rpc.jsonrpc != "2.0" {
            return Err(DapiError::InvalidArgument("jsonrpc must be \"2.0\"".into()));
        }

        match json_rpc.method.as_str() {
            "getStatus" => Ok(self.translate_platform_status()),
            "getBestBlockHash" => Ok(JsonRpcCall::CoreGetBestBlockHash),
            "getBlockHash" => {
                let height = params::parse_first_u32_param(json_rpc.params)
                    .map_err(DapiError::InvalidArgument)?;
                Ok(JsonRpcCall::CoreGetBlockHash { height })
            }
            "sendRawTransaction" => {
                let (tx, allow_high_fees, bypass_limits) =
                    params::parse_send_raw_tx_params(json_rpc.params)
                        .map_err(DapiError::InvalidArgument)?;
                let req = BroadcastTransactionRequest {
                    transaction: tx,
                    allow_high_fees,
                    bypass_limits,
                };
                Ok(JsonRpcCall::CoreBroadcastTransaction(req))
            }
            _ => Err(DapiError::MethodNotFound("Method not found".to_string())),
        }
    }

    /// Convert a gRPC Platform status response into a JSON-RPC success envelope.
    /// Serializes the message to JSON, wrapping serialization failures as internal errors.
    /// Propagates the original request id.
    pub async fn translate_response(
        &self,
        response: GetStatusResponse,
        id: Option<Value>,
    ) -> DapiResult<JsonRpcResponse> {
        let result = serde_json::to_value(&response)
            .map_err(|e| DapiError::Internal(format!("Failed to serialize response: {}", e)))?;
        Ok(JsonRpcResponse::ok(result, id))
    }

    /// Build a JSON-RPC error response from a rich `DapiError` using protocol mappings.
    pub fn error_response<E: Into<DapiError>>(
        &self,
        error: E,
        id: Option<Value>,
    ) -> JsonRpcResponse {
        let (code, message, data) = error::map_error(&error.into());
        JsonRpcResponse::error(code, message, data, id)
    }

    /// Build a JSON-RPC success response with the provided JSON result payload.
    pub fn ok_response(&self, result: Value, id: Option<Value>) -> JsonRpcResponse {
        JsonRpcResponse::ok(result, id)
    }

    /// Construct the gRPC request variant for the `getStatus` Platform call.
    fn translate_platform_status(&self) -> JsonRpcCall {
        use dapi_grpc::platform::v0::get_status_request::GetStatusRequestV0;

        let request_v0 = GetStatusRequestV0 {};
        let grpc_request = GetStatusRequest {
            version: Some(dapi_grpc::platform::v0::get_status_request::Version::V0(
                request_v0,
            )),
        };
        JsonRpcCall::PlatformGetStatus(grpc_request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn translate_get_status_request() {
        let t = JsonRpcTranslator::new();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "getStatus".to_string(),
            params: None,
            id: Some(json!(1)),
        };
        let call = t.translate_request(req).await.expect("translate ok");
        match call {
            JsonRpcCall::PlatformGetStatus(_) => {}
            _ => panic!("expected PlatformGetStatus"),
        }
    }

    #[tokio::test]
    async fn translate_get_best_block_hash_request() {
        let t = JsonRpcTranslator::new();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "getBestBlockHash".to_string(),
            params: None,
            id: Some(json!(2)),
        };
        let call = t.translate_request(req).await.expect("translate ok");
        match call {
            JsonRpcCall::CoreGetBestBlockHash => {}
            _ => panic!("expected CoreGetBestBlockHash"),
        }
    }

    #[tokio::test]
    async fn translate_get_block_hash_with_height() {
        let t = JsonRpcTranslator::new();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "getBlockHash".to_string(),
            params: Some(json!({"height": 12345})),
            id: Some(json!(3)),
        };
        let call = t.translate_request(req).await.expect("translate ok");
        match call {
            JsonRpcCall::CoreGetBlockHash { height } => assert_eq!(height, 12345),
            _ => panic!("expected CoreGetBlockHash"),
        }
    }

    #[tokio::test]
    async fn translate_get_block_hash_missing_param_errors() {
        let t = JsonRpcTranslator::new();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "getBlockHash".to_string(),
            params: Some(json!({})),
            id: Some(json!(4)),
        };
        let err = t.translate_request(req).await.unwrap_err();
        match err {
            DapiError::InvalidArgument(msg) => assert!(msg.contains("required property")),
            _ => panic!("expected InvalidArgument"),
        }
    }

    #[test]
    fn parse_first_param_validates_types() {
        use super::params::parse_first_u32_param;

        assert_eq!(
            parse_first_u32_param(Some(json!({"height": 0}))).unwrap(),
            0
        );
        assert!(
            parse_first_u32_param(Some(json!(null)))
                .unwrap_err()
                .contains("params must be object")
        );
        assert!(
            parse_first_u32_param(Some(json!({})))
                .unwrap_err()
                .contains("required property")
        );
        assert!(
            parse_first_u32_param(Some(json!({"height": -1})))
                .unwrap_err()
                .contains(">= 0")
        );
        assert!(
            parse_first_u32_param(Some(json!({"height": 0.5})))
                .unwrap_err()
                .contains("integer")
        );
        assert!(
            parse_first_u32_param(Some(json!({"height": (u32::MAX as u64) + 1})))
                .unwrap_err()
                .contains("<= 4294967295")
        );
    }

    #[tokio::test]
    async fn translate_response_wraps_result() {
        let t = JsonRpcTranslator::new();
        let resp = GetStatusResponse { version: None };
        let out = t
            .translate_response(resp, Some(json!(5)))
            .await
            .expect("serialize ok");
        assert_eq!(out.jsonrpc, "2.0");
        assert_eq!(out.id, Some(json!(5)));
        assert!(out.error.is_none());
        assert!(out.result.is_some());
    }

    #[test]
    fn error_response_codes_match() {
        let t = JsonRpcTranslator::new();
        let r = t.error_response(DapiError::InvalidArgument("bad".into()), Some(json!(1)));
        assert_eq!(r.error.as_ref().unwrap().code, -32602);
        let r = t.error_response(DapiError::NotFound("nope".into()), None);
        assert_eq!(r.error.as_ref().unwrap().code, -32602);
        let r = t.error_response(DapiError::ServiceUnavailable("x".into()), None);
        assert_eq!(r.error.as_ref().unwrap().code, -32003);
        let r = t.error_response(DapiError::Internal("x".into()), None);
        assert_eq!(r.error.as_ref().unwrap().code, -32603);
    }

    #[tokio::test]
    async fn translate_send_raw_transaction_basic() {
        let t = JsonRpcTranslator::new();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "sendRawTransaction".to_string(),
            params: Some(json!(["deadbeef"])),
            id: Some(json!(7)),
        };
        let call = t.translate_request(req).await.expect("translate ok");
        match call {
            JsonRpcCall::CoreBroadcastTransaction(r) => {
                assert_eq!(r.transaction, hex::decode("deadbeef").unwrap());
                assert!(!r.allow_high_fees);
                assert!(!r.bypass_limits);
            }
            _ => panic!("expected CoreBroadcastTransaction"),
        }
    }

    #[test]
    fn parse_send_raw_tx_params_variants() {
        use super::params::parse_send_raw_tx_params;

        let (tx, a, b) = parse_send_raw_tx_params(Some(json!("ff"))).unwrap();
        assert_eq!(tx, vec![0xff]);
        assert!(!a && !b);

        let (tx, a, b) = parse_send_raw_tx_params(Some(json!(["ff", true, true]))).unwrap();
        assert_eq!(tx, vec![0xff]);
        assert!(a && b);

        assert!(parse_send_raw_tx_params(Some(json!([]))).is_err());
        assert!(parse_send_raw_tx_params(Some(json!([123]))).is_err());
    }
}
