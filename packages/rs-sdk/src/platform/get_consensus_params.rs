//! `GetConsensusParams` request.

use dapi_grpc::platform::v0::{self as platform_proto};

/// Request consensus params.
// TODO: is it even user-facing?
#[derive(Debug)]
pub struct GetConsensusParamsRequest {
    /// Block height
    pub height: u32,
}

impl From<GetConsensusParamsRequest> for platform_proto::GetConsensusParamsRequest {
    fn from(dapi_request: GetConsensusParamsRequest) -> Self {
        platform_proto::GetConsensusParamsRequest {
            height: dapi_request.height as i64,
            prove: true,
        }
    }
}
