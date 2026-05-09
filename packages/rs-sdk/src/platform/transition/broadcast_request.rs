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

    fn wait_for_state_transition_result_request_with_hash(
        &self,
        transition_hash: [u8; 32],
    ) -> Result<WaitForStateTransitionResultRequest, Error> {
        Ok(WaitForStateTransitionResultRequest {
            version: Some(Version::V0(WaitForStateTransitionResultRequestV0 {
                state_transition_hash: transition_hash.to_vec(),
                prove: true,
            })),
        })
    }
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
        self.wait_for_state_transition_result_request_with_hash(self.transaction_id()?)
    }
}

#[cfg(test)]
mod tests {
    use super::BroadcastRequestForStateTransition;
    use crate::Error;
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    use dapi_grpc::platform::v0::wait_for_state_transition_result_request::Version;
    use dapi_grpc::platform::v0::BroadcastStateTransitionRequest;
    use dpp::serialization::PlatformDeserializable;
    use dpp::state_transition::StateTransition;
    use std::fmt::{Debug, Formatter};

    #[derive(Clone)]
    struct TestTransition;

    impl Debug for TestTransition {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            f.write_str("TestTransition")
        }
    }

    impl BroadcastRequestForStateTransition for TestTransition {
        fn broadcast_request_for_state_transition(
            &self,
        ) -> Result<BroadcastStateTransitionRequest, Error> {
            Ok(BroadcastStateTransitionRequest {
                state_transition: vec![],
            })
        }

        fn wait_for_state_transition_result_request(
            &self,
        ) -> Result<dapi_grpc::platform::v0::WaitForStateTransitionResultRequest, Error> {
            self.wait_for_state_transition_result_request_with_hash([1; 32])
        }
    }

    #[test]
    fn wait_request_with_hash_uses_precomputed_hash() {
        let request = TestTransition
            .wait_for_state_transition_result_request_with_hash([9; 32])
            .expect("request should build");

        let Some(Version::V0(v0)) = request.version else {
            panic!("expected v0 request");
        };

        assert_eq!(v0.state_transition_hash, [9; 32]);
        assert!(v0.prove);
    }

    #[test]
    fn wait_request_for_state_transition_uses_transaction_id() {
        const RAW_TRANSACTION_BASE64: &str = "AwADAAAAAAAAACEDeLqSkwVyfHvYThgegiZUvPu0+dU4kyd3PJKigGLC1spBH+wrzjjA/ZGZdQmUzpQyOiC3GyP2eBp8ga9cNlnIOkptMzAtfXPA2daH3xTqt25JQ+fZ6UKB3ypzTK3fOXaAATgAAQAAAgAAIQPoVeBC6iyS0jFV0Dly5WV0SEl6uDciQqqi4EATeUJutEEfAd6+/HbUM4FLS6+lNc6AH8vaD9lViiYny4GPsl/AlBxdr0WjJxxU/B0cNVH8kRMo+W6a+1iSN+NZS7MTyzmTHwACAAEDAAAhA6S0TKbm1a/xyrYMG+Y2odspJ1roL1TcoK9h552yE1VCQSA+KpHiQ8lDBseXI/1ZCMxEvu0qopdjDojaQ4FzaZMgUGfPBeXSfMbQGksLMNseKRBLob/g0DHJWqZAxSDOuAwZAfwAIQxGIDIHY9cjWxS0tJupeJuKMZwzFKmLxkU3NmqFTcFscilVAABBH9R3vwbfA3q5XJG4m4z87OAA1uG8wup915wGGKAxdEObXPSqIvPBWrHlGTf/Uymanc2cDH1uKdsniJyoORwauPBIqlz61/Kf9HDnubX4GoHRYdnb4WzE+Tdh+L39a2dN2A==";
        let raw_transaction = STANDARD
            .decode(RAW_TRANSACTION_BASE64)
            .expect("base64 transition should decode");
        let state_transition = StateTransition::deserialize_from_bytes(&raw_transaction)
            .expect("state transition should deserialize");
        let transaction_id = state_transition
            .transaction_id()
            .expect("transaction id should compute");

        let request = state_transition
            .wait_for_state_transition_result_request()
            .expect("request should build");

        let Some(Version::V0(v0)) = request.version else {
            panic!("expected v0 request");
        };

        assert_eq!(v0.state_transition_hash, transaction_id.to_vec());
        assert!(v0.prove);
    }
}
