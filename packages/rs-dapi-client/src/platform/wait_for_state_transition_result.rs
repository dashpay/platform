//! `WaitForStateTransitionResult*` requests.

use std::time::Duration;

use dapi_grpc::platform::v0::{self as platform_proto, Proof, ResponseMetadata};

use crate::{transport::TransportRequest, DapiRequest, Settings};

use super::IncompleteMessage;

/// TODO
#[derive(Debug)]
pub struct WaitForStateTransitionResult {
    /// State transition hash
    pub state_transition_hash: Vec<u8>,
}

/// TODO
#[derive(Debug)]
pub struct WaitForStateTransitionResultResponse {
    /// TODO
    pub broadcast_error: Option<StateTransitionBroadcastError>,
    /// Response metadata
    pub metadata: ResponseMetadata,
}

/// TODO
#[derive(Debug)]
pub struct StateTransitionBroadcastError {
    /// TODO
    pub code: u32,
    /// TODO
    pub message: String,
    /// TODO
    pub data: Vec<u8>,
}

impl DapiRequest for WaitForStateTransitionResult {
    type DapiResponse = WaitForStateTransitionResultResponse;

    const SETTINGS_OVERRIDES: Settings = Settings {
        timeout: Some(Duration::from_secs(120)),
        ..Settings::default()
    };

    type Error = IncompleteMessage;

    type TransportRequest = platform_proto::WaitForStateTransitionResultRequest;

    fn to_transport_request(&self) -> Self::TransportRequest {
        platform_proto::WaitForStateTransitionResultRequest {
            state_transition_hash: self.state_transition_hash.clone(),
            prove: false,
        }
    }

    fn try_from_transport_response(
        transport_response: <Self::TransportRequest as TransportRequest>::Response,
    ) -> Result<Self::DapiResponse, Self::Error> {
        use platform_proto::wait_for_state_transition_result_response::Result as GrpcResponseBody;
        use platform_proto::WaitForStateTransitionResultResponse as GrpcResponse;

        match transport_response {
            GrpcResponse {
                result: None,
                metadata: Some(metadata),
            } => Ok(WaitForStateTransitionResultResponse {
                metadata,
                broadcast_error: None,
            }),
            GrpcResponse {
                result:
                    Some(GrpcResponseBody::Error(platform_proto::StateTransitionBroadcastError {
                        code,
                        message,
                        data,
                    })),
                metadata: Some(metadata),
            } => Ok(WaitForStateTransitionResultResponse {
                broadcast_error: Some(StateTransitionBroadcastError {
                    code,
                    message,
                    data,
                }),
                metadata,
            }),
            _ => Err(IncompleteMessage),
        }
    }
}

/// TODO
#[derive(Debug)]
pub struct WaitForStateTransitionResultProof {
    /// State transition hash
    pub state_transition_hash: Vec<u8>,
}

/// TODO
#[derive(Debug)]
pub struct WaitForStateTransitionResultProofResponse {
    /// Proof data that wraps broadcast errors if any
    pub proof: Proof,
    /// Response metadata
    pub metadata: ResponseMetadata,
}

impl DapiRequest for WaitForStateTransitionResultProof {
    type DapiResponse = WaitForStateTransitionResultProofResponse;

    const SETTINGS_OVERRIDES: Settings = Settings {
        timeout: Some(Duration::from_secs(120)),
        ..Settings::default()
    };

    type Error = IncompleteMessage;

    type TransportRequest = platform_proto::WaitForStateTransitionResultRequest;

    fn to_transport_request(&self) -> Self::TransportRequest {
        platform_proto::WaitForStateTransitionResultRequest {
            state_transition_hash: self.state_transition_hash.clone(),
            prove: true,
        }
    }

    fn try_from_transport_response(
        transport_response: <Self::TransportRequest as TransportRequest>::Response,
    ) -> Result<Self::DapiResponse, Self::Error> {
        use platform_proto::wait_for_state_transition_result_response::Result as GrpcResponseBody;
        use platform_proto::WaitForStateTransitionResultResponse as GrpcResponse;

        match transport_response {
            GrpcResponse {
                result: Some(GrpcResponseBody::Proof(proof)),
                metadata: Some(metadata),
            } => Ok(WaitForStateTransitionResultProofResponse { proof, metadata }),
            _ => Err(IncompleteMessage),
        }
    }
}
