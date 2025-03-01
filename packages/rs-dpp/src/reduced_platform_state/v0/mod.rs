use crate::block::block_info::BlockInfo;
use crate::block::extended_block_info::ExtendedBlockInfo;
use crate::fee::default_costs::EpochIndexFeeVersionsForStorage;
use crate::util::deserializer::ProtocolVersion;
use bincode::{Decode, Encode};
use platform_value::Bytes32;

/// Reduced Platform State for Saving V0
#[derive(Clone, Debug, Encode, Decode)]
pub struct ReducedPlatformStateForSavingV0 {
    /// Information about the last block
    pub last_committed_block_info: Option<ExtendedBlockInfo>,
    /// Current Version
    pub current_protocol_version_in_consensus: ProtocolVersion,
    /// upcoming protocol version
    pub next_epoch_protocol_version: ProtocolVersion,
    /// current quorum
    pub current_validator_set_quorum_hash: Bytes32,
    /// next quorum
    pub next_validator_set_quorum_hash: Option<Bytes32>,
    /// previous Fee Versions
    pub previous_fee_versions: EpochIndexFeeVersionsForStorage,
}
