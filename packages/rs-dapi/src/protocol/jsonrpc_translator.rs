// JSON-RPC to gRPC translator and legacy Core helpers

use crate::error::{DapiError, DapiResult};
use dapi_grpc::core::v0::BroadcastTransactionRequest;
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

/// Supported JSON-RPC calls handled by the gateway
#[derive(Debug)]
pub enum JsonRpcCall {
    /// Platform: getStatus
    PlatformGetStatus(GetStatusRequest),
    /// Core: getBestBlockHash (no params)
    CoreGetBestBlockHash,
    /// Core: getBlockHash(height)
    CoreGetBlockHash { height: u32 },
    /// Core: sendRawTransaction(rawtx[, allowHighFees, bypassLimits])
    CoreBroadcastTransaction(BroadcastTransactionRequest),
}

impl JsonRpcTranslator {
    pub fn new() -> Self {
        Self
    }

    // Convert JSON-RPC request to an internal call representation
    pub async fn translate_request(
        &self,
        json_rpc: JsonRpcRequest,
    ) -> DapiResult<(JsonRpcCall, Option<Value>)> {
        match json_rpc.method.as_str() {
            "getStatus" => {
                use dapi_grpc::platform::v0::get_status_request::GetStatusRequestV0;

                let request_v0 = GetStatusRequestV0 {};
                let grpc_request = GetStatusRequest {
                    version: Some(dapi_grpc::platform::v0::get_status_request::Version::V0(
                        request_v0,
                    )),
                };

                Ok((JsonRpcCall::PlatformGetStatus(grpc_request), json_rpc.id))
            }
            "getBestBlockHash" => Ok((JsonRpcCall::CoreGetBestBlockHash, json_rpc.id)),
            "getBlockHash" => {
                // Expect params as [height]
                let height =
                    parse_first_u32_param(json_rpc.params).map_err(DapiError::InvalidArgument)?;
                Ok((JsonRpcCall::CoreGetBlockHash { height }, json_rpc.id))
            }
            "sendRawTransaction" => {
                let (tx, allow_high_fees, bypass_limits) =
                    parse_send_raw_tx_params(json_rpc.params)
                        .map_err(DapiError::InvalidArgument)?;
                let req = BroadcastTransactionRequest {
                    transaction: tx,
                    allow_high_fees,
                    bypass_limits,
                };
                Ok((JsonRpcCall::CoreBroadcastTransaction(req), json_rpc.id))
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

    /// Build a simple success response with a JSON result value
    pub fn ok_response(&self, result: Value, id: Option<Value>) -> JsonRpcResponse {
        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(result),
            error: None,
            id,
        }
    }
}

fn parse_first_u32_param(params: Option<Value>) -> Result<u32, String> {
    match params {
        Some(Value::Array(a)) => {
            if a.is_empty() {
                return Err("missing required parameter".to_string());
            }
            parse_u32_from_value(&a[0])
        }
        Some(Value::Object(map)) => {
            let mut last_error = Some("object must contain a numeric value".to_string());
            for value in map.values() {
                match parse_u32_from_value(value) {
                    Ok(v) => return Ok(v),
                    Err(e) => last_error = Some(e),
                }
            }
            Err(last_error.expect("object must contain a numeric value"))
        }
        _ => Err("params must be an array or object".to_string()),
    }
}

fn parse_u32_from_value(value: &Value) -> Result<u32, String> {
    match value {
        Value::Number(n) => n
            .as_u64()
            .ok_or_else(|| "value must be a non-negative integer".to_string())
            .and_then(|v| {
                if v <= u32::MAX as u64 {
                    Ok(v as u32)
                } else {
                    Err("value out of range".to_string())
                }
            }),
        _ => Err("value must be a number".to_string()),
    }
}

fn parse_send_raw_tx_params(params: Option<Value>) -> Result<(Vec<u8>, bool, bool), String> {
    match params {
        // Typical JSON-RPC usage: positional array
        Some(Value::Array(a)) => {
            if a.is_empty() {
                return Err("missing raw transaction parameter".to_string());
            }
            let raw_hex = a[0]
                .as_str()
                .ok_or_else(|| "raw transaction must be a hex string".to_string())?;
            let tx = hex::decode(raw_hex)
                .map_err(|_| "raw transaction must be valid hex".to_string())?;

            let allow_high_fees = a.get(1).and_then(|v| v.as_bool()).unwrap_or(false);
            let bypass_limits = a.get(2).and_then(|v| v.as_bool()).unwrap_or(false);
            Ok((tx, allow_high_fees, bypass_limits))
        }
        // Accept single string too
        Some(Value::String(s)) => {
            let tx =
                hex::decode(&s).map_err(|_| "raw transaction must be valid hex".to_string())?;
            Ok((tx, false, false))
        }
        _ => Err("params must be an array or hex string".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn translate_get_status_request() {
        let t = JsonRpcTranslator::default();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "getStatus".to_string(),
            params: None,
            id: Some(json!(1)),
        };
        let (call, id) = t.translate_request(req).await.expect("translate ok");
        match call {
            JsonRpcCall::PlatformGetStatus(_g) => {}
            _ => panic!("expected PlatformGetStatus"),
        }
        assert_eq!(id, Some(json!(1)));
    }

    #[tokio::test]
    async fn translate_get_best_block_hash_request() {
        let t = JsonRpcTranslator::default();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "getBestBlockHash".to_string(),
            params: None,
            id: Some(json!(2)),
        };
        let (call, id) = t.translate_request(req).await.expect("translate ok");
        match call {
            JsonRpcCall::CoreGetBestBlockHash => {}
            _ => panic!("expected CoreGetBestBlockHash"),
        }
        assert_eq!(id, Some(json!(2)));
    }

    #[tokio::test]
    async fn translate_get_block_hash_with_height() {
        let t = JsonRpcTranslator::default();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "getBlockHash".to_string(),
            params: Some(json!([12345])),
            id: Some(json!(3)),
        };
        let (call, id) = t.translate_request(req).await.expect("translate ok");
        match call {
            JsonRpcCall::CoreGetBlockHash { height } => assert_eq!(height, 12345),
            _ => panic!("expected CoreGetBlockHash"),
        }
        assert_eq!(id, Some(json!(3)));
    }

    #[tokio::test]
    async fn translate_get_block_hash_missing_param_errors() {
        let t = JsonRpcTranslator::default();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "getBlockHash".to_string(),
            params: Some(json!([])),
            id: Some(json!(4)),
        };
        let err = t.translate_request(req).await.unwrap_err();
        match err {
            DapiError::InvalidArgument(msg) => assert!(msg.contains("missing required")),
            _ => panic!("expected InvalidArgument"),
        }
    }

