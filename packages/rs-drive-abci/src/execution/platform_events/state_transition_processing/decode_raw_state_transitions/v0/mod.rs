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
                    > platform_version.system_limits.max_state_transition_size
                {
                    // The state transition is too big
                    let consensus_error = ConsensusError::BasicError(
                        BasicError::StateTransitionMaxSizeExceededError(
                            StateTransitionMaxSizeExceededError::new(
                                raw_state_transition.as_ref().len() as u64,
                                platform_version.system_limits.max_state_transition_size,
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

                    match StateTransition::deserialize_from_bytes_in_version(
                        raw_state_transition.as_ref(),
                        platform_version,
                    ) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::version::PlatformVersion;

    #[test]
    fn test_decode_empty_state_transitions() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let raw_state_transitions: Vec<Vec<u8>> = vec![];
        let container =
            platform.decode_raw_state_transitions_v0(&raw_state_transitions, platform_version);

        assert_eq!(container.into_iter().count(), 0);
    }

    #[test]
    fn test_decode_oversized_state_transition() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        // Create a state transition that exceeds the max size
        let max_size = platform_version.system_limits.max_state_transition_size as usize;
        let oversized = vec![0u8; max_size + 1];

        let raw_state_transitions = vec![oversized];
        let container =
            platform.decode_raw_state_transitions_v0(&raw_state_transitions, platform_version);

        let decoded: Vec<_> = container.into_iter().collect();
        assert_eq!(decoded.len(), 1);

        match &decoded[0] {
            DecodedStateTransition::InvalidEncoding(invalid) => {
                assert!(
                    matches!(
                        &invalid.error,
                        dpp::consensus::ConsensusError::BasicError(
                            dpp::consensus::basic::BasicError::StateTransitionMaxSizeExceededError(
                                _
                            )
                        )
                    ),
                    "expected StateTransitionMaxSizeExceededError"
                );
            }
            _ => panic!("expected InvalidEncoding for oversized state transition"),
        }
    }

    #[test]
    fn test_decode_invalid_bytes_state_transition() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        // Random garbage bytes that won't deserialize as a valid state transition
        let garbage = vec![0xFF, 0xFE, 0xFD, 0xFC, 0xFB];

        let raw_state_transitions = vec![garbage];
        let container =
            platform.decode_raw_state_transitions_v0(&raw_state_transitions, platform_version);

        let decoded: Vec<_> = container.into_iter().collect();
        assert_eq!(decoded.len(), 1);

        // Should be either InvalidEncoding (PlatformDeserializationError) or FailedToDecode
        match &decoded[0] {
            DecodedStateTransition::InvalidEncoding(_) => {}
            DecodedStateTransition::FailedToDecode(_) => {}
            DecodedStateTransition::SuccessfullyDecoded(_) => {
                panic!("garbage bytes should not decode successfully")
            }
        }
    }

    #[test]
    fn test_decode_multiple_mixed_state_transitions() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let max_size = platform_version.system_limits.max_state_transition_size as usize;
        let oversized = vec![0u8; max_size + 1];
        let garbage = vec![0xFF, 0xFE, 0xFD];

        let raw_state_transitions = vec![oversized, garbage];
        let container =
            platform.decode_raw_state_transitions_v0(&raw_state_transitions, platform_version);

        let decoded: Vec<_> = container.into_iter().collect();
        assert_eq!(decoded.len(), 2);

        // First should be oversized error
        match &decoded[0] {
            DecodedStateTransition::InvalidEncoding(_) => {}
            _ => panic!("first should be InvalidEncoding for oversized"),
        }

        // Second should be invalid encoding or failed to decode
        match &decoded[1] {
            DecodedStateTransition::InvalidEncoding(_) => {}
            DecodedStateTransition::FailedToDecode(_) => {}
            DecodedStateTransition::SuccessfullyDecoded(_) => {
                panic!("garbage should not decode successfully")
            }
        }
    }

    #[test]
    fn test_decode_state_transition_at_exact_max_size_is_not_rejected_as_oversized() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        // Create a state transition that is exactly at the max size limit.
        // It should attempt to decode (not be rejected as oversized).
        let max_size = platform_version.system_limits.max_state_transition_size as usize;
        let at_limit = vec![0u8; max_size];

        let raw_state_transitions = vec![at_limit];
        let container =
            platform.decode_raw_state_transitions_v0(&raw_state_transitions, platform_version);

        let decoded: Vec<_> = container.into_iter().collect();
        assert_eq!(decoded.len(), 1);

        // Should NOT be rejected as oversized - it should pass the size check
        // and attempt deserialization. Any result other than the size error is acceptable.
        if let DecodedStateTransition::InvalidEncoding(ref inv) = decoded[0] {
            assert!(
                !matches!(
                    &inv.error,
                    ConsensusError::BasicError(
                        BasicError::StateTransitionMaxSizeExceededError(_)
                    )
                ),
                "buffer at exactly max size should not be rejected by the size check"
            );
        }
    }
}
