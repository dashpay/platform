use crate::execution::types::state_transition_container::v0::{
    DecodedStateTransition, InvalidStateTransition, InvalidWithProtocolErrorStateTransition,
    StateTransitionContainerV0, SuccessfullyDecodedStateTransition,
};
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::consensus::basic::decode::SerializedObjectParsingError;
use dpp::consensus::basic::state_transition::{
    StateTransitionMaxSizeExceededError, StateTransitionNotActiveError,
};
use dpp::consensus::basic::BasicError;
use dpp::consensus::ConsensusError;
use dpp::state_transition::errors::StateTransitionError;
use dpp::state_transition::StateTransition;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use std::time::{Duration, Instant};

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Decodes raw state transitions like version 0, except that each one must be exactly one
    /// encoded transition.
    ///
    /// Version 0 ignores bytes left over after the transition, so the same transition with
    /// anything appended decoded, validated and executed as the original. Here left over bytes
    /// are an `InvalidEncoding` carrying a `SerializedObjectParsingError`, which is unpaid. A
    /// transition whose version is not active at `platform_version` is an `InvalidEncoding`
    /// carrying a `StateTransitionNotActiveError`, not a decode failure.
    ///
    /// ## Arguments
    ///
    /// - `raw_state_transitions`: the raw state transitions, each as its bytes.
    /// - `platform_version`: the active platform version.
    ///
    /// ## Returns
    ///
    /// - `StateTransitionContainerV0`: every transition, decoded or with the reason it could not
    ///   be, in input order.
    pub(super) fn decode_raw_state_transitions_v1<'a>(
        &self,
        raw_state_transitions: &'a [impl AsRef<[u8]>],
        platform_version: &PlatformVersion,
    ) -> StateTransitionContainerV0<'a> {
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

                    match StateTransition::deserialize_from_bytes_untrusted_exact_in_version(
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
                            // Also the error for bytes left over after the transition.
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
                            // A transition whose version is not active yet decoded fine; its
                            // bytes are not at fault, and the submitter is owed a coded answer
                            // rather than an internal error. Matched by variant: the sibling
                            // `InvalidStateTransitionError` carries its own consensus errors and
                            // is a separate question.
                            ProtocolError::StateTransitionError(
                                StateTransitionError::StateTransitionIsNotActiveError {
                                    state_transition_type,
                                    active_version_range,
                                    current_protocol_version,
                                },
                            ) => {
                                // The range can be missed from either side. Below its start, the
                                // start is the version to reach. Above its end, the end is the
                                // last protocol version that accepted these bytes; naming the
                                // start there would point at a version the chain is already past.
                                let boundary =
                                    if current_protocol_version > *active_version_range.end() {
                                        *active_version_range.end()
                                    } else {
                                        *active_version_range.start()
                                    };

                                let consensus_error = StateTransitionNotActiveError::new(
                                    state_transition_type,
                                    current_protocol_version,
                                    boundary,
                                )
                                .into();

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
    use dpp::identifier::Identifier;
    use dpp::platform_value::BinaryData;
    use dpp::serialization::PlatformSerializable;
    use dpp::state_transition::batch_transition::{BatchTransition, BatchTransitionV1};
    use dpp::version::PlatformVersion;

    fn batch_transition(owner_byte: u8) -> StateTransition {
        StateTransition::Batch(BatchTransition::V1(BatchTransitionV1 {
            owner_id: Identifier::from([owner_byte; 32]),
            transitions: vec![],
            user_fee_increase: 0,
            signature_public_key_id: 0,
            signature: BinaryData::new(vec![0u8; 65]),
        }))
    }

    fn assert_serialized_object_parsing_error(decoded: &DecodedStateTransition) {
        match decoded {
            DecodedStateTransition::InvalidEncoding(invalid) => assert!(
                matches!(
                    &invalid.error,
                    ConsensusError::BasicError(BasicError::SerializedObjectParsingError(_))
                ),
                "expected SerializedObjectParsingError, got {:?}",
                invalid.error
            ),
            other => panic!("expected InvalidEncoding, got {other:?}"),
        }
    }

    #[test]
    fn should_decode_the_exact_bytes_of_a_transition() {
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let transition = batch_transition(1);
        let raw_state_transitions = vec![transition.serialize_to_bytes().expect("serialize")];
        let decoded: Vec<_> = platform
            .decode_raw_state_transitions_v1(&raw_state_transitions, PlatformVersion::latest())
            .into_iter()
            .collect();

        match decoded.as_slice() {
            [DecodedStateTransition::SuccessfullyDecoded(success)] => {
                assert_eq!(success.decoded, transition);
                assert_eq!(success.raw, raw_state_transitions[0].as_slice());
            }
            other => panic!("expected one decoded transition, got {other:?}"),
        }
    }

    #[test]
    fn should_refuse_bytes_left_over_after_a_transition() {
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let bytes = batch_transition(1).serialize_to_bytes().expect("serialize");
        let raw_state_transitions: Vec<Vec<u8>> = [1usize, 100]
            .into_iter()
            .map(|left_over| {
                let mut padded = bytes.clone();
                padded.extend(std::iter::repeat_n(0u8, left_over));
                padded
            })
            .collect();

        let decoded: Vec<_> = platform
            .decode_raw_state_transitions_v1(&raw_state_transitions, PlatformVersion::latest())
            .into_iter()
            .collect();

        assert_eq!(decoded.len(), 2);
        decoded
            .iter()
            .for_each(assert_serialized_object_parsing_error);
    }

    /// Two transitions in one raw transaction are refused as a whole, not decoded as the first.
    #[test]
    fn should_refuse_a_transition_followed_by_another() {
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let mut both = batch_transition(1).serialize_to_bytes().expect("serialize");
        both.extend(batch_transition(2).serialize_to_bytes().expect("serialize"));

        let raw_state_transitions = vec![both];
        let decoded: Vec<_> = platform
            .decode_raw_state_transitions_v1(&raw_state_transitions, PlatformVersion::latest())
            .into_iter()
            .collect();

        assert_eq!(decoded.len(), 1);
        assert_serialized_object_parsing_error(&decoded[0]);
    }

    /// The size limit is still checked first, on the whole raw transaction.
    #[test]
    fn should_refuse_an_oversized_transaction_before_decoding() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let max_size = platform_version.system_limits.max_state_transition_size as usize;
        let mut oversized = batch_transition(1).serialize_to_bytes().expect("serialize");
        oversized.resize(max_size + 1, 0);

        let raw_state_transitions = vec![oversized];
        let decoded: Vec<_> = platform
            .decode_raw_state_transitions_v1(&raw_state_transitions, platform_version)
            .into_iter()
            .collect();

        match decoded.as_slice() {
            [DecodedStateTransition::InvalidEncoding(invalid)] => assert!(
                matches!(
                    &invalid.error,
                    ConsensusError::BasicError(BasicError::StateTransitionMaxSizeExceededError(_))
                ),
                "expected StateTransitionMaxSizeExceededError, got {:?}",
                invalid.error
            ),
            other => panic!("expected one InvalidEncoding, got {other:?}"),
        }
    }

    /// A transition that decodes cleanly but whose version is not active is a coded consensus
    /// rejection, not a decode failure (which would reach the internal-error path).
    #[test]
    fn should_report_an_inactive_transition_as_not_active() {
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        // A batch transition version 1 is active only from protocol version 9.
        let raw_state_transitions =
            vec![batch_transition(1).serialize_to_bytes().expect("serialize")];
        let older = PlatformVersion::get(1).expect("protocol version 1 should exist");
        let decoded: Vec<_> = platform
            .decode_raw_state_transitions_v1(&raw_state_transitions, older)
            .into_iter()
            .collect();

        match decoded.as_slice() {
            [DecodedStateTransition::InvalidEncoding(invalid)] => assert!(
                matches!(
                    &invalid.error,
                    ConsensusError::BasicError(BasicError::StateTransitionNotActiveError(_))
                ),
                "expected StateTransitionNotActiveError, got {:?}",
                invalid.error
            ),
            other => panic!("expected one InvalidEncoding, got {other:?}"),
        }
    }
}
