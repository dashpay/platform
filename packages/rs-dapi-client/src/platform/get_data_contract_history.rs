//! `GetDataContractHistory*` requests.

use dapi_grpc::platform::v0::{self as platform_proto, Proof, ResponseMetadata};

use super::IncompleteMessage;
use crate::{transport::TransportRequest, DapiRequest, Settings};

/// TODO
#[derive(Debug)]
pub struct GetDataContractHistory {
    /// Data contract id
    pub id: Vec<u8>,
    /// TODO
    pub limit: u32,
    /// TODO
    pub offset: u32,
    /// TODO
    pub start_at_seconds: u64,
}

/// DAPI response for [GetDataContractHistory].
#[derive(Debug)]
pub struct GetDataContractHistoryResponse {
    /// Serialized DataContractHistory
    pub data_contract_history: Vec<DataContractHistoryEntry>,
    /// Response metadata
    pub metadata: ResponseMetadata,
}

/// TODO
#[derive(Debug)]
pub struct DataContractHistoryEntry {
    /// TODO
    pub date: u64,
    /// TODO
    pub value: Vec<u8>,
}

impl From<platform_proto::get_data_contract_history_response::DataContractHistoryEntry>
    for DataContractHistoryEntry
{
    fn from(
        proto: platform_proto::get_data_contract_history_response::DataContractHistoryEntry,
    ) -> Self {
        DataContractHistoryEntry {
            date: proto.date,
            value: proto.value,
        }
    }
}

impl DapiRequest for GetDataContractHistory {
    type DapiResponse = GetDataContractHistoryResponse;

    const SETTINGS_OVERRIDES: Settings = Settings::default();

    type Error = IncompleteMessage;

    type TransportRequest = platform_proto::GetDataContractHistoryRequest;

    fn to_transport_request(&self) -> Self::TransportRequest {
        platform_proto::GetDataContractHistoryRequest {
            id: self.id.clone(),
            limit: self.limit,
            offset: self.offset,
            start_at_ms: self.start_at_seconds,
            prove: false,
        }
    }

    fn try_from_transport_response(
        transport_response: <Self::TransportRequest as TransportRequest>::Response,
    ) -> Result<Self::DapiResponse, Self::Error> {
        use platform_proto::get_data_contract_history_response::DataContractHistory;
        use platform_proto::get_data_contract_history_response::Result as GrpcResponseBody;
        use platform_proto::GetDataContractHistoryResponse as GrpcResponse;

        match transport_response {
            GrpcResponse {
                result: None,
                metadata: Some(metadata),
            } => Ok(GetDataContractHistoryResponse {
                data_contract_history: Vec::new(),
                metadata,
            }),
            GrpcResponse {
                result:
                    Some(GrpcResponseBody::DataContractHistory(DataContractHistory {
                        data_contract_entries,
                    })),
                metadata: Some(metadata),
            } => Ok(GetDataContractHistoryResponse {
                data_contract_history: data_contract_entries.into_iter().map(Into::into).collect(),
                metadata,
            }),
            _ => Err(IncompleteMessage),
        }
    }
}

/// TODO
#[derive(Debug)]
pub struct GetDataContractHistoryProof {
    /// Data contract id
    pub id: Vec<u8>,
    /// TODO
    pub limit: u32,
    /// TODO
    pub offset: u32,
    /// TODO
    pub start_at_seconds: u64,
}

/// DAPI response for [GetDataContractHistory].
#[derive(Debug)]
pub struct GetDataContractHistoryProofResponse {
    /// Proof data that wraps DataContractHistory
    pub proof: Proof,
    /// Response metadata
    pub metadata: ResponseMetadata,
}

impl DapiRequest for GetDataContractHistoryProof {
    type DapiResponse = GetDataContractHistoryProofResponse;

    const SETTINGS_OVERRIDES: Settings = Settings::default();

    type Error = IncompleteMessage;

    type TransportRequest = platform_proto::GetDataContractHistoryRequest;

    fn to_transport_request(&self) -> Self::TransportRequest {
        platform_proto::GetDataContractHistoryRequest {
            id: self.id.clone(),
            limit: self.limit,
            offset: self.offset,
            start_at_ms: self.start_at_seconds,
            prove: true,
        }
    }

    fn try_from_transport_response(
        transport_response: <Self::TransportRequest as TransportRequest>::Response,
    ) -> Result<Self::DapiResponse, Self::Error> {
        use platform_proto::get_data_contract_history_response::Result as GrpcResponseBody;
        use platform_proto::GetDataContractHistoryResponse as GrpcResponse;

        match transport_response {
            GrpcResponse {
                result: Some(GrpcResponseBody::Proof(proof)),
                metadata: Some(metadata),
            } => Ok(GetDataContractHistoryProofResponse { proof, metadata }),
            _ => Err(IncompleteMessage),
        }
    }
}
