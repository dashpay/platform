// REST to gRPC translator

use crate::error::{DapiError, DapiResult};
use dapi_grpc::core::v0::GetTransactionResponse as CoreGetTransactionResponse;
use dapi_grpc::platform::v0::{GetStatusRequest, GetStatusResponse};
use serde_json::Value;

#[derive(Debug, Default)]
pub struct RestTranslator;

impl RestTranslator {
    pub fn new() -> Self {
        Self
    }

    // Convert REST GET /v1/platform/status to gRPC GetStatusRequest
    pub async fn translate_get_status(&self) -> DapiResult<GetStatusRequest> {
        // For getStatus, there are no parameters in the REST call
        use dapi_grpc::platform::v0::get_status_request::GetStatusRequestV0;

        let request_v0 = GetStatusRequestV0 {};

        Ok(GetStatusRequest {
            version: Some(dapi_grpc::platform::v0::get_status_request::Version::V0(
                request_v0,
            )),
        })
    }

    // Convert gRPC GetStatusResponse back to REST JSON
    pub async fn translate_status_response(
        &self,
        response: GetStatusResponse,
    ) -> DapiResult<Value> {
        // Convert the gRPC response to JSON
        // This is a simplified implementation
        let json_value = serde_json::to_value(&response)
            .map_err(|e| DapiError::Internal(format!("Failed to serialize response: {}", e)))?;

        Ok(json_value)
    }

    // Convert gRPC best block height response to REST JSON
    pub async fn translate_best_block_height(&self, height: u32) -> DapiResult<Value> {
        Ok(serde_json::json!({ "height": height }))
    }

    // Convert gRPC GetTransactionResponse back to REST JSON
    pub async fn translate_transaction_response(
        &self,
        response: CoreGetTransactionResponse,
    ) -> DapiResult<Value> {
        let block_hash_hex = hex::encode(response.block_hash);
        Ok(serde_json::json!({
            "transaction": response.transaction,
            "blockHash": block_hash_hex,
            "height": response.height,
            "confirmations": response.confirmations,
            "isInstantLocked": response.is_instant_locked,
            "isChainLocked": response.is_chain_locked
        }))
    }
}
