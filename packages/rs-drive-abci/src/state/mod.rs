use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::quorum::Quorum;
use crate::rpc::core::QuorumListExtendedInfo;
use dashcore_rpc::dashcore::{ProTxHash, QuorumHash};
use dashcore_rpc::dashcore_rpc_json::MasternodeListItem;
use dashcore_rpc::json::QuorumType;
use dpp::block::block_info::ExtendedBlockInfo;
use dpp::block::epoch::Epoch;

use drive::dpp::util::deserializer::ProtocolVersion;
use indexmap::IndexMap;
use std::collections::{BTreeMap, HashMap};

mod commit;
mod genesis;

/// Platform state
#[derive(Clone, Debug)]
pub struct PlatformState {
    /// Information about the last block
    pub last_committed_block_info: Option<ExtendedBlockInfo>,
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
#[derive(Clone, Debug)]
pub struct PlatformInitializationState {
    /// Core initialization height
    pub core_initialization_height: u32,
}

impl PlatformState {
    /// The default state at init chain
    pub fn default_with_protocol_versions(
        current_protocol_version_in_consensus: ProtocolVersion,
        next_epoch_protocol_version: ProtocolVersion,
    ) -> PlatformState {
        PlatformState {
            last_committed_block_info: None,
            current_protocol_version_in_consensus,
            next_epoch_protocol_version,
            quorums_extended_info: Default::default(),
            current_validator_set_quorum_hash: Default::default(),
            next_validator_set_quorum_hash: None,
            validator_sets: Default::default(),
            full_masternode_list: Default::default(),
            hpmn_masternode_list: Default::default(),
            initialization_information: None,
        }
    }
    /// The height of the platform, only committed blocks increase height
    pub fn height(&self) -> u64 {
        self.last_committed_block_info
            .as_ref()
            .map(|block_info| block_info.basic_info.height)
            .unwrap_or_default()
    }

    /// The height of the platform, only committed blocks increase height
    pub fn known_height_or(&self, default: u64) -> u64 {
        self.last_committed_block_info
            .as_ref()
            .map(|block_info| block_info.basic_info.height)
            .unwrap_or(default)
    }

    /// The height of the core blockchain that Platform knows about through chain locks
    pub fn core_height(&self) -> u32 {
        self.last_committed_block_info
            .as_ref()
            .map(|block_info| block_info.basic_info.core_height)
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
            .map(|block_info| block_info.basic_info.core_height)
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
            .map(|block_info| block_info.basic_info.time_ms)
    }

    /// The last quorum hash
    pub fn last_quorum_hash(&self) -> [u8; 32] {
        self.last_committed_block_info
            .as_ref()
            .map(|block_info| block_info.quorum_hash)
            .unwrap_or_default()
    }

    /// The last block signature
    pub fn last_block_signature(&self) -> [u8; 96] {
        self.last_committed_block_info
            .as_ref()
            .map(|block_info| block_info.signature)
            .unwrap_or([0u8; 96])
    }

    /// The last block round
    pub fn last_block_round(&self) -> u32 {
        self.last_committed_block_info
            .as_ref()
            .map(|block_info| block_info.round)
            .unwrap_or_default()
    }

    /// The current epoch
    pub fn epoch(&self) -> Epoch {
        self.last_committed_block_info
            .as_ref()
            .map(|block_info| block_info.basic_info.epoch)
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
