// REST to gRPC translator

use crate::errors::{DapiError, DapiResult};
use axum::{Json, extract::Path, response::Json as ResponseJson};
use dapi_grpc::platform::v0::{GetStatusRequest, GetStatusResponse};
use serde_json::Value;

#[derive(Debug)]
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
            version: Some(dapi_grpc::platform::v0::get_status_request::Version::V0(request_v0)),
        })
    }

    // Convert gRPC GetStatusResponse back to REST JSON
    pub async fn translate_status_response(&self, response: GetStatusResponse) -> DapiResult<Value> {
        // Convert the gRPC response to JSON
        // This is a simplified implementation
        let json_value = serde_json::to_value(&response)
            .map_err(|e| DapiError::Internal(format!("Failed to serialize response: {}", e)))?;
        
        Ok(json_value)
    }
}
