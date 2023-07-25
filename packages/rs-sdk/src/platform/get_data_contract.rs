//! `GetDataContract` request.

use dapi_grpc::platform::v0::{self as platform_proto};

/// Request DataContract.
#[derive(Debug)]
pub struct GetDataContractRequest {
    /// DataContract ID to search.
    pub id: Vec<u8>,
}

impl From<GetDataContractRequest> for platform_proto::GetDataContractRequest {
    fn from(dapi_request: GetDataContractRequest) -> Self {
        platform_proto::GetDataContractRequest {
            id: dapi_request.id,
            prove: true,
        }
    }
}
