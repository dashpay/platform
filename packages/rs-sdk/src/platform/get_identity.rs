//! `GetIdentity` request.

use dapi_grpc::platform::v0::{self as platform_proto};

/// Request Identity.
#[derive(Debug)]
pub struct GetIdentityRequest {
    /// Identity ID to search.
    pub id: Vec<u8>,
}

impl From<GetIdentityRequest> for platform_proto::GetIdentityRequest {
    fn from(dapi_request: GetIdentityRequest) -> Self {
        platform_proto::GetIdentityRequest {
            id: dapi_request.id,
            prove: true,
        }
    }
}
