use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::quorum::Quorum;
use crate::rpc::core::QuorumListExtendedInfo;
use dashcore_rpc::dashcore::{ProTxHash, QuorumHash};
use dashcore_rpc::dashcore_rpc_json::MasternodeListItem;
use dashcore_rpc::json::QuorumType;
use dpp::block::block_info::BlockInfo;
use dpp::block::epoch::Epoch;
use drive::dpp::util::deserializer::ProtocolVersion;
use indexmap::IndexMap;
use std::collections::{BTreeMap, HashMap};

mod genesis;

/// Platform state
#[derive(Clone)]
pub struct PlatformState {
    //todo: add quorum hash to block info
    /// Information about the last block
    pub last_committed_block_info: Option<BlockInfo>,
    /// Current Version
    pub current_protocol_version_in_consensus: ProtocolVersion,
    /// upcoming protocol version
    pub next_epoch_protocol_version: ProtocolVersion,
    /// current quorums
    pub quorums_extended_info: HashMap<QuorumType, QuorumListExtendedInfo>,
    /// current quorum
    pub current_validator_set_quorum_hash: QuorumHash,
    /// next quorum
    pub next_validator_set_quorum_hash: Option<QuorumHash>,
    /// current validator set quorums
    /// The validator set quorums are a subset of the quorums, but they also contain the list of
    /// all members
    pub validator_sets: IndexMap<QuorumHash, Quorum>,

    /// current full masternode list
    pub full_masternode_list: BTreeMap<ProTxHash, MasternodeListItem>,

    /// current HPMN masternode list
    pub hpmn_masternode_list: BTreeMap<ProTxHash, MasternodeListItem>,

    /// if we initialized the chain this block
    pub initialization_information: Option<PlatformInitializationState>,
}

/// Platform state for the first block
#[derive(Clone)]
pub struct PlatformInitializationState {
    /// Core initialization height
    pub core_initialization_height: u32,
}

impl PlatformState {
    /// The height of the platform, only committed blocks increase height
    pub fn height(&self) -> u64 {
        self.last_committed_block_info
            .as_ref()
            .map(|block_info| block_info.height)
            .unwrap_or_default()
    }

    /// The height of the platform, only committed blocks increase height
    pub fn known_height_or(&self, default: u64) -> u64 {
        self.last_committed_block_info
            .as_ref()
            .map(|block_info| block_info.height)
            .unwrap_or(default)
    }

    /// The height of the core blockchain that Platform knows about through chain locks
    pub fn core_height(&self) -> u32 {
        self.last_committed_block_info
            .as_ref()
            .map(|block_info| block_info.core_height)
            .unwrap_or_else(|| {
                self.initialization_information
                    .as_ref()
                    .map(|initialization_information| {
                        initialization_information.core_initialization_height
                    })
                    .unwrap_or_default()
            })
    }

    /// The height of the core blockchain that Platform knows about through chain locks
    pub fn known_core_height_or(&self, default: u32) -> u32 {
        self.last_committed_block_info
            .as_ref()
            .map(|block_info| block_info.core_height)
            .unwrap_or_else(|| {
                self.initialization_information
                    .as_ref()
                    .map(|initialization_information| {
                        initialization_information.core_initialization_height
                    })
                    .unwrap_or(default)
            })
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

    /// HPMN list len
    pub fn hpmn_list_len(&self) -> usize {
        self.hpmn_masternode_list.len()
    }

    /// Get the current quorum
    pub fn current_validator_set(&self) -> Result<&Quorum, Error> {
        self.validator_sets
            .get(&self.current_validator_set_quorum_hash)
            .ok_or(Error::Execution(ExecutionError::CorruptedCachedState(
                "current validator quorum hash not in current known validator sets",
            )))
    }
}
