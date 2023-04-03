use drive::dpp::util::deserializer::ProtocolVersion;
use drive::drive::block_info::BlockInfo;
use drive::fee_pools::epochs::Epoch;

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
}

impl PlatformState {
    /// The height of the platform, only committed blocks increase height
    pub fn height(&self) -> u64 {
        self.last_committed_block_info.unwrap_or_default().height
    }

    /// The height of the core blockchain that Platform knows about through chain locks
    pub fn core_height(&self) -> u32 {
        self.last_committed_block_info
            .unwrap_or_default()
            .core_height
    }

    /// The last block time in milliseconds
    pub fn last_block_time_ms(&self) -> u64 {
        self.last_committed_block_info.unwrap_or_default().time_ms
    }

    /// The current epoch
    pub fn epoch(&self) -> Epoch {
        self.last_committed_block_info.unwrap_or_default().epoch
    }
}
