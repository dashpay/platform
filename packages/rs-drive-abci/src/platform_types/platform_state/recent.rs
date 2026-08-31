//! The part of the platform state that changes on every block.
//!
//! The full saved state is over a megabyte on mainnet — masternode lists,
//! validator sets and the chain-lock and instant-lock quorum sets — and those
//! parts only change when Core's masternode list or quorums do. This record
//! carries the rest, so a block that changed nothing heavy writes a couple of
//! hundred bytes instead of rewriting the whole state.

use crate::platform_types::platform_state::PlatformState;
use bincode::{Decode, Encode};
use dpp::block::block_info::BlockInfo;
use dpp::block::extended_block_info::v0::ExtendedBlockInfoV0Getters;
use dpp::block::extended_block_info::ExtendedBlockInfo;
use dpp::dashcore::hashes::Hash;
use dpp::dashcore::QuorumHash;
use dpp::platform_value::Bytes32;
use dpp::util::deserializer::ProtocolVersion;

/// Versioned per-block platform state record.
#[derive(Clone, Debug, Encode, Decode)]
pub enum PlatformStateRecent {
    /// Version 0
    V0(PlatformStateRecentV0),
}

/// Version 0 of the per-block platform state record.
#[derive(Clone, Debug, Encode, Decode)]
pub struct PlatformStateRecentV0 {
    /// Information about the genesis block
    pub genesis_block_info: Option<BlockInfo>,
    /// Information about the last block
    pub last_committed_block_info: Option<ExtendedBlockInfo>,
    /// Current version
    pub current_protocol_version_in_consensus: ProtocolVersion,
    /// Upcoming protocol version
    pub next_epoch_protocol_version: ProtocolVersion,
    /// Current quorum
    pub current_validator_set_quorum_hash: Bytes32,
    /// Next quorum
    pub next_validator_set_quorum_hash: Option<Bytes32>,
}

impl From<&PlatformState> for PlatformStateRecent {
    fn from(state: &PlatformState) -> Self {
        PlatformStateRecent::V0(PlatformStateRecentV0 {
            genesis_block_info: state.genesis_block_info,
            last_committed_block_info: state.last_committed_block_info.clone(),
            current_protocol_version_in_consensus: state.current_protocol_version_in_consensus,
            next_epoch_protocol_version: state.next_epoch_protocol_version,
            current_validator_set_quorum_hash: state
                .current_validator_set_quorum_hash
                .to_byte_array()
                .into(),
            next_validator_set_quorum_hash: state
                .next_validator_set_quorum_hash
                .map(|hash| hash.to_byte_array().into()),
        })
    }
}

impl PlatformStateRecent {
    /// Overwrite the per-block fields of `state` with the ones in this record.
    ///
    /// The heavy fields are left alone: they came from a full record written at
    /// or before the height this record was written at, and are unchanged since.
    pub fn apply_to(self, state: &mut PlatformState) {
        let PlatformStateRecent::V0(v0) = self;
        state.genesis_block_info = v0.genesis_block_info;
        state.last_committed_block_info = v0.last_committed_block_info;
        state.current_protocol_version_in_consensus = v0.current_protocol_version_in_consensus;
        state.next_epoch_protocol_version = v0.next_epoch_protocol_version;
        state.current_validator_set_quorum_hash =
            QuorumHash::from_byte_array(v0.current_validator_set_quorum_hash.to_buffer());
        state.next_validator_set_quorum_hash = v0
            .next_validator_set_quorum_hash
            .map(|bytes| QuorumHash::from_byte_array(bytes.to_buffer()));
    }

    /// The height this record was written at, if it has block info.
    pub fn height(&self) -> Option<u64> {
        let PlatformStateRecent::V0(v0) = self;
        v0.last_committed_block_info
            .as_ref()
            .map(|info| info.basic_info().height)
    }
}
