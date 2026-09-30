//! The part of the platform state that changes from block to block.
//!
//! The full saved state is over a megabyte on mainnet — masternode lists,
//! validator sets and the chain-lock and instant-lock quorum sets — and those
//! parts only change when Core's masternode list or quorums do. This record
//! carries the block info, written every block, and the validator set quorum
//! hashes, which rotate every few blocks, so a block that changed nothing
//! heavy writes a couple of hundred bytes instead of rewriting the whole state.
//!
//! Everything else in the state changes rarely enough that a change just
//! rewrites the full record: the protocol versions move once per epoch at
//! most, and the genesis block info is cleared before the first store and
//! never written as anything but `None`.

use crate::error::Error;
use crate::platform_types::platform_state::PlatformState;
use bincode::{Decode, Encode};
use dpp::block::extended_block_info::v0::ExtendedBlockInfoV0Getters;
use dpp::block::extended_block_info::ExtendedBlockInfo;
use dpp::dashcore::hashes::Hash;
use dpp::dashcore::QuorumHash;
use dpp::platform_value::Bytes32;
use dpp::ProtocolError;

/// Versioned per-block platform state record.
#[derive(Clone, Debug, Encode, Decode)]
pub enum PlatformStateRecent {
    /// Version 0
    V0(PlatformStateRecentV0),
}

/// Version 0 of the per-block platform state record.
#[derive(Clone, Debug, Encode, Decode)]
pub struct PlatformStateRecentV0 {
    /// Information about the last block
    pub last_committed_block_info: Option<ExtendedBlockInfo>,
    /// Current quorum
    pub current_validator_set_quorum_hash: Bytes32,
    /// Next quorum
    pub next_validator_set_quorum_hash: Option<Bytes32>,
}

impl From<&PlatformState> for PlatformStateRecent {
    fn from(state: &PlatformState) -> Self {
        PlatformStateRecent::V0(PlatformStateRecentV0 {
            last_committed_block_info: state.last_committed_block_info.clone(),
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
    fn bincode_config() -> bincode::config::Configuration<
        bincode::config::BigEndian,
        bincode::config::Varint,
        bincode::config::NoLimit,
    > {
        bincode::config::standard()
            .with_big_endian()
            .with_no_limit()
    }

    /// Encodes the record for the `saved_state_recent` aux key.
    pub fn serialize_to_bytes(&self) -> Result<Vec<u8>, Error> {
        bincode::encode_to_vec(self, Self::bincode_config()).map_err(|e| {
            Error::Protocol(ProtocolError::PlatformSerializationError(format!(
                "unable to serialize recent platform state: {e}"
            )))
        })
    }

    /// Decodes a record written by [`serialize_to_bytes`](Self::serialize_to_bytes).
    pub fn deserialize(bytes: &[u8]) -> Result<Self, Error> {
        bincode::decode_from_slice(bytes, Self::bincode_config())
            .map(|(record, _)| record)
            .map_err(|e| {
                Error::Protocol(ProtocolError::PlatformDeserializationError(format!(
                    "unable to deserialize recent platform state: {e}"
                )))
            })
    }

    /// Overwrite the per-block fields of `state` with the ones in this record.
    ///
    /// Every other field is left alone: it came from a full record written at
    /// or before the height this record was written at, and is unchanged since.
    pub fn apply_to(self, state: &mut PlatformState) {
        let PlatformStateRecent::V0(v0) = self;
        state.last_committed_block_info = v0.last_committed_block_info;
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
