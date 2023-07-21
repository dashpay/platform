//! `GetIdentitiesByPublicKeyHashes*` requests.

use dapi_grpc::platform::v0::{self as platform_proto, Proof, ResponseMetadata};

use super::IncompleteMessage;
use crate::{transport::TransportRequest, DapiRequest, Settings};

/// Request Identities' bytes by public key hashes.
#[derive(Debug)]
pub struct GetIdentitiesByPublicKeyHashesRequest {
    /// Public key hashes
    pub public_key_hashes: Vec<Vec<u8>>,
}

/// DAPI response for [GetIdentitiesByPublicKeyHashes].
#[derive(Debug)]
pub struct GetIdentitiesByPublicKeyHashesResponse {
    /// Serialized Identities
    pub identities_bytes: Vec<Vec<u8>>,
    /// Response metadata
    pub metadata: ResponseMetadata,
}

impl DapiRequest for GetIdentitiesByPublicKeyHashesRequest {
    type DapiResponse = GetIdentitiesByPublicKeyHashesResponse;

    const SETTINGS_OVERRIDES: Settings = Settings::default();

    type Error = IncompleteMessage;

    type TransportRequest = platform_proto::GetIdentitiesByPublicKeyHashesRequest;

    fn to_transport_request(&self) -> Self::TransportRequest {
        platform_proto::GetIdentitiesByPublicKeyHashesRequest {
            public_key_hashes: self.public_key_hashes.clone(),
            prove: false,
        }
    }

    fn try_from_transport_response(
        transport_response: <Self::TransportRequest as TransportRequest>::Response,
    ) -> Result<Self::DapiResponse, Self::Error> {
        use platform_proto::get_identities_by_public_key_hashes_response::{
            Identities, Result as GrpcResponseBody,
        };
        use platform_proto::GetIdentitiesByPublicKeyHashesResponse as GrpcResponse;

        match transport_response {
            GrpcResponse {
                result: Some(GrpcResponseBody::Identities(Identities { identities })),
                metadata: Some(metadata),
            } => Ok(GetIdentitiesByPublicKeyHashesResponse {
                metadata,
                identities_bytes: identities,
            }),
            _ => Err(IncompleteMessage),
        }
    }
}

/// Request Identities' bytes by public key hashes wrapped into proof.
#[derive(Debug)]
pub struct GetIdentitiesByPublicKeyHashesProofRequest {
    /// Public key hashes
    pub public_key_hashes: Vec<Vec<u8>>,
}

/// DAPI response for [GetIdentitiesByPublicKeyHashes].
#[derive(Debug)]
pub struct GetIdentitiesByPublicKeyHashesProofResponse {
    /// Proof data that wraps Identity
    pub proof: Proof,
    /// Response metadata
    pub metadata: ResponseMetadata,
}

impl DapiRequest for GetIdentitiesByPublicKeyHashesProofRequest {
    type DapiResponse = GetIdentitiesByPublicKeyHashesProofResponse;

    const SETTINGS_OVERRIDES: Settings = Settings::default();

    type Error = IncompleteMessage;

    type TransportRequest = platform_proto::GetIdentitiesByPublicKeyHashesRequest;

    fn to_transport_request(&self) -> Self::TransportRequest {
        platform_proto::GetIdentitiesByPublicKeyHashesRequest {
            public_key_hashes: self.public_key_hashes.clone(),
            prove: true,
        }
    }

    fn try_from_transport_response(
        transport_response: <Self::TransportRequest as TransportRequest>::Response,
    ) -> Result<Self::DapiResponse, Self::Error> {
        use platform_proto::get_identities_by_public_key_hashes_response::Result as GrpcResponseBody;
        use platform_proto::GetIdentitiesByPublicKeyHashesResponse as GrpcResponse;

        match transport_response {
            GrpcResponse {
                result: Some(GrpcResponseBody::Proof(proof)),
                metadata: Some(metadata),
            } => Ok(GetIdentitiesByPublicKeyHashesProofResponse { proof, metadata }),
            _ => Err(IncompleteMessage),
        }
    }
}
