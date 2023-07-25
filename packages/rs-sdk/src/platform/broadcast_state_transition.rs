//! `BroadcastStateTransition` request.

use dapi_grpc::platform::v0::{self as platform_proto};
use drive::dpp::state_transition::StateTransition;

/// DAPI request to suggest a [StateTransition] to the network.
#[derive(Debug)]
pub struct BroadcastStateTransitionRequest {
    /// State transition to broadcast
    pub state_transition: StateTransition,
}

impl From<BroadcastStateTransitionRequest> for platform_proto::BroadcastStateTransitionRequest {
    fn from(dapi_request: BroadcastStateTransitionRequest) -> Self {
        let mut cbor_state_transition = Vec::new();
        ciborium::into_writer(&dapi_request.state_transition, &mut cbor_state_transition)
            .expect("byte slice is a safe writer");
        platform_proto::BroadcastStateTransitionRequest {
            state_transition: cbor_state_transition,
        }
    }
}
