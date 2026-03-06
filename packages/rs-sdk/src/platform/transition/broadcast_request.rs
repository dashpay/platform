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
/// To broadcast a [`StateTransition`] and wait for
/// Platform to confirm it, use the higher-level
/// [`BroadcastStateTransition`](super::broadcast::BroadcastStateTransition) trait which wraps
/// this trait with retry logic, error handling, and proof verification:
///
/// ```rust,ignore
/// use dash_sdk::platform::transition::broadcast::BroadcastStateTransition;
///
/// // Assume `sdk` is a connected Sdk instance and
/// // `state_transition` is an already-constructed and signed StateTransition.
/// state_transition.broadcast_and_wait(&sdk, None).await?;
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
