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

use dpp::state_transition::{validate_state_transition_size, StateTransition};
use dpp::version::PlatformVersion;

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
    /// Serializes the state transition into a [`BroadcastStateTransitionRequest`].
    ///
    /// The request is not checked against `max_state_transition_size`: DAPI refuses an
    /// oversized one with an untyped `InvalidArgument`. To build a request for sending, use
    /// [`checked_broadcast_request`], which refuses it with the typed
    /// `StateTransitionMaxSizeExceededError` first.
    ///
    /// # Returns
    /// On success, the serialized request. On failure, an [`Error`].
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
                // A proof of an owned, fee-paying transition carries the owner's balance itself.
                request_user_balance: false,
            })),
        })
    }
}

/// The broadcast request for `transition`, refused with the typed
/// `StateTransitionMaxSizeExceededError` if the transition is over
/// `max_state_transition_size` ([`validate_state_transition_size`]): DAPI
/// would refuse it with an untyped `InvalidArgument`. Every SDK path that
/// sends a transition builds its request through this, and so should a caller
/// that sends one through the DAPI client directly;
/// [`BroadcastRequestForStateTransition::broadcast_request_for_state_transition`]
/// alone does not check the size, because the wait path rebuilds the request
/// of a transition that has already executed.
pub fn checked_broadcast_request(
    transition: &StateTransition,
    platform_version: &PlatformVersion,
) -> Result<BroadcastStateTransitionRequest, Error> {
    let request = transition.broadcast_request_for_state_transition()?;
    validate_state_transition_size(request.state_transition.len() as u64, platform_version)?;
    Ok(request)
}

#[cfg(test)]
mod tests {
    use dpp::consensus::basic::BasicError;
    use dpp::consensus::ConsensusError;
    use dpp::platform_value::BinaryData;
    use dpp::state_transition::identity_topup_transition::v0::IdentityTopUpTransitionV0;
    use dpp::state_transition::identity_topup_transition::IdentityTopUpTransition;
    use dpp::state_transition::STATE_TRANSITION_MAX_ENCODED_BYTES;
    use dpp::ProtocolError;

    use super::*;

    fn max_size() -> usize {
        PlatformVersion::latest()
            .system_limits
            .max_state_transition_size as usize
    }

    fn is_size_exceeded(error: &Error, actual_size_bytes: impl Fn(u64) -> bool) -> bool {
        matches!(
            error,
            Error::Protocol(ProtocolError::ConsensusError(e)) if matches!(
                e.as_ref(),
                ConsensusError::BasicError(BasicError::StateTransitionMaxSizeExceededError(e))
                    if actual_size_bytes(e.actual_size_bytes())
            )
        )
    }

    /// An identity top-up of exactly `size_bytes` serialized bytes, padded
    /// through its signature.
    fn top_up_of_size(size_bytes: usize) -> StateTransition {
        let mut v0 = IdentityTopUpTransitionV0::default();
        let base = StateTransition::IdentityTopUp(IdentityTopUpTransition::V0(v0.clone()))
            .serialize_to_bytes()
            .expect("serializes")
            .len();
        // The signature's varint length prefix grows from 1 to 3 bytes once
        // the signature is 251 bytes or longer.
        v0.signature = BinaryData::new(vec![0u8; size_bytes - base - 2]);
        let transition = StateTransition::IdentityTopUp(IdentityTopUpTransition::V0(v0));
        assert_eq!(
            transition.serialize_to_bytes().expect("serializes").len(),
            size_bytes
        );
        transition
    }

    #[test]
    fn should_build_the_request_for_a_transition_at_max_state_transition_size() {
        let request =
            checked_broadcast_request(&top_up_of_size(max_size()), PlatformVersion::latest())
                .expect("a transition at the limit is accepted");

        assert_eq!(request.state_transition.len(), max_size());
    }

    #[test]
    fn should_refuse_an_oversized_transition_with_the_typed_size_error() {
        let error =
            checked_broadcast_request(&top_up_of_size(max_size() + 1), PlatformVersion::latest())
                .expect_err("one byte over the limit is refused");

        assert!(
            is_size_exceeded(&error, |actual| actual == max_size() as u64 + 1),
            "unexpected error: {error:?}"
        );
    }

    /// A transition past `STATE_TRANSITION_MAX_ENCODED_BYTES` (an IS proof
    /// over a funding tx of ~500 inputs) still encodes, so it gets the same
    /// typed size error with its real length.
    #[test]
    fn should_refuse_a_transition_past_the_encoded_bytes_budget_with_the_typed_size_error() {
        let transition = StateTransition::IdentityTopUp(IdentityTopUpTransition::V0(
            IdentityTopUpTransitionV0 {
                signature: BinaryData::new(vec![0u8; STATE_TRANSITION_MAX_ENCODED_BYTES]),
                ..Default::default()
            },
        ));

        let error = checked_broadcast_request(&transition, PlatformVersion::latest())
            .expect_err("an oversized transition is refused");

        assert!(
            is_size_exceeded(&error, |actual| actual
                > STATE_TRANSITION_MAX_ENCODED_BYTES as u64),
            "unexpected error: {error:?}"
        );
    }
}
