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
    use dpp::version::PlatformVersion;

    /// The last protocol version whose tables select the v0 decoder.
    const LAST_V0_DECODER_PROTOCOL_VERSION: u32 = 16;

    /// Both generations reject an ordinary transition above the ordinary cap with the same
    /// error, and both let one at the cap through to the decode step.
    #[test]
    fn should_bound_ordinary_transitions_identically_through_both_generations() {
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();
        let v0_version =
            PlatformVersion::get(LAST_V0_DECODER_PROTOCOL_VERSION).expect("protocol version 16");
        let v1_version = PlatformVersion::latest();
        assert_eq!(
            v0_version
                .drive_abci
                .methods
                .state_transition_processing
                .decode_raw_state_transitions,
            0
        );
        assert_eq!(
            v1_version
                .drive_abci
                .methods
                .state_transition_processing
                .decode_raw_state_transitions,
            1
        );
        let max_size = v1_version.system_limits.max_state_transition_size as usize;
        assert_eq!(
            max_size as u64,
            v0_version.system_limits.max_state_transition_size
        );

        // Outer index 2 is Batch: an ordinary family through both generations. The filler is
        // not zero because bincode decodes trailing zeroes as valid empty fields.
        let mut oversized = vec![0xFFu8; max_size + 1];
        oversized[0] = 2;
        let mut at_limit = vec![0xFFu8; max_size];
        at_limit[0] = 2;
        let raw_state_transitions = vec![oversized, at_limit];

        for platform_version in [v0_version, v1_version] {
            let container = platform
                .decode_raw_state_transitions(&raw_state_transitions, platform_version)
                .expect("expected the dispatcher to select a decoder");
            let decoded: Vec<_> = container.into_iter().collect();
            assert_eq!(decoded.len(), 2);
            match &decoded[0] {
                DecodedStateTransition::InvalidEncoding(invalid) => assert!(matches!(
                    &invalid.error,
                    ConsensusError::BasicError(BasicError::StateTransitionMaxSizeExceededError(_))
                )),
                other => panic!(
                    "protocol version {}: expected the oversized rejection, got {other:?}",
                    platform_version.protocol_version
                ),
            }
            match &decoded[1] {
                DecodedStateTransition::InvalidEncoding(invalid) => assert!(matches!(
                    &invalid.error,
                    ConsensusError::BasicError(BasicError::SerializedObjectParsingError(_))
                )),
                other => panic!(
                    "protocol version {}: expected a parsing error, got {other:?}",
                    platform_version.protocol_version
                ),
            }
        }
    }

    /// A contract-code capable envelope is bounded by the ordinary cap through v0 (the family
    /// cap does not exist there) and by the family cap through v1.
    #[test]
    fn should_bound_a_contract_code_capable_envelope_only_from_the_v1_generation() {
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();
        let v0_version =
            PlatformVersion::get(LAST_V0_DECODER_PROTOCOL_VERSION).expect("protocol version 16");
        let v1_version = PlatformVersion::latest();
        let ordinary_cap = v1_version.system_limits.max_state_transition_size as usize;

        let mut envelope = vec![0xFFu8; ordinary_cap + 1];
        envelope[0] = 0;
        envelope[1] = 1;
        let raw_state_transitions = vec![envelope];

        let container = platform
            .decode_raw_state_transitions(&raw_state_transitions, v0_version)
            .expect("v0 decoder");
        let decoded: Vec<_> = container.into_iter().collect();
        assert!(
            matches!(
                &decoded[0],
                DecodedStateTransition::InvalidEncoding(invalid)
                    if matches!(
                        &invalid.error,
                        ConsensusError::BasicError(BasicError::StateTransitionMaxSizeExceededError(_))
                    )
            ),
            "the v0 decoder knows no family cap"
        );

        let container = platform
            .decode_raw_state_transitions(&raw_state_transitions, v1_version)
            .expect("v1 decoder");
        let decoded: Vec<_> = container.into_iter().collect();
        assert!(
            matches!(
                &decoded[0],
                DecodedStateTransition::InvalidEncoding(invalid)
                    if matches!(
                        &invalid.error,
                        ConsensusError::BasicError(BasicError::SerializedObjectParsingError(_))
                    )
            ),
            "the v1 decoder lets a contract-code envelope above the ordinary cap reach the decode step"
        );
    }
}
