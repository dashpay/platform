use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::rpc::core::QuorumListExtendedInfo;
use dashcore_rpc::dashcore::{ProTxHash, QuorumHash};
use dashcore_rpc::dashcore_rpc_json::{ExtendedQuorumDetails, MasternodeListItem};
use dashcore_rpc::json::QuorumType;
use dpp::block::epoch::{Epoch, EPOCH_0};
use dpp::block::extended_block_info::ExtendedBlockInfo;

use dpp::bincode::{Decode, Encode};
use dpp::dashcore::hashes::Hash;

use dpp::platform_value::Bytes32;

use drive::dpp::util::deserializer::ProtocolVersion;
use indexmap::IndexMap;

use crate::platform_types::masternode::Masternode;
use crate::platform_types::validator_set::ValidatorSet;
use dpp::block::extended_block_info::v0::ExtendedBlockInfoV0Getters;
use dpp::version::{PlatformVersion, TryIntoPlatformVersioned};
use std::collections::{BTreeMap, HashMap};

/// Platform state
#[derive(Clone, Debug)]
pub struct PlatformStateV0 {
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
pub(super) struct PlatformStateForSavingV0 {
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
    pub full_masternode_list: BTreeMap<Bytes32, Masternode>,

    /// current HPMN masternode list
    pub hpmn_masternode_list: BTreeMap<Bytes32, Masternode>,

    /// if we initialized the chain this block
    pub initialization_information: Option<PlatformInitializationState>,
}

impl TryFrom<PlatformStateV0> for PlatformStateForSavingV0 {
    type Error = Error;

