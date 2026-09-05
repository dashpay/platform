//! Reduced platform state
//!
//! A minimal subset of the Platform state that is stored inside the replicated GroveDB
//! state (under the Misc tree), allowing a node that syncs via ABCI state sync to
//! reconstruct the full Platform state. The full Platform state itself is only persisted
//! to GroveDB aux storage, which is not replicated by GroveDB state sync.

use crate::serialization::{PlatformDeserializableFromVersionedStructure, PlatformSerializable};
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_version::version::PlatformVersion;

pub mod v0;

use v0::ReducedPlatformStateV0;

/// Reduced Platform State (platform-versioned wrapper)
#[derive(Clone, Debug, PartialEq, Encode, Decode, derive_more::From)]
pub enum ReducedPlatformState {
    /// Version 0
    V0(ReducedPlatformStateV0),
}

impl PlatformSerializable for ReducedPlatformState {
    type Error = ProtocolError;

    fn serialize_to_bytes(&self) -> Result<Vec<u8>, Self::Error> {
        let config = bincode::config::standard();
        bincode::encode_to_vec(self, config).map_err(|e| {
            ProtocolError::PlatformSerializationError(format!(
                "cannot serialize ReducedPlatformState: {}",
                e
            ))
        })
    }
}

impl PlatformDeserializableFromVersionedStructure for ReducedPlatformState {
    fn versioned_deserialize(
        data: &[u8],
        _platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized,
    {
        // The version of the structure is encoded in the enum discriminant, so the
        // platform version is not needed to pick the variant.
        let config = bincode::config::standard();
        bincode::decode_from_slice(data, config)
            .map_err(|e| {
                ProtocolError::PlatformDeserializationError(format!(
                    "cannot deserialize ReducedPlatformState: {}",
                    e
                ))
            })
            .map(|(object, _)| object)
    }
}

#[cfg(test)]
mod tests {
    use super::v0::{
        ReducedBlockInfoV0, ReducedPlatformStateV0, ReducedPreviousQuorumsV0,
        ReducedVerificationQuorumV0,
    };
    use super::*;
    use crate::block::block_info::BlockInfo;

    #[test]
    fn should_roundtrip_reduced_platform_state_serialization() {
        let state = ReducedPlatformState::V0(ReducedPlatformStateV0 {
            last_committed_block_info: Some(ReducedBlockInfoV0 {
                basic_info: BlockInfo::default_with_time(1_700_000_000_000),
                app_hash: None,
                quorum_hash: [1u8; 32].into(),
                block_id_hash: None,
                proposer_pro_tx_hash: [2u8; 32].into(),
                signature: None,
                round: 3,
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
            ReducedPlatformState::versioned_deserialize(&bytes, PlatformVersion::latest())
                .expect("should deserialize");

        assert_eq!(state, restored);
    }
}
