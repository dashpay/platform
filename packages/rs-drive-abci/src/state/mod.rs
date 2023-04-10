use crate::rpc::core::QuorumListExtendedInfo;
use dashcore_rpc::json::QuorumType;
use drive::dpp::util::deserializer::ProtocolVersion;
use drive::drive::block_info::BlockInfo;
use drive::fee_pools::epochs::Epoch;
use std::collections::HashMap;

mod genesis;

/// Platform state
#[derive(Clone)]
pub struct PlatformState {
    /// Information about the last block
    pub last_committed_block_info: Option<BlockInfo>,
    /// The current Epoch
    pub current_epoch: Epoch,
    /// Current Version
    pub current_protocol_version_in_consensus: ProtocolVersion,
    /// upcoming protocol version
    pub next_epoch_protocol_version: ProtocolVersion,
    /// current quorums
    pub quorums: HashMap<QuorumType, QuorumListExtendedInfo>,
}

impl PlatformState {
    /// The height of the platform, only committed blocks increase height
    pub fn height(&self) -> u64 {
        self.last_committed_block_info
            .as_ref()
            .map(|block_info| block_info.height)
            .unwrap_or_default()
    }

    /// The height of the core blockchain that Platform knows about through chain locks
    pub fn core_height(&self) -> u32 {
        self.last_committed_block_info
            .as_ref()
            .map(|block_info| block_info.core_height)
            .unwrap_or_default()
    }

    /// The last block time in milliseconds
    pub fn last_block_time_ms(&self) -> Option<u64> {
        self.last_committed_block_info
            .as_ref()
            .map(|block_info| block_info.time_ms)
    }

    /// The current epoch
    pub fn epoch(&self) -> Epoch {
        self.last_committed_block_info
            .as_ref()
            .map(|block_info| block_info.epoch)
            .unwrap_or_default()
    }
}
