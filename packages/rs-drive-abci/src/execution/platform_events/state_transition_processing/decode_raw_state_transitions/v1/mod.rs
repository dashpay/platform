use crate::execution::types::state_transition_container::v0::{
    DecodedStateTransition, InvalidStateTransition, InvalidWithProtocolErrorStateTransition,
    StateTransitionContainerV0, SuccessfullyDecodedStateTransition,
};
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::consensus::basic::decode::SerializedObjectParsingError;
use dpp::consensus::basic::state_transition::{
    StateTransitionFamilyMaxSizeExceededError, StateTransitionMaxSizeExceededError,
};
use dpp::consensus::basic::BasicError;
use dpp::consensus::ConsensusError;
use dpp::state_transition::envelope_kind::StateTransitionEnvelopeKind;
use dpp::state_transition::StateTransition;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use std::time::{Duration, Instant};

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Decodes raw state transitions, version 1: the size cap and the decode budget depend on
    /// the family the wire prefix names.
    ///
    /// Three things differ from version 0:
    ///
    /// * The raw length is compared with the cap of the family
    ///   `StateTransition::peek_envelope_kind` reads from the first bytes: the contract-code
    ///   capable generations of the contract create and update transitions are bounded by
    ///   `SystemLimits::max_contract_code_state_transition_size` and rejected with
    ///   `StateTransitionFamilyMaxSizeExceededError`, every other family by
    ///   `max_state_transition_size` and `StateTransitionMaxSizeExceededError` as before.
    /// * The bytes are decoded through `deserialize_from_bytes_in_version_bounded`, which
    ///   applies the family's bincode budget; ordinary families keep the shipped decode.
    /// * A variant the active protocol version does not admit comes back as a consensus error
    ///   and is filed as `InvalidEncoding` (an unpaid consensus rejection) rather than as
    ///   `FailedToDecode` (a node fault): a transition a later protocol version introduces is a
    ///   rejection two nodes agree on, not a broken build.
    ///
    /// ## Arguments
    ///
    /// - `raw_state_transitions`: the raw state transitions of the block, in block order.
    ///
    /// ## Returns
    ///
    /// - `StateTransitionContainerV0`: the decoded and the rejected transitions, in block
    ///   order, each rejection with the error that classifies it.
    pub(super) fn decode_raw_state_transitions_v1<'a>(
        &self,
        raw_state_transitions: &'a [impl AsRef<[u8]>],
        platform_version: &PlatformVersion,
    ) -> StateTransitionContainerV0<'a> {
        let decoded_state_transitions = raw_state_transitions
            .iter()
            .map(|raw_state_transition| {
                let raw = raw_state_transition.as_ref();
                let kind = StateTransition::peek_envelope_kind(raw);
                let max_size = StateTransition::family_max_size(kind, platform_version);
                if raw.len() as u64 > max_size {
                    let consensus_error = match kind {
                        StateTransitionEnvelopeKind::Ordinary => ConsensusError::BasicError(
                            BasicError::StateTransitionMaxSizeExceededError(
                                StateTransitionMaxSizeExceededError::new(
                                    raw.len() as u64,
                                    max_size,
                                ),
                            ),
                        ),
                        StateTransitionEnvelopeKind::ContractCodeCapable { family } => {
                            ConsensusError::BasicError(
                                BasicError::StateTransitionFamilyMaxSizeExceededError(
                                    StateTransitionFamilyMaxSizeExceededError::new(
                                        family.to_string(),
                                        raw.len() as u64,
                                        max_size,
                                    ),
                                ),
                            )
                        }
                    };

                    return DecodedStateTransition::InvalidEncoding(InvalidStateTransition {
                        raw,
                        error: consensus_error,
                        elapsed_time: Duration::default(),
                    });
                }

                let start_time = Instant::now();

                match StateTransition::deserialize_from_bytes_in_version_bounded(
                    raw,
                    platform_version,
                ) {
                    Ok(state_transition) => DecodedStateTransition::SuccessfullyDecoded(
                        SuccessfullyDecodedStateTransition {
                            decoded: state_transition,
                            raw,
                            elapsed_time: start_time.elapsed(),
                        },
                    ),
                    Err(ProtocolError::PlatformDeserializationError(message)) => {
                        DecodedStateTransition::InvalidEncoding(InvalidStateTransition {
                            raw,
                            error: SerializedObjectParsingError::new(message).into(),
                            elapsed_time: start_time.elapsed(),
                        })
                    }
                    Err(error @ ProtocolError::MaxEncodedBytesReachedError { .. }) => {
                        DecodedStateTransition::InvalidEncoding(InvalidStateTransition {
                            raw,
                            error: SerializedObjectParsingError::new(error.to_string()).into(),
                            elapsed_time: start_time.elapsed(),
                        })
                    }
                    Err(ProtocolError::ConsensusError(consensus_error)) => {
                        DecodedStateTransition::InvalidEncoding(InvalidStateTransition {
                            raw,
                            error: *consensus_error,
                            elapsed_time: start_time.elapsed(),
                        })
                    }
                    Err(protocol_error) => DecodedStateTransition::FailedToDecode(
                        InvalidWithProtocolErrorStateTransition {
                            raw,
                            error: protocol_error,
                            elapsed_time: start_time.elapsed(),
                        },
                    ),
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
    use dpp::consensus::codes::ErrorWithCode;
    use dpp::data_contract::config::v0::DataContractConfigV0;
    use dpp::data_contract::config::DataContractConfig;
    use dpp::data_contract::serialized_version::DataContractInSerializationFormat;
    use dpp::data_contract::v1::DataContractV1;
    use dpp::data_contract::DataContract;
    use dpp::identifier::Identifier;
    use dpp::platform_value::BinaryData;
    use dpp::serialization::PlatformSerializable;
    use dpp::state_transition::data_contract_create_transition::{
        DataContractCreateTransition, DataContractCreateTransitionV0,
    };
    use dpp::version::TryIntoPlatformVersioned;
    use std::collections::BTreeMap;

    /// The prefix of a contract create transition in the contract-code capable generation:
    /// outer index 0 (`DataContractCreate`), inner index 1.
    const CONTRACT_CODE_CAPABLE_CREATE_PREFIX: [u8; 2] = [0, 1];

    /// A synthetic envelope: the given prefix followed by bytes no state transition decodes
    /// from (bincode decodes trailing zeroes as valid empty fields, so the filler is not zero).
    fn envelope_with_prefix(prefix: &[u8], len: usize) -> Vec<u8> {
        let mut envelope = vec![0xFFu8; len];
        envelope[..prefix.len()].copy_from_slice(prefix);
        envelope
    }

    fn contract_code_capable_envelope(len: usize) -> Vec<u8> {
        envelope_with_prefix(&CONTRACT_CODE_CAPABLE_CREATE_PREFIX, len)
    }

    fn format_v1_contract_create() -> StateTransition {
        let contract = DataContract::V1(DataContractV1 {
            id: Identifier::from([9u8; 32]),
            version: 1,
            owner_id: Identifier::from([7u8; 32]),
            document_types: BTreeMap::new(),
            config: DataContractConfig::V0(DataContractConfigV0 {
                can_be_deleted: false,
                readonly: false,
                keeps_history: false,
                documents_keep_history_contract_default: false,
                documents_mutable_contract_default: false,
                documents_can_be_deleted_contract_default: false,
                requires_identity_encryption_bounded_key: None,
                requires_identity_decryption_bounded_key: None,
            }),
            schema_defs: None,
            created_at: None,
            updated_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            created_at_epoch: None,
            updated_at_epoch: None,
            groups: BTreeMap::new(),
            tokens: BTreeMap::new(),
            keywords: Vec::new(),
            description: None,
        });
        let data_contract: DataContractInSerializationFormat = contract
            .try_into_platform_versioned(PlatformVersion::latest())
            .expect("expected to serialize a trivial contract");
        assert!(matches!(
            data_contract,
            DataContractInSerializationFormat::V1(_)
        ));
        StateTransition::DataContractCreate(DataContractCreateTransition::V0(
            DataContractCreateTransitionV0 {
                data_contract,
                identity_nonce: 1,
                user_fee_increase: 0,
                signature_public_key_id: 0,
                signature: BinaryData::new(vec![0u8; 65]),
            },
        ))
    }

    #[test]
    fn should_decode_an_empty_list() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let raw_state_transitions: Vec<Vec<u8>> = vec![];
        let container =
            platform.decode_raw_state_transitions_v1(&raw_state_transitions, platform_version);

        assert_eq!(container.into_iter().count(), 0);
    }

    #[test]
    fn should_keep_the_ordinary_cap_for_every_ordinary_family() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let max_size = platform_version.system_limits.max_state_transition_size as usize;
        // Batch is outer index 2: an ordinary family whatever its inner index.
        let oversized = envelope_with_prefix(&[2, 1], max_size + 1);
        let at_limit = envelope_with_prefix(&[2, 1], max_size);

        let raw_state_transitions = vec![oversized, at_limit];
        let container =
            platform.decode_raw_state_transitions_v1(&raw_state_transitions, platform_version);
        let decoded: Vec<_> = container.into_iter().collect();
        assert_eq!(decoded.len(), 2);

        match &decoded[0] {
            DecodedStateTransition::InvalidEncoding(invalid) => match &invalid.error {
                ConsensusError::BasicError(BasicError::StateTransitionMaxSizeExceededError(
                    error,
                )) => {
                    assert_eq!(error.actual_size_bytes(), max_size as u64 + 1);
                    assert_eq!(error.max_size_bytes(), max_size as u64);
                }
                other => panic!("expected StateTransitionMaxSizeExceededError, got {other:?}"),
            },
            other => panic!("expected InvalidEncoding, got {other:?}"),
        }
        match &decoded[1] {
            DecodedStateTransition::InvalidEncoding(invalid) => assert!(
                !matches!(
                    &invalid.error,
                    ConsensusError::BasicError(BasicError::StateTransitionMaxSizeExceededError(_))
                        | ConsensusError::BasicError(
                            BasicError::StateTransitionFamilyMaxSizeExceededError(_)
                        )
                ),
                "a buffer at the ordinary cap passes the size gate and fails as garbage"
            ),
            other => panic!("expected InvalidEncoding, got {other:?}"),
        }
    }

    #[test]
    fn should_keep_the_ordinary_cap_for_v0_contract_transitions() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let max_size = platform_version.system_limits.max_state_transition_size as usize;
        // Outer index 0 (create) and 1 (update) with inner index 0: the original generation.
        let oversized_create = envelope_with_prefix(&[0, 0], max_size + 1);
        let oversized_update = envelope_with_prefix(&[1, 0], max_size + 1);

        let raw_state_transitions = vec![oversized_create, oversized_update];
        let container =
            platform.decode_raw_state_transitions_v1(&raw_state_transitions, platform_version);
        for decoded in container.into_iter() {
            match decoded {
                DecodedStateTransition::InvalidEncoding(invalid) => assert!(
                    matches!(
                        &invalid.error,
                        ConsensusError::BasicError(
                            BasicError::StateTransitionMaxSizeExceededError(_)
                        )
                    ),
                    "a V0 contract transition keeps the ordinary cap, got {:?}",
                    invalid.error
                ),
                other => panic!("expected InvalidEncoding, got {other:?}"),
            }
        }
    }

    #[test]
    fn should_bound_contract_code_capable_envelopes_by_the_family_cap() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let family_cap = platform_version
            .system_limits
            .max_contract_code_state_transition_size
            .expect("the latest version bounds contract code envelopes")
            as usize;
        assert!(family_cap > platform_version.system_limits.max_state_transition_size as usize);

        let raw_state_transitions = vec![
            contract_code_capable_envelope(family_cap + 1),
            contract_code_capable_envelope(family_cap),
        ];
        let container =
            platform.decode_raw_state_transitions_v1(&raw_state_transitions, platform_version);
        let decoded: Vec<_> = container.into_iter().collect();
        assert_eq!(decoded.len(), 2);

        match &decoded[0] {
            DecodedStateTransition::InvalidEncoding(invalid) => match &invalid.error {
                ConsensusError::BasicError(
                    BasicError::StateTransitionFamilyMaxSizeExceededError(error),
                ) => {
                    assert_eq!(error.family(), "DataContractCreate");
                    assert_eq!(error.actual_size_bytes(), family_cap as u64 + 1);
                    assert_eq!(error.max_size_bytes(), family_cap as u64);
                    assert_eq!(invalid.error.code(), 10604);
                }
                other => {
                    panic!("expected StateTransitionFamilyMaxSizeExceededError, got {other:?}")
                }
            },
            other => panic!("expected InvalidEncoding, got {other:?}"),
        }
        // At the cap the envelope passes the size gate and fails decode as garbage: never as
        // oversized and never as a node fault.
        match &decoded[1] {
            DecodedStateTransition::InvalidEncoding(invalid) => assert!(
                matches!(
                    &invalid.error,
                    ConsensusError::BasicError(BasicError::SerializedObjectParsingError(_))
                ),
                "expected a parsing error, got {:?}",
                invalid.error
            ),
            other => panic!("expected InvalidEncoding, got {other:?}"),
        }
    }

    #[test]
    fn should_classify_not_active_as_invalid_encoding() {
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        // A format-V1 contract is active from protocol version 9. Run the v1 decoder against
        // the tables of protocol version 8 to reach the not-active branch.
        let too_early = PlatformVersion::get(8).expect("protocol version 8");
        let bytes = format_v1_contract_create()
            .serialize_to_bytes()
            .expect("serialize");

        let raw_state_transitions = vec![bytes];
        let container = platform.decode_raw_state_transitions_v1(&raw_state_transitions, too_early);
        let decoded: Vec<_> = container.into_iter().collect();
        assert_eq!(decoded.len(), 1);
        match &decoded[0] {
            DecodedStateTransition::InvalidEncoding(invalid) => match &invalid.error {
                ConsensusError::BasicError(BasicError::StateTransitionNotActiveError(error)) => {
                    assert_eq!(error.state_transition_type(), "DataContractCreate");
                    assert_eq!(error.current_protocol_version(), 8);
                    assert_eq!(error.required_protocol_version(), 9);
                }
                other => panic!("expected StateTransitionNotActiveError, got {other:?}"),
            },
            other => panic!("expected InvalidEncoding, got {other:?}"),
        }

        // The same bytes decode at the latest version.
        let container = platform
            .decode_raw_state_transitions_v1(&raw_state_transitions, PlatformVersion::latest());
        let decoded: Vec<_> = container.into_iter().collect();
        assert!(matches!(
            &decoded[0],
            DecodedStateTransition::SuccessfullyDecoded(_)
        ));
    }

    #[test]
    fn should_decode_garbage_and_empty_bytes_as_invalid_encoding_not_oversized() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let raw_state_transitions: Vec<Vec<u8>> = vec![vec![0xFF, 0xFE, 0xFD], vec![]];
        let container =
            platform.decode_raw_state_transitions_v1(&raw_state_transitions, platform_version);
        for decoded in container.into_iter() {
            match decoded {
                DecodedStateTransition::InvalidEncoding(invalid) => assert!(
                    !matches!(
                        &invalid.error,
                        ConsensusError::BasicError(
                            BasicError::StateTransitionMaxSizeExceededError(_)
                        )
                    ),
                    "garbage must not be rejected by the size check"
                ),
                DecodedStateTransition::FailedToDecode(_) => {}
                DecodedStateTransition::SuccessfullyDecoded(_) => {
                    panic!("garbage must not decode")
                }
            }
        }
    }
}