    fn try_from(value: PlatformStateV0) -> Result<Self, Self::Error> {
        let platform_version = PlatformVersion::get(value.current_protocol_version_in_consensus)?;
        Ok(PlatformStateForSavingV0 {
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
                .map(|(k, v)| {
                    Ok((
                        k.into_inner().into(),
                        v.try_into_platform_versioned(platform_version)?,
                    ))
                })
                .collect::<Result<BTreeMap<Bytes32, Masternode>, Error>>()?,
            hpmn_masternode_list: value
                .hpmn_masternode_list
                .into_iter()
                .map(|(k, v)| {
                    Ok((
                        k.into_inner().into(),
                        v.try_into_platform_versioned(platform_version)?,
                    ))
                })
                .collect::<Result<BTreeMap<Bytes32, Masternode>, Error>>()?,
            initialization_information: value.initialization_information,
        })
    }
}

impl From<PlatformStateForSavingV0> for PlatformStateV0 {
    fn from(value: PlatformStateForSavingV0) -> Self {
        PlatformStateV0 {
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
        }
    }
}

/// Platform state for the first block
#[derive(Clone, Debug, Encode, Decode)]
pub struct PlatformInitializationState {
    /// Core initialization height
    pub core_initialization_height: u32,
}

impl PlatformStateV0 {
    /// The default state at init chain
    pub(super) fn default_with_protocol_versions(
        current_protocol_version_in_consensus: ProtocolVersion,
        next_epoch_protocol_version: ProtocolVersion,
    ) -> PlatformStateV0 {
        PlatformStateV0 {
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
}

/// Platform state methods introduced in version 0 of Platform State Struct
pub trait PlatformStateV0Methods {
    /// The height of the platform, only committed blocks increase height
    fn height(&self) -> u64;
    /// The height of the platform, only committed blocks increase height
    fn known_height_or(&self, default: u64) -> u64;
    /// The height of the core blockchain that Platform knows about through chain locks
    fn core_height(&self) -> u32;
    /// The height of the core blockchain that Platform knows about through chain locks
    fn known_core_height_or(&self, default: u32) -> u32;
    /// The last block time in milliseconds
    fn last_block_time_ms(&self) -> Option<u64>;
    /// The last quorum hash
    fn last_quorum_hash(&self) -> [u8; 32];
    /// The last block signature
    fn last_block_signature(&self) -> [u8; 96];
    /// The last block app hash
    fn last_block_app_hash(&self) -> Option<[u8; 32]>;
    /// The last block height or 0 for genesis
    fn last_block_height(&self) -> u64;
    /// The last block round
    fn last_block_round(&self) -> u32;
    /// The current epoch
    fn epoch(&self) -> Epoch;
    /// HPMN list len
    fn hpmn_list_len(&self) -> usize;
    /// Get the current quorum
    fn current_validator_set(&self) -> Result<&ValidatorSet, Error>;
    /// Returns information about the last committed block.
    fn last_committed_block_info(&self) -> &Option<ExtendedBlockInfo>;
    /// Returns the current protocol version that is in consensus.
    fn current_protocol_version_in_consensus(&self) -> ProtocolVersion;

    /// Returns the upcoming protocol version for the next epoch.
    fn next_epoch_protocol_version(&self) -> ProtocolVersion;

    /// Returns extended information about the current quorums.
    fn quorums_extended_info(&self) -> &HashMap<QuorumType, QuorumListExtendedInfo>;

    /// Returns the quorum hash of the current validator set.
    fn current_validator_set_quorum_hash(&self) -> QuorumHash;

    /// Returns the quorum hash of the next validator set, if it exists.
    fn next_validator_set_quorum_hash(&self) -> &Option<QuorumHash>;

    /// Returns the quorum hash of the next validator set, if it exists and replaces current value with none.
    fn take_next_validator_set_quorum_hash(&mut self) -> Option<QuorumHash>;

    /// Returns the current validator sets.
    fn validator_sets(&self) -> &IndexMap<QuorumHash, ValidatorSet>;

    /// Returns the full list of masternodes.
    fn full_masternode_list(&self) -> &BTreeMap<ProTxHash, MasternodeListItem>;

    /// Returns the list of high performance masternodes.
    fn hpmn_masternode_list(&self) -> &BTreeMap<ProTxHash, MasternodeListItem>;

    /// Returns information about the platform initialization state, if it exists.
    fn initialization_information(&self) -> &Option<PlatformInitializationState>;

    /// Sets the last committed block info.
    fn set_last_committed_block_info(&mut self, info: Option<ExtendedBlockInfo>);

    /// Sets the current protocol version in consensus.
    fn set_current_protocol_version_in_consensus(&mut self, version: ProtocolVersion);

    /// Sets the next epoch protocol version.
    fn set_next_epoch_protocol_version(&mut self, version: ProtocolVersion);

    /// Sets the extended info for the current quorums.
    fn set_quorums_extended_info(&mut self, info: HashMap<QuorumType, QuorumListExtendedInfo>);

    /// Sets the current validator set quorum hash.
    fn set_current_validator_set_quorum_hash(&mut self, hash: QuorumHash);

    /// Sets the next validator set quorum hash.
    fn set_next_validator_set_quorum_hash(&mut self, hash: Option<QuorumHash>);

    /// Sets the current validator sets.
    fn set_validator_sets(&mut self, sets: IndexMap<QuorumHash, ValidatorSet>);

    /// Sets the full masternode list.
    fn set_full_masternode_list(&mut self, list: BTreeMap<ProTxHash, MasternodeListItem>);

    /// Sets the list of high performance masternodes.
    fn set_hpmn_masternode_list(&mut self, list: BTreeMap<ProTxHash, MasternodeListItem>);
    /// Sets the platform initialization information.
    fn set_initialization_information(&mut self, info: Option<PlatformInitializationState>);

    /// Returns a mutable reference to the last committed block info.
    fn last_committed_block_info_mut(&mut self) -> &mut Option<ExtendedBlockInfo>;

    /// Returns a mutable reference to the current protocol version in consensus.
    fn current_protocol_version_in_consensus_mut(&mut self) -> &mut ProtocolVersion;

    /// Returns a mutable reference to the next epoch protocol version.
    fn next_epoch_protocol_version_mut(&mut self) -> &mut ProtocolVersion;

    /// Returns a mutable reference to the extended info for the current quorums.
    fn quorums_extended_info_mut(&mut self) -> &mut HashMap<QuorumType, QuorumListExtendedInfo>;

    /// Returns a mutable reference to the current validator set quorum hash.
    fn current_validator_set_quorum_hash_mut(&mut self) -> &mut QuorumHash;

    /// Returns a mutable reference to the next validator set quorum hash.
    fn next_validator_set_quorum_hash_mut(&mut self) -> &mut Option<QuorumHash>;

    /// Returns a mutable reference to the current validator sets.
    fn validator_sets_mut(&mut self) -> &mut IndexMap<QuorumHash, ValidatorSet>;

    /// Returns a mutable reference to the full masternode list.
    fn full_masternode_list_mut(&mut self) -> &mut BTreeMap<ProTxHash, MasternodeListItem>;

    /// Returns a mutable reference to the list of high performance masternodes.
    fn hpmn_masternode_list_mut(&mut self) -> &mut BTreeMap<ProTxHash, MasternodeListItem>;

    /// Returns a mutable reference to the platform initialization information.
    fn initialization_information_mut(&mut self) -> &mut Option<PlatformInitializationState>;
    /// The epoch ref
    fn epoch_ref(&self) -> &Epoch;
    /// The last block id hash
    fn last_block_id_hash(&self) -> [u8; 32];
}

impl PlatformStateV0Methods for PlatformStateV0 {
    /// The height of the platform, only committed blocks increase height
    fn height(&self) -> u64 {
        self.last_committed_block_info
            .as_ref()
            .map(|block_info| block_info.basic_info().height)
            .unwrap_or_default()
    }

    /// The height of the platform, only committed blocks increase height
    fn known_height_or(&self, default: u64) -> u64 {
        self.last_committed_block_info
            .as_ref()
            .map(|block_info| block_info.basic_info().height)
            .unwrap_or(default)
    }

    /// The height of the core blockchain that Platform knows about through chain locks
    fn core_height(&self) -> u32 {
        self.last_committed_block_info
            .as_ref()
            .map(|block_info| block_info.basic_info().core_height)
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
    fn known_core_height_or(&self, default: u32) -> u32 {
        self.last_committed_block_info
            .as_ref()
            .map(|block_info| block_info.basic_info().core_height)
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
    fn last_block_time_ms(&self) -> Option<u64> {
        self.last_committed_block_info
            .as_ref()
            .map(|block_info| block_info.basic_info().time_ms)
    }

    /// The last quorum hash
    fn last_quorum_hash(&self) -> [u8; 32] {
        self.last_committed_block_info
            .as_ref()
            .map(|block_info| *block_info.quorum_hash())
            .unwrap_or_default()
    }

    /// The last block id hash
    fn last_block_id_hash(&self) -> [u8; 32] {
        self.last_committed_block_info
            .as_ref()
            .map(|block_info| *block_info.block_id_hash())
            .unwrap_or_default()
    }

    /// The last block signature
    fn last_block_signature(&self) -> [u8; 96] {
        self.last_committed_block_info
            .as_ref()
            .map(|block_info| *block_info.signature())
            .unwrap_or([0u8; 96])
    }

    /// The last block app hash
    fn last_block_app_hash(&self) -> Option<[u8; 32]> {
        self.last_committed_block_info
            .as_ref()
            .map(|block_info| *block_info.app_hash())
    }

    /// The last block height or 0 for genesis
    fn last_block_height(&self) -> u64 {
        self.last_committed_block_info
            .as_ref()
            .map(|block_info| block_info.basic_info().height)
            .unwrap_or_default()
    }

    /// The last block round
    fn last_block_round(&self) -> u32 {
        self.last_committed_block_info
            .as_ref()
            .map(|block_info| block_info.round())
            .unwrap_or_default()
    }

    /// The current epoch
    fn epoch(&self) -> Epoch {
        self.last_committed_block_info
            .as_ref()
            .map(|block_info| block_info.basic_info().epoch)
            .unwrap_or_default()
    }

    fn epoch_ref(&self) -> &Epoch {
        self.last_committed_block_info
            .as_ref()
            .map(|block_info| &block_info.basic_info().epoch)
            .unwrap_or(&EPOCH_0)
    }

    /// HPMN list len
    fn hpmn_list_len(&self) -> usize {
        self.hpmn_masternode_list.len()
    }

    /// Get the current quorum
    fn current_validator_set(&self) -> Result<&ValidatorSet, Error> {
        self.validator_sets
            .get(&self.current_validator_set_quorum_hash)
            .ok_or(Error::Execution(ExecutionError::CorruptedCachedState(
                "current validator quorum hash not in current known validator sets",
            )))
    }

    /// Get the current protocol version in consensus
    fn current_protocol_version_in_consensus(&self) -> ProtocolVersion {
        self.current_protocol_version_in_consensus
    }

    /// Returns information about the last committed block.
    fn last_committed_block_info(&self) -> &Option<ExtendedBlockInfo> {
        &self.last_committed_block_info
    }

    /// Returns the upcoming protocol version for the next epoch.
    fn next_epoch_protocol_version(&self) -> ProtocolVersion {
        self.next_epoch_protocol_version
    }

    /// Returns extended information about the current quorums.
    fn quorums_extended_info(&self) -> &HashMap<QuorumType, QuorumListExtendedInfo> {
        &self.quorums_extended_info
    }

    /// Returns the quorum hash of the next validator set, if it exists.
    fn next_validator_set_quorum_hash(&self) -> &Option<QuorumHash> {
        &self.next_validator_set_quorum_hash
    }

    /// Returns the quorum hash of the next validator set, if it exists, and replaces current value with None.
    fn take_next_validator_set_quorum_hash(&mut self) -> Option<QuorumHash> {
        self.next_validator_set_quorum_hash.take()
    }

    /// Returns the current validator sets.
    fn validator_sets(&self) -> &IndexMap<QuorumHash, ValidatorSet> {
        &self.validator_sets
    }

    /// Returns the full list of masternodes.
    fn full_masternode_list(&self) -> &BTreeMap<ProTxHash, MasternodeListItem> {
        &self.full_masternode_list
    }

    /// Returns information about the platform initialization state, if it exists.
    fn initialization_information(&self) -> &Option<PlatformInitializationState> {
        &self.initialization_information
    }

    /// Returns the quorum hash of the current validator set.
    fn current_validator_set_quorum_hash(&self) -> QuorumHash {
        self.current_validator_set_quorum_hash
    }

    /// Returns the list of high performance masternodes.
    fn hpmn_masternode_list(&self) -> &BTreeMap<ProTxHash, MasternodeListItem> {
        &self.hpmn_masternode_list
    }

    /// Sets the last committed block info.
    fn set_last_committed_block_info(&mut self, info: Option<ExtendedBlockInfo>) {
        self.last_committed_block_info = info;
    }

    /// Sets the current protocol version in consensus.
    fn set_current_protocol_version_in_consensus(&mut self, version: ProtocolVersion) {
        self.current_protocol_version_in_consensus = version;
    }

    /// Sets the next epoch protocol version.
    fn set_next_epoch_protocol_version(&mut self, version: ProtocolVersion) {
        self.next_epoch_protocol_version = version;
    }

    /// Sets the extended info for the current quorums.
    fn set_quorums_extended_info(&mut self, info: HashMap<QuorumType, QuorumListExtendedInfo>) {
        self.quorums_extended_info = info;
    }

    /// Sets the current validator set quorum hash.
    fn set_current_validator_set_quorum_hash(&mut self, hash: QuorumHash) {
        self.current_validator_set_quorum_hash = hash;
    }

    /// Sets the next validator set quorum hash.
    fn set_next_validator_set_quorum_hash(&mut self, hash: Option<QuorumHash>) {
        self.next_validator_set_quorum_hash = hash;
    }

    /// Sets the current validator sets.
    fn set_validator_sets(&mut self, sets: IndexMap<QuorumHash, ValidatorSet>) {
        self.validator_sets = sets;
    }

    /// Sets the full masternode list.
    fn set_full_masternode_list(&mut self, list: BTreeMap<ProTxHash, MasternodeListItem>) {
        self.full_masternode_list = list;
    }

    /// Sets the list of high performance masternodes.
    fn set_hpmn_masternode_list(&mut self, list: BTreeMap<ProTxHash, MasternodeListItem>) {
        self.hpmn_masternode_list = list;
    }

    /// Sets the platform initialization information.
    fn set_initialization_information(&mut self, info: Option<PlatformInitializationState>) {
        self.initialization_information = info;
    }

    fn last_committed_block_info_mut(&mut self) -> &mut Option<ExtendedBlockInfo> {
        &mut self.last_committed_block_info
    }

    fn current_protocol_version_in_consensus_mut(&mut self) -> &mut ProtocolVersion {
        &mut self.current_protocol_version_in_consensus
    }

    fn next_epoch_protocol_version_mut(&mut self) -> &mut ProtocolVersion {
        &mut self.next_epoch_protocol_version
    }

    fn quorums_extended_info_mut(&mut self) -> &mut HashMap<QuorumType, QuorumListExtendedInfo> {
        &mut self.quorums_extended_info
    }

    fn current_validator_set_quorum_hash_mut(&mut self) -> &mut QuorumHash {
        &mut self.current_validator_set_quorum_hash
    }

    fn next_validator_set_quorum_hash_mut(&mut self) -> &mut Option<QuorumHash> {
        &mut self.next_validator_set_quorum_hash
    }

    fn validator_sets_mut(&mut self) -> &mut IndexMap<QuorumHash, ValidatorSet> {
        &mut self.validator_sets
    }

    fn full_masternode_list_mut(&mut self) -> &mut BTreeMap<ProTxHash, MasternodeListItem> {
        &mut self.full_masternode_list
    }

    fn hpmn_masternode_list_mut(&mut self) -> &mut BTreeMap<ProTxHash, MasternodeListItem> {
        &mut self.hpmn_masternode_list
    }

    fn initialization_information_mut(&mut self) -> &mut Option<PlatformInitializationState> {
        &mut self.initialization_information
    }
}
