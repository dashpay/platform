use crate::execution::types::state_transition_container::v0::StateTransitionContainerV0;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::consensus::basic::decode::SerializedObjectParsingError;
use dpp::consensus::basic::state_transition::StateTransitionMaxSizeExceededError;
use dpp::consensus::basic::BasicError;
use dpp::consensus::ConsensusError;
use dpp::dashcore::hashes::Hash;
use dpp::serialization::PlatformDeserializable;
use dpp::state_transition::StateTransition;
use dpp::version::PlatformVersion;
use dpp::{dashcore, ProtocolError};

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Decodes and processes raw state transitions for version 0.
    ///
    /// This method deserializes each raw state transition from the provided vector, attempting to process
    /// and categorize them into valid or invalid state transitions based on the deserialization outcome
    /// and specific validation rules. It encapsulates the results in a `StateTransitionContainerV0`,
    /// which separately tracks valid and invalid state transitions along with any associated errors.
    ///
    /// ## Arguments
    ///
    /// - `raw_state_transitions`: A reference to a vector of raw state transitions, where each state transition
    ///   is represented as a vector of bytes.
    ///
    /// ## Returns
    ///
    /// - `StateTransitionContainerV0`: A container holding the processed state transitions, including
    ///   both successfully deserialized transitions and those that failed validation with their corresponding errors.
    ///
    /// ## Errors
    ///
    /// Errors can arise from issues in deserializing the raw state transitions or if the processing of state transitions
    /// encounters a scenario that warrants halting further execution. Specific errors include deserialization failures,
    /// exceeding the maximum encoded bytes limit for a state transition. Protocol level errors should never occur, but
    /// are also included in the result container.
    ///
    pub(super) fn decode_raw_state_transitions_v0<'a>(
        &self,
        raw_state_transitions: &'a Vec<Vec<u8>>,
        platform_version: &PlatformVersion,
    ) -> StateTransitionContainerV0<'a> {
        // Todo: might be better to have StateTransitionContainerV0 be a decoder instead and have
        //  the method decode_raw_state_transitions
        let mut container = StateTransitionContainerV0::default();
        for raw_state_transition in raw_state_transitions {
            if raw_state_transition.len() as u64
                > platform_version
                    .dpp
                    .state_transitions
                    .max_state_transition_size
            {
                // The state transition is too big
                let consensus_error =
                    ConsensusError::BasicError(BasicError::StateTransitionMaxSizeExceededError(
                        StateTransitionMaxSizeExceededError::new(
                            raw_state_transition.len() as u64,
                            platform_version
                                .dpp
                                .state_transitions
                                .max_state_transition_size,
                        ),
                    ));
                tracing::debug!(?consensus_error, "State transition too big");

                container.push_invalid_raw_state_transition(raw_state_transition, consensus_error);
                continue;
            }

            match StateTransition::deserialize_from_bytes(raw_state_transition) {
                Ok(state_transition) => {
                    container.push_valid_state_transition(raw_state_transition, state_transition);
                }
                Err(error) => match error {
                    ProtocolError::PlatformDeserializationError(message) => {
                        let consensus_error =
                            SerializedObjectParsingError::new(message.clone()).into();
                        let errors = vec![&consensus_error];

                        tracing::debug!(
                            ?errors,
                            "Invalid unknown state transition ({}): {}",
                            hex::encode(
                                dashcore::hashes::sha256::Hash::hash(raw_state_transition)
                                    .to_byte_array(),
                            ),
                            message
                        );
                        container.push_invalid_raw_state_transition(
                            raw_state_transition,
                            consensus_error,
                        );
                    }
                    ProtocolError::MaxEncodedBytesReachedError { .. } => {
                        let message = error.to_string();
                        let consensus_error =
                            SerializedObjectParsingError::new(message.clone()).into();
                        let errors = vec![&consensus_error];

                        tracing::debug!(
                            ?errors,
                            "State transition beyond max encoded bytes limit ({}): {}",
                            hex::encode(
                                dashcore::hashes::sha256::Hash::hash(raw_state_transition)
                                    .to_byte_array(),
                            ),
                            message
                        );

                        container.push_invalid_raw_state_transition(
                            raw_state_transition,
                            consensus_error,
                        );
                    }
                    e => container.push_invalid_raw_state_transition_with_protocol_error(
                        raw_state_transition,
                        e,
                    ),
                },
            }
        }

        container
    }
}
