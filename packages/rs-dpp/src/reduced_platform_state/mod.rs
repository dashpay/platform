//! Reduced platform state
//!
//! A minimal subset of the Platform state that is stored inside the replicated GroveDB
//! state (under the Misc tree), allowing a node that syncs via ABCI state sync to
//! reconstruct the full Platform state. The full Platform state itself is only persisted
//! to GroveDB aux storage, which is not replicated by GroveDB state sync.

use crate::ProtocolError;
use bincode::{Decode, Encode};
use derive_more::From;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};

pub mod v0;

use v0::ReducedPlatformStateV0;

/// Reduced Platform State (platform-versioned wrapper)
///
/// The structure version is the enum discriminant, so it serializes `unversioned` (big
/// endian, no limit) exactly like the other versioned platform types. These bytes are
/// covered by the app hash, so the encoding is consensus-fixed.
#[derive(Clone, Debug, PartialEq, Encode, Decode, PlatformSerialize, PlatformDeserialize, From)]
#[platform_serialize(unversioned)]
pub enum ReducedPlatformState {
    /// Version 0
    V0(ReducedPlatformStateV0),
}

#[cfg(test)]
mod tests {
    use super::v0::{
        ReducedBlockInfoV0, ReducedPlatformStateV0, ReducedPreviousQuorumsV0,
        ReducedVerificationQuorumV0,
    };
    use super::*;
    use crate::block::block_info::BlockInfo;
    use crate::serialization::{PlatformDeserializable, PlatformSerializable};

    #[test]
    fn should_roundtrip_reduced_platform_state_serialization() {
        let state = ReducedPlatformState::V0(ReducedPlatformStateV0 {
            last_committed_block_info: Some(ReducedBlockInfoV0 {
                basic_info: BlockInfo::default_with_time(1_700_000_000_000),
                quorum_hash: [1u8; 32].into(),
                proposer_pro_tx_hash: [2u8; 32].into(),
            }),
            current_protocol_version_in_consensus: 15,
            next_epoch_protocol_version: 15,
            current_validator_set_quorum_hash: [4u8; 32].into(),
            next_validator_set_quorum_hash: Some([5u8; 32].into()),
            previous_fee_versions: [(0u16, 1u32)].into_iter().collect(),
            quorum_positions: vec![[4u8; 32].into(), [5u8; 32].into()],
            proposed_core_chain_locked_height: 1000,
            previous_chain_lock_quorums: Some(ReducedPreviousQuorumsV0 {
                quorums: vec![ReducedVerificationQuorumV0 {
                    quorum_hash: [6u8; 32].into(),
                    public_key: [7u8; 48],
                    index: None,
                }],
                last_active_core_height: 990,
                updated_at_core_height: 995,
                previous_change_height: Some(900),
            }),
            previous_instant_lock_quorums: Some(ReducedPreviousQuorumsV0 {
                quorums: vec![ReducedVerificationQuorumV0 {
                    quorum_hash: [8u8; 32].into(),
                    public_key: [9u8; 48],
                    index: Some(3),
                }],
                last_active_core_height: 991,
                updated_at_core_height: 996,
                previous_change_height: None,
            }),
        });

        let bytes = state.serialize_to_bytes().expect("should serialize");
        let restored =
            ReducedPlatformState::deserialize_from_bytes(&bytes).expect("should deserialize");

        assert_eq!(state, restored);
    }

    /// The reduced state is encoded like every other versioned platform type: big
    /// endian. Pin it so the app-hash-covered encoding cannot drift silently.
    #[test]
    fn should_encode_big_endian_like_other_platform_types() {
        let state = ReducedPlatformState::V0(ReducedPlatformStateV0 {
            last_committed_block_info: None,
            current_protocol_version_in_consensus: 0x0102_0304,
            next_epoch_protocol_version: 0,
            current_validator_set_quorum_hash: [0u8; 32].into(),
            next_validator_set_quorum_hash: None,
            previous_fee_versions: Default::default(),
            quorum_positions: vec![],
            proposed_core_chain_locked_height: 0,
            previous_chain_lock_quorums: None,
            previous_instant_lock_quorums: None,
        });

        let bytes = state.serialize_to_bytes().expect("should serialize");
        // discriminant 0, then `None`, then the protocol version as a big-endian varint
        // (bincode's varint marker 0xfc precedes a u32 payload)
        assert_eq!(&bytes[..2], &[0u8, 0u8]);
        assert_eq!(&bytes[2..7], &[0xfc, 0x01, 0x02, 0x03, 0x04]);
    }
}
