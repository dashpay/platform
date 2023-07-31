/// Version 0
pub mod v0;

use crate::error::Error;
use crate::platform_types::platform_state::v0::{
    PlatformInitializationState, PlatformStateForSavingV0, PlatformStateV0, PlatformStateV0Methods,
};

use crate::platform_types::validator_set::ValidatorSet;
use crate::rpc::core::QuorumListExtendedInfo;
use dashcore_rpc::dashcore_rpc_json::{MasternodeListItem, QuorumType};
use derive_more::From;
use dpp::bincode::{config, Decode, Encode};
use dpp::block::block_info::ExtendedBlockInfo;
use dpp::block::epoch::Epoch;
use dpp::block::extended_block_info::ExtendedBlockInfo;
use dpp::dashcore::{ProTxHash, QuorumHash};
use dpp::serialization::{PlatformDeserializable, PlatformSerializable};
use dpp::util::deserializer::ProtocolVersion;
use dpp::version::drive_versions::DriveVersion;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use indexmap::IndexMap;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use std::collections::{BTreeMap, HashMap};

/// Platform state
#[derive(Clone, Debug, PlatformSerialize, PlatformDeserialize, From)]
#[platform_serialize(into = "PlatformStateForSaving")]
pub enum PlatformState {
    /// Version 0
    V0(PlatformStateV0),
}

/// Platform state
#[derive(Clone, Debug, Encode, Decode, From)]
enum PlatformStateForSaving {
    /// Version 0
    V0(PlatformStateForSavingV0),
}

impl PlatformState {
    /// Get the current platform version
    pub fn current_platform_version(&self) -> Result<&PlatformVersion, Error> {
        PlatformVersion::get(self.current_protocol_version_in_consensus()).map_err(Error::Protocol)
    }

    /// The default state at platform start
    pub fn default_with_protocol_versions(
        current_protocol_version_in_consensus: ProtocolVersion,
        next_epoch_protocol_version: ProtocolVersion,
    ) -> PlatformState {
        //todo find the current Platform state for the protocol version
        PlatformStateV0::default_with_protocol_versions(
            current_protocol_version_in_consensus,
            next_epoch_protocol_version,
        )
        .into()
    }

    /// Retrieve version 0, or an error if not currently on version 0
    pub fn v0(&self) -> Result<&PlatformStateV0, Error> {
        match self {
            PlatformState::V0(v) => Ok(v),
            //_ => Err(Error::Execution(ExecutionError::CorruptedCodeVersionMismatch("platform state version mismatch"))),
        }
    }

    /// Retrieve version 0 as mutable, or an error if not currently on version 0
    pub fn v0_mut(&mut self) -> Result<&mut PlatformStateV0, Error> {
        match self {
            PlatformState::V0(v) => Ok(v),
            //_ => Err(Error::Execution(ExecutionError::CorruptedCodeVersionMismatch("platform state version mismatch"))),
        }
    }
}

impl From<PlatformState> for PlatformStateForSaving {
    fn from(value: PlatformState) -> Self {
        match value {
            PlatformState::V0(v0_value) => PlatformStateForSaving::V0(v0_value.into()),
        }
    }
}

impl TryFrom<PlatformStateForSaving> for PlatformState {
    type Error = ProtocolError;

    fn try_from(value: PlatformStateForSaving) -> Result<Self, Self::Error> {
        match value {
            PlatformStateForSaving::V0(v0_value) => Ok(PlatformState::V0(v0_value.try_into()?)),
        }
    }
}

impl PlatformStateV0Methods for PlatformState {
    fn height(&self) -> u64 {
        match self {
            PlatformState::V0(v0) => v0.height(),
        }
    }

    fn known_height_or(&self, default: u64) -> u64 {
        match self {
            PlatformState::V0(v0) => v0.known_height_or(default),
        }
    }

    fn core_height(&self) -> u32 {
        match self {
            PlatformState::V0(v0) => v0.core_height(),
        }
    }

