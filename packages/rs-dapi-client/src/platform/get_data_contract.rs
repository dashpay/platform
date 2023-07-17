//! `GetDataContract*` requests.

use dapi_grpc::platform::v0::{self as platform_proto, Proof, ResponseMetadata};

use super::IncompleteMessage;
use crate::{transport::TransportRequest, DapiRequest, Settings};

/// Request DataContract bytes.
#[derive(Debug)]
pub struct GetDataContract {
    /// DataContract ID to search.
    pub id: Vec<u8>,
}

/// DAPI response for [GetDataContract].
#[derive(Debug)]
pub struct GetDataContractResponse {
    /// Serialized DataContract
    pub data_contract_bytes: Option<Vec<u8>>,
    /// Response metadata
    pub metadata: ResponseMetadata,
}

impl DapiRequest for GetDataContract {
    type DapiResponse = GetDataContractResponse;

    const SETTINGS_OVERRIDES: Settings = Settings::default();

    type Error = IncompleteMessage;

    type TransportRequest = platform_proto::GetDataContractRequest;

    fn to_transport_request(&self) -> Self::TransportRequest {
        platform_proto::GetDataContractRequest {
            id: self.id.clone(),
            prove: false,
        }
    }

    fn try_from_transport_response(
        transport_response: <Self::TransportRequest as TransportRequest>::Response,
    ) -> Result<Self::DapiResponse, Self::Error> {
        use platform_proto::get_data_contract_response::Result as GrpcResponseBody;
        use platform_proto::GetDataContractResponse as GrpcResponse;

        match transport_response {
            GrpcResponse {
                result: None,
                metadata: Some(metadata),
            } => Ok(GetDataContractResponse {
                data_contract_bytes: None,
                metadata,
            }),
            GrpcResponse {
                result: Some(GrpcResponseBody::DataContract(data_contract_bytes)),
                metadata: Some(metadata),
            } => Ok(GetDataContractResponse {
                data_contract_bytes: Some(data_contract_bytes),
                metadata,
            }),
            _ => Err(IncompleteMessage),
        }
    }
}

/// Request DataContract bytes wrapped into proof.
#[derive(Debug)]
pub struct GetDataContractProof {
    /// DataContract ID to search.
    pub id: Vec<u8>,
}

/// DAPI response for [GetDataContract].
#[derive(Debug)]
pub struct GetDataContractProofResponse {
    /// Proof data that wraps DataContract
    pub proof: Proof,
    /// Response metadata
    pub metadata: ResponseMetadata,
}

impl DapiRequest for GetDataContractProof {
    type DapiResponse = GetDataContractProofResponse;

    const SETTINGS_OVERRIDES: Settings = Settings::default();

    type Error = IncompleteMessage;

    type TransportRequest = platform_proto::GetDataContractRequest;

    fn to_transport_request(&self) -> Self::TransportRequest {
        platform_proto::GetDataContractRequest {
            id: self.id.clone(),
            prove: true,
        }
    }

    fn try_from_transport_response(
        transport_response: <Self::TransportRequest as TransportRequest>::Response,
    ) -> Result<Self::DapiResponse, Self::Error> {
        use platform_proto::get_data_contract_response::Result as GrpcResponseBody;
        use platform_proto::GetDataContractResponse as GrpcResponse;

        match transport_response {
            GrpcResponse {
                result: Some(GrpcResponseBody::Proof(proof)),
                metadata: Some(metadata),
            } => Ok(GetDataContractProofResponse { proof, metadata }),
            _ => Err(IncompleteMessage),
        }
    }
}
