//! `GetDataContractHistory` request.

use dapi_grpc::platform::v0::{self as platform_proto};

/// TODO
#[derive(Debug)]
pub struct GetDataContractHistoryRequest {
    /// Data contract id
    pub id: Vec<u8>,
    /// TODO
    pub limit: u32,
    /// TODO
    pub offset: u32,
    /// TODO
    pub start_at_ms: u64,
}

impl From<GetDataContractHistoryRequest> for platform_proto::GetDataContractHistoryRequest {
    fn from(dapi_request: GetDataContractHistoryRequest) -> Self {
        platform_proto::GetDataContractHistoryRequest {
            id: dapi_request.id,
            limit: Some(dapi_request.limit),
            offset: Some(dapi_request.offset),
            start_at_ms: dapi_request.start_at_ms,
            prove: true,
        }
    }
}