    fn known_core_height_or(&self, default: u32) -> u32 {
        match self {
            PlatformState::V0(v0) => v0.known_core_height_or(default),
        }
    }

    fn last_block_time_ms(&self) -> Option<u64> {
        match self {
            PlatformState::V0(v0) => v0.last_block_time_ms(),
        }
    }

    fn last_quorum_hash(&self) -> [u8; 32] {
        match self {
            PlatformState::V0(v0) => v0.last_quorum_hash(),
        }
    }

    fn last_block_signature(&self) -> [u8; 96] {
        match self {
            PlatformState::V0(v0) => v0.last_block_signature(),
        }
    }

    fn last_block_app_hash(&self) -> Option<[u8; 32]> {
        match self {
            PlatformState::V0(v0) => v0.last_block_app_hash(),
        }
    }

    fn last_block_height(&self) -> u64 {
        match self {
            PlatformState::V0(v0) => v0.last_block_height(),
        }
    }

    fn last_block_round(&self) -> u32 {
        match self {
            PlatformState::V0(v0) => v0.last_block_round(),
        }
    }

    fn epoch(&self) -> Epoch {
        match self {
            PlatformState::V0(v0) => v0.epoch(),
        }
    }

    fn hpmn_list_len(&self) -> usize {
        match self {
            PlatformState::V0(v0) => v0.hpmn_list_len(),
        }
    }

    fn current_validator_set(&self) -> Result<&ValidatorSet, Error> {
        match self {
            PlatformState::V0(v0) => v0.current_validator_set(),
        }
    }

    fn current_protocol_version_in_consensus(&self) -> ProtocolVersion {
        match self {
            PlatformState::V0(v0) => v0.current_protocol_version_in_consensus(),
        }
    }

    fn last_committed_block_info(&self) -> &Option<ExtendedBlockInfo> {
        match self {
            PlatformState::V0(v0) => &v0.last_committed_block_info,
        }
    }

    fn next_epoch_protocol_version(&self) -> ProtocolVersion {
        match self {
            PlatformState::V0(v0) => v0.next_epoch_protocol_version,
        }
    }

    fn quorums_extended_info(&self) -> &HashMap<QuorumType, QuorumListExtendedInfo> {
        match self {
            PlatformState::V0(v0) => &v0.quorums_extended_info,
        }
    }

    fn current_validator_set_quorum_hash(&self) -> QuorumHash {
        match self {
            PlatformState::V0(v0) => v0.current_validator_set_quorum_hash,
        }
    }

    fn next_validator_set_quorum_hash(&self) -> &Option<QuorumHash> {
        match self {
            PlatformState::V0(v0) => &v0.next_validator_set_quorum_hash,
        }
    }

    fn validator_sets(&self) -> &IndexMap<QuorumHash, ValidatorSet> {
        match self {
            PlatformState::V0(v0) => &v0.validator_sets,
        }
    }

    fn full_masternode_list(&self) -> &BTreeMap<ProTxHash, MasternodeListItem> {
        match self {
            PlatformState::V0(v0) => &v0.full_masternode_list,
        }
    }

    fn hpmn_masternode_list(&self) -> &BTreeMap<ProTxHash, MasternodeListItem> {
        match self {
            PlatformState::V0(v0) => &v0.hpmn_masternode_list,
        }
    }

    fn initialization_information(&self) -> &Option<PlatformInitializationState> {
        match self {
            PlatformState::V0(v0) => &v0.initialization_information,
        }
    }

    fn set_last_committed_block_info(&mut self, info: Option<ExtendedBlockInfo>) {
        match self {
            PlatformState::V0(v0) => v0.set_last_committed_block_info(info),
        }
    }

    fn set_current_protocol_version_in_consensus(&mut self, version: ProtocolVersion) {
        match self {
            PlatformState::V0(v0) => v0.set_current_protocol_version_in_consensus(version),
        }
    }

    fn set_next_epoch_protocol_version(&mut self, version: ProtocolVersion) {
        match self {
            PlatformState::V0(v0) => v0.set_next_epoch_protocol_version(version),
        }
    }

