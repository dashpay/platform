use crate::block::block_info::BlockInfo;
use crate::util::deserializer::ProtocolVersion;
use crate::{
    block::extended_block_info::ExtendedBlockInfo,
    fee::default_costs::EpochIndexFeeVersionsForStorage,
};
use bincode::{Decode, Encode};
use platform_value::Bytes32;

/// Reduced Platform State for Saving V0
/// This minimal version of Platform state is written in GroveDB under the root hash.
/// This allows a freshly new synced node to reconstruct Platform state.
#[derive(Clone, Debug, Encode, Decode)]
pub struct ReducedPlatformStateForSavingV0 {
    /// The last committed block info (at height `H-1`)
    pub last_committed_block_info: Option<ExtendedBlockInfo>,

    /// Currently processed block info (at height `H`)
    pub current_block_info: BlockInfo,

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

    /// ordered list of quorum hashes that reflect quorum positions
    /// TODO: optimize this to not store the whole quorum hash, but only some index
    pub quorum_positions: Vec<Vec<u8>>,

    /// Core chain locked height, as provided in RequestProcessProposal ABCI message;
    /// note this can differ from the one in RequestPrepareProposal, as it can be modified by
    /// proposer.
    pub proposed_core_chain_locked_height: u32,
}
