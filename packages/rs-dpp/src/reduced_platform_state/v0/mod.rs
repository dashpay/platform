use crate::block::block_info::BlockInfo;
use crate::fee::default_costs::EpochIndexFeeVersionsForStorage;
use crate::util::deserializer::ProtocolVersion;
use bincode::{Decode, Encode};
use platform_value::Bytes32;

/// Block information persisted as part of the reduced platform state.
///
/// The reduced state is written while the block is still being executed, before it is
/// signed and before the resulting app hash is known, so `app_hash`, `block_id_hash` and
/// `signature` are `Option`s rather than zero-filled placeholders. They are `None` when
/// stored and are filled in (where possible) during state reconstruction.
#[derive(Clone, Debug, PartialEq, Encode, Decode)]
pub struct ReducedBlockInfoV0 {
    /// Basic block info (height, core height, time, epoch)
    pub basic_info: BlockInfo,
    /// The app hash resulting from this block; unknown at store time
    pub app_hash: Option<Bytes32>,
    /// The quorum that signed (or will sign) this block
    pub quorum_hash: Bytes32,
    /// The block id hash; unknown at store time
    pub block_id_hash: Option<Bytes32>,
    /// The block proposer's pro tx hash
    pub proposer_pro_tx_hash: Bytes32,
    /// The block signature; unknown at store time
    pub signature: Option<[u8; 96]>,
    /// The consensus round that produced this block
    pub round: u32,
}

/// Reduced Platform State V0.
///
/// This minimal version of the Platform state is written into GroveDB (under the Misc
/// tree, hence below the root hash) on every block proposal. Because it is part of the
/// replicated state, a freshly state-synced node can read it back and reconstruct the
/// full in-memory Platform state, which is otherwise only persisted to non-replicated
/// GroveDB aux storage.
#[derive(Clone, Debug, PartialEq, Encode, Decode)]
pub struct ReducedPlatformStateV0 {
    /// Info about the block that was being processed when this state was written
    /// (it becomes the last committed block once the block finalizes)
    pub last_committed_block_info: Option<ReducedBlockInfoV0>,
    /// Current protocol version in consensus
    pub current_protocol_version_in_consensus: ProtocolVersion,
    /// Upcoming protocol version
    pub next_epoch_protocol_version: ProtocolVersion,
    /// Current validator set quorum hash
    pub current_validator_set_quorum_hash: Bytes32,
    /// Next validator set quorum hash
    pub next_validator_set_quorum_hash: Option<Bytes32>,
    /// Fee versions of previous epochs, stored by fee version number so they can be
    /// restored faithfully on reconstruction
    pub previous_fee_versions: EpochIndexFeeVersionsForStorage,
    /// Ordered list of quorum hashes reflecting validator set quorum positions
    // TODO: optimize this to not store the whole quorum hash, but only some index
    pub quorum_positions: Vec<Bytes32>,
    /// Core chain locked height, as provided in RequestProcessProposal ABCI message;
    /// note this can differ from the one in RequestPrepareProposal, as it can be
    /// modified by the proposer.
    pub proposed_core_chain_locked_height: u32,
}