    #[test]
    fn parse_first_param_validates_types() {
        assert_eq!(parse_first_u32_param(Some(json!([0]))).unwrap(), 0);
        assert!(
            parse_first_u32_param(Some(json!(["x"])))
                .unwrap_err()
                .contains("number")
        );
        // Out of range
        let big = (u64::from(u32::MAX)) + 1;
        assert!(
            parse_first_u32_param(Some(json!([big])))
                .unwrap_err()
                .contains("range")
        );
        // Not an array
        assert_eq!(
            parse_first_u32_param(Some(json!({"height": 1}))).unwrap(),
            1
        );
        assert_eq!(parse_first_u32_param(Some(json!({"count": 2}))).unwrap(), 2);
        assert!(
            parse_first_u32_param(Some(json!({})))
                .unwrap_err()
                .contains("numeric value")
        );
    }

    #[tokio::test]
    async fn translate_response_wraps_result() {
        let t = JsonRpcTranslator::default();
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
        let t = JsonRpcTranslator::default();
        let r = t.error_response(DapiError::InvalidArgument("bad".into()), Some(json!(1)));
        assert_eq!(r.error.unwrap().code, -32602);
        let r = t.error_response(DapiError::NotFound("nope".into()), None);
        assert_eq!(r.error.unwrap().code, -32601);
        let r = t.error_response(DapiError::ServiceUnavailable("x".into()), None);
        assert_eq!(r.error.unwrap().code, -32003);
        let r = t.error_response(DapiError::Internal("x".into()), None);
        assert_eq!(r.error.unwrap().code, -32603);
    }

    #[tokio::test]
    async fn translate_send_raw_transaction_basic() {
        let t = JsonRpcTranslator::default();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "sendRawTransaction".to_string(),
            params: Some(json!(["deadbeef"])),
            id: Some(json!(7)),
        };
        let (call, id) = t.translate_request(req).await.expect("translate ok");
        match call {
            JsonRpcCall::CoreBroadcastTransaction(r) => {
                assert_eq!(r.transaction, hex::decode("deadbeef").unwrap());
                assert!(!r.allow_high_fees);
                assert!(!r.bypass_limits);
            }
            _ => panic!("expected CoreBroadcastTransaction"),
        }
        assert_eq!(id, Some(json!(7)));
    }

    #[test]
    fn parse_send_raw_tx_params_variants() {
        // string
        let (tx, a, b) = parse_send_raw_tx_params(Some(json!("ff"))).unwrap();
        assert_eq!(tx, vec![0xff]);
        assert!(!a && !b);
        // array with flags
        let (tx, a, b) = parse_send_raw_tx_params(Some(json!(["ff", true, true]))).unwrap();
        assert_eq!(tx, vec![0xff]);
        assert!(a && b);
        // errors
        assert!(parse_send_raw_tx_params(Some(json!([]))).is_err());
        assert!(parse_send_raw_tx_params(Some(json!([123]))).is_err());
    }
}
