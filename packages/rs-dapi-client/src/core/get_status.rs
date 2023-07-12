/// `GetStatus` request.
use dapi_grpc::core::v0::{self as core_proto};

use crate::{transport::TransportRequest, DapiRequest, Settings};

/// Request core status.
pub struct GetStatus {}

/// Core status response.
pub struct GetStatusResponse {
    // TODO
}

impl DapiRequest for GetStatus {
    type DapiResponse = GetStatusResponse;

    const SETTINGS_OVERRIDES: Settings = Settings::default();

    type Error = ();

    type TransportRequest = core_proto::GetStatusRequest;

    fn to_transport_request(&self) -> Self::TransportRequest {
        core_proto::GetStatusRequest {}
    }

    fn try_from_transport_response(
        _transport_response: <Self::TransportRequest as TransportRequest>::Response,
    ) -> Result<Self::DapiResponse, Self::Error> {
        // TODO
        Ok(GetStatusResponse {})
    }
}
