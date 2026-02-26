//! Broadcast trait representing the action of broadcasting a state transition to Platform.
//!
//! The [`BroadcastRequestForStateTransition`] trait is designed for serializing state transitions
//! into transport requests suitable for broadcasting to Platform.
//!
//! This trait is expected to be implemented by objects that encapsulate the necessary data and logic
//! to serialize a state transition and prepare it for transport.
use std::fmt::Debug;

use dapi_grpc::platform::v0::wait_for_state_transition_result_request::{
    Version, WaitForStateTransitionResultRequestV0,
};
use dapi_grpc::platform::v0::{
    BroadcastStateTransitionRequest, WaitForStateTransitionResultRequest,
};

use dpp::serialization::PlatformSerializable;

use dpp::state_transition::StateTransition;

use crate::error::Error;

/// Trait implemented by objects that can be used to create broadcast requests for state transitions.
///
/// [`BroadcastRequestForStateTransition`] trait is used when a state transition needs to be broadcasted on Platform.
/// It encapsulates the serialization logic required to convert a state transition into a transport request.
///
/// Implementors of this trait will typically be responsible for serializing a state transition
/// and preparing it for transport to Platform.
///
/// ## Example
///
/// Given a [`StateTransition`](dpp::state_transition::StateTransition), you can create a broadcast
/// request using the `broadcast_request_for_state_transition` method:
///
/// ```rust,ignore
/// use dash_sdk::platform::transition::broadcast_request::BroadcastRequestForStateTransition;
/// use dpp::state_transition::StateTransition;
///
/// // Assume `state_transition` is an already-constructed and signed StateTransition
/// let broadcast_request = state_transition
///     .broadcast_request_for_state_transition()
///     .expect("failed to serialize state transition");
///
/// // The broadcast_request can now be sent to Platform via DAPI.
/// ```
///
/// As [`BroadcastRequestForStateTransition`] is a trait, it can be implemented for any type that represents
/// a state transition, allowing for flexibility in how state transitions are broadcasted.
pub trait BroadcastRequestForStateTransition: Send + Debug + Clone {
    /// Serializes the state transition into a [`BroadcastStateTransitionRequest`] ready for broadcasting.
    ///
    /// # Returns
    /// On success, this method yields a [`BroadcastStateTransitionRequest`] which can be sent to Platform.
    /// On failure, it yields an [`Error`].
    ///
    /// # Error Handling
    /// This method propagates any errors encountered during serialization.
    /// These are returned as [`Error`] instances.
    fn broadcast_request_for_state_transition(
        &self,
    ) -> Result<BroadcastStateTransitionRequest, Error>;

    fn wait_for_state_transition_result_request(
        &self,
    ) -> Result<WaitForStateTransitionResultRequest, Error>;
}

impl BroadcastRequestForStateTransition for StateTransition {
    fn broadcast_request_for_state_transition(
        &self,
    ) -> Result<BroadcastStateTransitionRequest, Error> {
        Ok(BroadcastStateTransitionRequest {
            state_transition: self.serialize_to_bytes()?,
        })
    }

    fn wait_for_state_transition_result_request(
        &self,
    ) -> Result<WaitForStateTransitionResultRequest, Error> {
        Ok(WaitForStateTransitionResultRequest {
            version: Some(Version::V0(WaitForStateTransitionResultRequestV0 {
                state_transition_hash: self.transaction_id()?.to_vec(),
                prove: true,
            })),
        })
    }
}
