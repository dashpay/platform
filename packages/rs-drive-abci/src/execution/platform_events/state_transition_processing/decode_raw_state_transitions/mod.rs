mod v0;
mod v1;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::state_transition_container::StateTransitionContainer;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::version::PlatformVersion;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Processes the given raw state transitions based on the `block_info` and `transaction`.
    ///
    /// Version 0 ignores bytes left over after a transition; version 1 (from protocol version
    /// 14) refuses them as an invalid encoding.
    ///
    /// # Arguments
    ///
    /// * `raw_state_transitions` - A reference to a vector of raw state transitions.
    /// * `platform_version` - A `PlatformVersion` reference that dictates which version of
    ///   the method to call.
    ///
    /// # Returns
    ///
    /// * `Result<(FeeResult, Vec<ExecTxResult>), Error>` - If the processing is successful, it returns
    ///   a tuple consisting of a `FeeResult` and a vector of `ExecTxResult`. If the processing fails,
    ///   it returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function may return an `Error` variant if there is a problem with deserializing the raw
    /// state transitions, processing state transitions, or executing events.
    pub(in crate::execution) fn decode_raw_state_transitions<'a>(
        &self,
        raw_state_transitions: &'a [impl AsRef<[u8]>],
        platform_version: &PlatformVersion,
    ) -> Result<StateTransitionContainer<'a>, Error> {
        match platform_version
            .drive_abci
            .methods
            .state_transition_processing
            .decode_raw_state_transitions
        {
            0 => Ok(self
                .decode_raw_state_transitions_v0(raw_state_transitions, platform_version)
                .into()),
            1 => Ok(self
                .decode_raw_state_transitions_v1(raw_state_transitions, platform_version)
                .into()),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "decode_raw_state_transitions".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::execution::types::state_transition_container::v0::DecodedStateTransition;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::consensus::basic::BasicError;
    use dpp::consensus::ConsensusError;
    use dpp::identifier::Identifier;
    use dpp::platform_value::BinaryData;
    use dpp::serialization::PlatformSerializable;
    use dpp::state_transition::batch_transition::{BatchTransition, BatchTransitionV1};
    use dpp::state_transition::StateTransition;
    use dpp::version::PlatformVersion;

    /// Protocol version 13 decodes a transition with bytes after it as the transition alone;
    /// protocol version 14 refuses it. The exact bytes decode at both.
    #[test]
    fn should_refuse_bytes_after_a_transition_from_protocol_version_14_only() {
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let transition = StateTransition::Batch(BatchTransition::V1(BatchTransitionV1 {
            owner_id: Identifier::from([1u8; 32]),
            transitions: vec![],
            user_fee_increase: 0,
            signature_public_key_id: 0,
            signature: BinaryData::new(vec![0u8; 65]),
        }));
        let exact = transition.serialize_to_bytes().expect("serialize");
        let padded = |left_over: usize| {
            let mut padded = exact.clone();
            padded.extend(std::iter::repeat_n(0u8, left_over));
            padded
        };
        let raw_state_transitions = vec![exact.clone(), padded(1), padded(100)];

        let decode = |platform_version: &PlatformVersion| -> Vec<DecodedStateTransition> {
            let decoded: Vec<DecodedStateTransition> = platform
                .decode_raw_state_transitions(&raw_state_transitions, platform_version)
                .expect("known decoder version")
                .into();
            assert_eq!(decoded.len(), 3);
            decoded
        };

        let protocol_version_13 = PlatformVersion::get(13).expect("protocol version 13 exists");
        for decoded in decode(protocol_version_13) {
            match decoded {
                DecodedStateTransition::SuccessfullyDecoded(success) => {
                    assert_eq!(success.decoded, transition)
                }
                other => panic!("protocol version 13 decodes every variant, got {other:?}"),
            }
        }

        let protocol_version_14 = PlatformVersion::get(14).expect("protocol version 14 exists");
        let mut decoded = decode(protocol_version_14).into_iter();
        match decoded.next() {
            Some(DecodedStateTransition::SuccessfullyDecoded(success)) => {
                assert_eq!(success.decoded, transition)
            }
            other => panic!("protocol version 14 decodes the exact bytes, got {other:?}"),
        }
        for refused in decoded {
            match refused {
                DecodedStateTransition::InvalidEncoding(invalid) => assert!(
                    matches!(
                        invalid.error,
                        ConsensusError::BasicError(BasicError::SerializedObjectParsingError(_))
                    ),
                    "expected SerializedObjectParsingError, got {:?}",
                    invalid.error
                ),
                other => panic!("protocol version 14 refuses left over bytes, got {other:?}"),
            }
        }
    }
}
