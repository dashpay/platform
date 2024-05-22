use crate::execution::types::state_transition_container::v0::{
    DecodedStateTransition, InvalidStateTransition, InvalidWithProtocolErrorStateTransition,
    StateTransitionContainerV0, SuccessfullyDecodedStateTransition,
};
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::consensus::basic::decode::SerializedObjectParsingError;
use dpp::consensus::basic::state_transition::StateTransitionMaxSizeExceededError;
use dpp::consensus::basic::BasicError;
use dpp::consensus::ConsensusError;
use dpp::serialization::PlatformDeserializable;
use dpp::state_transition::StateTransition;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use std::time::{Duration, Instant};

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
        raw_state_transitions: &'a [impl AsRef<[u8]>],
        platform_version: &PlatformVersion,
    ) -> StateTransitionContainerV0<'a> {
        // Todo: might be better to have StateTransitionContainerV0 be a decoder instead and have
        //  the method decode_raw_state_transitions
        let decoded_state_transitions = raw_state_transitions
            .iter()
            .map(|raw_state_transition| {
                if raw_state_transition.as_ref().len() as u64
                    > platform_version
                        .dpp
                        .state_transitions
                        .max_state_transition_size
                {
                    // The state transition is too big
                    let consensus_error = ConsensusError::BasicError(
                        BasicError::StateTransitionMaxSizeExceededError(
                            StateTransitionMaxSizeExceededError::new(
                                raw_state_transition.as_ref().len() as u64,
                                platform_version
                                    .dpp
                                    .state_transitions
                                    .max_state_transition_size,
                            ),
                        ),
                    );

                    DecodedStateTransition::InvalidEncoding(InvalidStateTransition {
                        raw: raw_state_transition.as_ref(),
                        error: consensus_error,
                        elapsed_time: Duration::default(),
                    })
                } else {
                    let start_time = Instant::now();

                    match StateTransition::deserialize_from_bytes(raw_state_transition.as_ref()) {
                        Ok(state_transition) => DecodedStateTransition::SuccessfullyDecoded(
                            SuccessfullyDecodedStateTransition {
                                decoded: state_transition,
                                raw: raw_state_transition.as_ref(),
                                elapsed_time: start_time.elapsed(),
                            },
                        ),
                        Err(error) => match error {
                            ProtocolError::PlatformDeserializationError(message) => {
                                let consensus_error =
                                    SerializedObjectParsingError::new(message.clone()).into();

                                DecodedStateTransition::InvalidEncoding(InvalidStateTransition {
                                    raw: raw_state_transition.as_ref(),
                                    error: consensus_error,
                                    elapsed_time: start_time.elapsed(),
                                })
                            }
                            ProtocolError::MaxEncodedBytesReachedError { .. } => {
                                let message = error.to_string();
                                let consensus_error =
                                    SerializedObjectParsingError::new(message.clone()).into();

                                DecodedStateTransition::InvalidEncoding(InvalidStateTransition {
                                    raw: raw_state_transition.as_ref(),
                                    error: consensus_error,
                                    elapsed_time: start_time.elapsed(),
                                })
                            }
                            protocol_error => DecodedStateTransition::FailedToDecode(
                                InvalidWithProtocolErrorStateTransition {
                                    raw: raw_state_transition.as_ref(),
                                    error: protocol_error,
                                    elapsed_time: start_time.elapsed(),
                                },
                            ),
                        },
                    }
                }
            })
            .collect();

        StateTransitionContainerV0::new(decoded_state_transitions)
    }
}
