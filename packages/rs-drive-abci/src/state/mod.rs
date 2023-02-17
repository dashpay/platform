use drive::dpp::util::deserializer::ProtocolVersion;
use drive::drive::block_info::BlockInfo;

mod genesis;

/// Platform state
#[derive(Clone)]
pub struct PlatformState {
    /// Information about the last block
    pub last_block_info: Option<BlockInfo>,
    /// Current Version
    pub current_protocol_version_in_consensus: ProtocolVersion,
    /// upcoming protocol version
    pub next_epoch_protocol_version: ProtocolVersion,
}
