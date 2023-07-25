//! `WaitForStateTransitionResult` request.

use dapi_grpc::platform::v0::{self as platform_proto};

/// Request for a state transition broadcast errors if any.
#[derive(Debug)]
pub struct WaitForStateTransitionResultRequest {
    /// State transition hash
    pub state_transition_hash: Vec<u8>,
}

impl From<WaitForStateTransitionResultRequest>
    for platform_proto::WaitForStateTransitionResultRequest
{
    fn from(dapi_request: WaitForStateTransitionResultRequest) -> Self {
        platform_proto::WaitForStateTransitionResultRequest {
            state_transition_hash: dapi_request.state_transition_hash,
            prove: true,
        }
    }
}
