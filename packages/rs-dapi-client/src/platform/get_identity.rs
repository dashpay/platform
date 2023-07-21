//! `GetIdentity*` requests.

use dapi_grpc::platform::v0::{self as platform_proto, Proof, ResponseMetadata};

use super::IncompleteMessage;
use crate::{transport::TransportRequest, DapiRequest, Settings};

/// Request Identity bytes.
#[derive(Debug)]
pub struct GetIdentityRequest {
    /// Identity ID to search.
    pub id: Vec<u8>,
}

/// DAPI response for [GetIdentity].
#[derive(Debug)]
pub struct GetIdentityResponse {
    /// Serialized Identity
    pub identity_bytes: Option<Vec<u8>>,
    /// Response metadata
    pub metadata: ResponseMetadata,
}

impl DapiRequest for GetIdentityRequest {
    type DapiResponse = GetIdentityResponse;

    const SETTINGS_OVERRIDES: Settings = Settings::default();

    type Error = IncompleteMessage;

    type TransportRequest = platform_proto::GetIdentityRequest;

    fn to_transport_request(&self) -> Self::TransportRequest {
        platform_proto::GetIdentityRequest {
            id: self.id.clone(),
            prove: false,
        }
    }

    fn try_from_transport_response(
        transport_response: <Self::TransportRequest as TransportRequest>::Response,
    ) -> Result<Self::DapiResponse, Self::Error> {
        use platform_proto::get_identity_response::Result as GrpcResponseBody;
        use platform_proto::GetIdentityResponse as GrpcResponse;

        match transport_response {
            GrpcResponse {
                result: None,
                metadata: Some(metadata),
            } => Ok(GetIdentityResponse {
                metadata,
                identity_bytes: None,
            }),
            GrpcResponse {
                result: Some(GrpcResponseBody::Identity(identity_bytes)),
                metadata: Some(metadata),
            } => Ok(GetIdentityResponse {
                identity_bytes: Some(identity_bytes),
                metadata,
            }),
            _ => Err(IncompleteMessage),
        }
    }
}

/// Request Identity bytes wrapped into proof.
#[derive(Debug)]
pub struct GetIdentityProofRequest {
    /// Identity ID to search.
    pub id: Vec<u8>,
}

/// DAPI response for [GetIdentity].
#[derive(Debug)]
pub struct GetIdentityProofResponse {
    /// Proof data that wraps Identity
    pub proof: Proof,
    /// Response metadata
    pub metadata: ResponseMetadata,
}

impl DapiRequest for GetIdentityProofRequest {
    type DapiResponse = GetIdentityProofResponse;

    const SETTINGS_OVERRIDES: Settings = Settings::default();

    type Error = IncompleteMessage;

    type TransportRequest = platform_proto::GetIdentityRequest;

    fn to_transport_request(&self) -> Self::TransportRequest {
        platform_proto::GetIdentityRequest {
            id: self.id.clone(),
            prove: true,
        }
    }

    fn try_from_transport_response(
        transport_response: <Self::TransportRequest as TransportRequest>::Response,
    ) -> Result<Self::DapiResponse, Self::Error> {
        use platform_proto::get_identity_response::Result as GrpcResponseBody;
        use platform_proto::GetIdentityResponse as GrpcResponse;

        match transport_response {
            GrpcResponse {
                result: Some(GrpcResponseBody::Proof(proof)),
                metadata: Some(metadata),
            } => Ok(GetIdentityProofResponse { proof, metadata }),
            _ => Err(IncompleteMessage),
        }
    }
}