    fn set_quorums_extended_info(&mut self, info: HashMap<QuorumType, QuorumListExtendedInfo>) {
        match self {
            PlatformState::V0(v0) => v0.set_quorums_extended_info(info),
        }
    }

    fn set_current_validator_set_quorum_hash(&mut self, hash: QuorumHash) {
        match self {
            PlatformState::V0(v0) => v0.set_current_validator_set_quorum_hash(hash),
        }
    }

    fn set_next_validator_set_quorum_hash(&mut self, hash: Option<QuorumHash>) {
        match self {
            PlatformState::V0(v0) => v0.set_next_validator_set_quorum_hash(hash),
        }
    }

    fn set_validator_sets(&mut self, sets: IndexMap<QuorumHash, ValidatorSet>) {
        match self {
            PlatformState::V0(v0) => v0.set_validator_sets(sets),
        }
    }

    fn set_full_masternode_list(&mut self, list: BTreeMap<ProTxHash, MasternodeListItem>) {
        match self {
            PlatformState::V0(v0) => v0.set_full_masternode_list(list),
        }
    }

    fn set_hpmn_masternode_list(&mut self, list: BTreeMap<ProTxHash, MasternodeListItem>) {
        match self {
            PlatformState::V0(v0) => v0.set_hpmn_masternode_list(list),
        }
    }

    fn set_initialization_information(&mut self, info: Option<PlatformInitializationState>) {
        match self {
            PlatformState::V0(v0) => v0.set_initialization_information(info),
        }
    }

    fn last_committed_block_info_mut(&mut self) -> &mut Option<ExtendedBlockInfo> {
        match self {
            PlatformState::V0(v0) => v0.last_committed_block_info_mut(),
        }
    }

    fn current_protocol_version_in_consensus_mut(&mut self) -> &mut ProtocolVersion {
        match self {
            PlatformState::V0(v0) => v0.current_protocol_version_in_consensus_mut(),
        }
    }

    fn next_epoch_protocol_version_mut(&mut self) -> &mut ProtocolVersion {
        match self {
            PlatformState::V0(v0) => v0.next_epoch_protocol_version_mut(),
        }
    }

    fn quorums_extended_info_mut(&mut self) -> &mut HashMap<QuorumType, QuorumListExtendedInfo> {
        match self {
            PlatformState::V0(v0) => v0.quorums_extended_info_mut(),
        }
    }

    fn current_validator_set_quorum_hash_mut(&mut self) -> &mut QuorumHash {
        match self {
            PlatformState::V0(v0) => v0.current_validator_set_quorum_hash_mut(),
        }
    }

    fn next_validator_set_quorum_hash_mut(&mut self) -> &mut Option<QuorumHash> {
        match self {
            PlatformState::V0(v0) => v0.next_validator_set_quorum_hash_mut(),
        }
    }

    fn validator_sets_mut(&mut self) -> &mut IndexMap<QuorumHash, ValidatorSet> {
        match self {
            PlatformState::V0(v0) => v0.validator_sets_mut(),
        }
    }

    fn full_masternode_list_mut(&mut self) -> &mut BTreeMap<ProTxHash, MasternodeListItem> {
        match self {
            PlatformState::V0(v0) => v0.full_masternode_list_mut(),
        }
    }

    fn hpmn_masternode_list_mut(&mut self) -> &mut BTreeMap<ProTxHash, MasternodeListItem> {
        match self {
            PlatformState::V0(v0) => v0.hpmn_masternode_list_mut(),
        }
    }

    fn initialization_information_mut(&mut self) -> &mut Option<PlatformInitializationState> {
        match self {
            PlatformState::V0(v0) => v0.initialization_information_mut(),
        }
    }

    fn take_next_validator_set_quorum_hash(&mut self) -> Option<QuorumHash> {
        match self {
            PlatformState::V0(v0) => v0.take_next_validator_set_quorum_hash(),
        }
    }
}
