use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::rpc::core::QuorumListExtendedInfo;
use dashcore_rpc::dashcore::{ProTxHash, QuorumHash};
use dashcore_rpc::dashcore_rpc_json::{ExtendedQuorumDetails, MasternodeListItem};
use dashcore_rpc::json::QuorumType;
use dpp::block::block_info::ExtendedBlockInfo;
use dpp::block::epoch::Epoch;

use dpp::bincode::{config, Decode, Encode};
use dpp::dashcore::hashes::Hash;
use dpp::platform_serialization::{PlatformDeserialize, PlatformSerialize};
use dpp::platform_value::Bytes32;
use dpp::serialization_traits::{PlatformDeserializable, PlatformSerializable};
use dpp::ProtocolError;
use drive::dpp::util::deserializer::ProtocolVersion;
use indexmap::IndexMap;

use crate::platform_types::masternode;
use crate::platform_types::validator_set::v0::ValidatorSet;
use std::collections::{BTreeMap, HashMap};

/// Platform state
#[derive(Clone, Debug, PlatformSerialize, PlatformDeserialize)]
#[platform_serialize_into(PlatformStateForSaving)]
#[platform_deserialize_from(PlatformStateForSaving)]
#[platform_error_type(ProtocolError)]
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
    pub validator_sets: IndexMap<QuorumHash, ValidatorSet>,

    /// current full masternode list
    pub full_masternode_list: BTreeMap<ProTxHash, MasternodeListItem>,

    /// current HPMN masternode list
    pub hpmn_masternode_list: BTreeMap<ProTxHash, MasternodeListItem>,

    /// if we initialized the chain this block
    pub initialization_information: Option<PlatformInitializationState>,
}

/// Platform state
#[derive(Clone, Debug, Encode, Decode)]
pub struct PlatformStateForSaving {
    /// Information about the last block
    pub last_committed_block_info: Option<ExtendedBlockInfo>,
    /// Current Version
    pub current_protocol_version_in_consensus: ProtocolVersion,
    /// upcoming protocol version
    pub next_epoch_protocol_version: ProtocolVersion,
    /// current quorums
    pub quorums_extended_info: Vec<(QuorumType, Vec<(Bytes32, ExtendedQuorumDetails)>)>,
    /// current quorum
    pub current_validator_set_quorum_hash: Bytes32,
    /// next quorum
    pub next_validator_set_quorum_hash: Option<Bytes32>,
    /// current validator set quorums
    /// The validator set quorums are a subset of the quorums, but they also contain the list of
    /// all members
    #[bincode(with_serde)]
    pub validator_sets: Vec<(Bytes32, ValidatorSet)>,

    /// current full masternode list
    pub full_masternode_list: BTreeMap<Bytes32, masternode::v0::Masternode>,

    /// current HPMN masternode list
    pub hpmn_masternode_list: BTreeMap<Bytes32, masternode::v0::Masternode>,

    /// if we initialized the chain this block
    pub initialization_information: Option<PlatformInitializationState>,
}

impl From<PlatformState> for PlatformStateForSaving {
    fn from(value: PlatformState) -> Self {
        PlatformStateForSaving {
            last_committed_block_info: value.last_committed_block_info,
            current_protocol_version_in_consensus: value.current_protocol_version_in_consensus,
            next_epoch_protocol_version: value.next_epoch_protocol_version,
            quorums_extended_info: value
                .quorums_extended_info
                .into_iter()
                .map(|(quorum_type, quorum_extended_info)| {
                    (
                        quorum_type,
                        quorum_extended_info
                            .into_iter()
                            .map(|(k, v)| (k.into_inner().into(), v))
                            .collect(),
                    )
                })
                .collect(),
            current_validator_set_quorum_hash: value
                .current_validator_set_quorum_hash
                .into_inner()
                .into(),
            next_validator_set_quorum_hash: value
                .next_validator_set_quorum_hash
                .map(|quorum_hash| quorum_hash.into_inner().into()),
            validator_sets: value
                .validator_sets
                .into_iter()
                .map(|(k, v)| (k.into_inner().into(), v))
                .collect(),
            full_masternode_list: value
                .full_masternode_list
                .into_iter()
                .map(|(k, v)| (k.into_inner().into(), v.into()))
                .collect(),
            hpmn_masternode_list: value
                .hpmn_masternode_list
                .into_iter()
                .map(|(k, v)| (k.into_inner().into(), v.into()))
                .collect(),
            initialization_information: value.initialization_information,
        }
    }
}

impl TryFrom<PlatformStateForSaving> for PlatformState {
    type Error = ProtocolError;

    fn try_from(value: PlatformStateForSaving) -> Result<Self, Self::Error> {
        Ok(PlatformState {
            last_committed_block_info: value.last_committed_block_info,
            current_protocol_version_in_consensus: value.current_protocol_version_in_consensus,
            next_epoch_protocol_version: value.next_epoch_protocol_version,
            quorums_extended_info: value
                .quorums_extended_info
                .into_iter()
                .map(|(quorum_type, quorum_extended_info)| {
                    (
                        quorum_type,
                        quorum_extended_info
                            .into_iter()
                            .map(|(k, v)| (QuorumHash::from_inner(k.to_buffer()), v))
                            .collect(),
                    )
                })
                .collect(),
            current_validator_set_quorum_hash: QuorumHash::from_inner(
                value.current_validator_set_quorum_hash.to_buffer(),
            ),
            next_validator_set_quorum_hash: value
                .next_validator_set_quorum_hash
                .map(|bytes| QuorumHash::from_inner(bytes.to_buffer())),
            validator_sets: value
                .validator_sets
                .into_iter()
                .map(|(k, v)| (QuorumHash::from_inner(k.to_buffer()), v))
                .collect(),
            full_masternode_list: value
                .full_masternode_list
                .into_iter()
                .map(|(k, v)| (ProTxHash::from_inner(k.to_buffer()), v.into()))
                .collect(),
            hpmn_masternode_list: value
                .hpmn_masternode_list
                .into_iter()
                .map(|(k, v)| (ProTxHash::from_inner(k.to_buffer()), v.into()))
                .collect(),
            initialization_information: value.initialization_information,
        })
    }
}

/// Platform state for the first block
#[derive(Clone, Debug, Encode, Decode)]
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

    /// The last block id hash
    pub fn last_block_id_hash(&self) -> [u8; 32] {
        self.last_committed_block_info
            .as_ref()
            .map(|block_info| block_info.block_id_hash)
            .unwrap_or_default()
    }

    /// The last block signature
    pub fn last_block_signature(&self) -> [u8; 96] {
        self.last_committed_block_info
            .as_ref()
            .map(|block_info| block_info.signature)
            .unwrap_or([0u8; 96])
    }

    /// The last block app hash
    pub fn last_block_app_hash(&self) -> Option<[u8; 32]> {
        self.last_committed_block_info
            .as_ref()
            .map(|block_info| block_info.app_hash)
    }

    /// The last block height or 0 for genesis
    pub fn last_block_height(&self) -> u64 {
        self.last_committed_block_info
            .as_ref()
            .map(|block_info| block_info.basic_info.height)
            .unwrap_or_default()
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
    pub fn current_validator_set(&self) -> Result<&ValidatorSet, Error> {
        self.validator_sets
            .get(&self.current_validator_set_quorum_hash)
            .ok_or(Error::Execution(ExecutionError::CorruptedCachedState(
                "current validator quorum hash not in current known validator sets",
            )))
    }
}
