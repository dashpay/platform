use crate::rpc::core::QuorumListExtendedInfo;
use dashcore_rpc::dashcore_rpc_json::{ProTxHash, QuorumHash, QuorumInfoResult};
use dashcore_rpc::json::{QuorumMasternodeListItem, QuorumType};
use drive::dpp::util::deserializer::ProtocolVersion;
use drive::drive::block_info::BlockInfo;
use drive::fee_pools::epochs::Epoch;
use std::collections::{BTreeMap, HashMap};

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
    pub quorums_extended_info: HashMap<QuorumType, QuorumListExtendedInfo>,
    /// current validator set quorums
    /// The validator set quorums are a subset of the quorums, but they also contain the list of
    /// all members
    pub validator_sets: HashMap<QuorumHash, QuorumInfoResult>,

    /// current full masternode list
    pub full_masternode_list: BTreeMap<ProTxHash, QuorumMasternodeListItem>,

    /// current hpmn masternode list
    pub hpmn_masternode_list: BTreeMap<ProTxHash, QuorumMasternodeListItem>,
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
            .unwrap_or_default()
    }

    /// The height of the core blockchain that Platform knows about through chain locks
    pub fn known_core_height_or(&self, default: u32) -> u32 {
        self.last_committed_block_info
            .as_ref()
            .map(|block_info| block_info.core_height)
            .unwrap_or(default)
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
}
