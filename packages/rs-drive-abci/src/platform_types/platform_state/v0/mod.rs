use crate::error::execution::ExecutionError;
use crate::error::Error;
use dashcore_rpc::dashcore::{ProTxHash, QuorumHash};
use dashcore_rpc::dashcore_rpc_json::MasternodeListItem;
use dpp::block::epoch::{Epoch, EPOCH_0};
use dpp::block::extended_block_info::ExtendedBlockInfo;

use dpp::bincode::{Decode, Encode};
use dpp::dashcore::hashes::Hash;

use dpp::platform_value::Bytes32;

use drive::dpp::util::deserializer::ProtocolVersion;
use indexmap::IndexMap;

use crate::platform_types::masternode::Masternode;
use crate::platform_types::validator_set::ValidatorSet;
use dpp::block::block_info::{BlockInfo, DEFAULT_BLOCK_INFO};
use dpp::block::extended_block_info::v0::ExtendedBlockInfoV0Getters;
use dpp::version::{PlatformVersion, TryIntoPlatformVersioned};

use crate::config::PlatformConfig;
use crate::platform_types::signature_verification_quorum_set::{
    SignatureVerificationQuorumSet, SignatureVerificationQuorumSetForSaving,
};
use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
use itertools::Itertools;
use std::collections::BTreeMap;
use std::fmt::{Debug, Formatter};

/// Platform state
#[derive(Clone)]
pub struct PlatformStateV0 {
    /// Information about the genesis block
    pub genesis_block_info: Option<BlockInfo>, // TODO: we already have it in epoch 0
    /// Information about the last block
    pub last_committed_block_info: Option<ExtendedBlockInfo>,
    /// Current Version
    pub current_protocol_version_in_consensus: ProtocolVersion,
    /// upcoming protocol version
    pub next_epoch_protocol_version: ProtocolVersion,
    /// current quorum
    pub current_validator_set_quorum_hash: QuorumHash,
    /// next quorum
    pub next_validator_set_quorum_hash: Option<QuorumHash>,
    /// This is a modified current platform version based on
    /// `current_protocol_version_in_consensus` with some function versions
    /// changed to fix an urgent bug that is not a part of normal upgrade process
    pub patched_platform_version: Option<&'static PlatformVersion>,
    /// current validator set quorums
    /// The validator set quorums are a subset of the quorums, but they also contain the list of
    /// all members
    pub validator_sets: IndexMap<QuorumHash, ValidatorSet>,

    /// Quorums used for validating chain locks (400 60 for mainnet)
    pub chain_lock_validating_quorums: SignatureVerificationQuorumSet,

    /// Quorums used for validating instant locks
    pub instant_lock_validating_quorums: SignatureVerificationQuorumSet,

    /// current full masternode list
    pub full_masternode_list: BTreeMap<ProTxHash, MasternodeListItem>,

    /// current HPMN masternode list
    pub hpmn_masternode_list: BTreeMap<ProTxHash, MasternodeListItem>,

    /// previous Fee Versions
    pub previous_fee_versions: CachedEpochIndexFeeVersions,
}

impl Debug for PlatformStateV0 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlatformStateV0")
            .field("genesis_block_info", &self.genesis_block_info)
            .field("last_committed_block_info", &self.last_committed_block_info)
            .field(
                "current_protocol_version_in_consensus",
                &self.current_protocol_version_in_consensus,
            )
            .field(
                "next_epoch_protocol_version",
                &self.next_epoch_protocol_version,
            )
            .field(
                "current_validator_set_quorum_hash",
                &self.current_validator_set_quorum_hash.to_string(),
            )
            .field(
                "next_validator_set_quorum_hash",
                &self
                    .next_validator_set_quorum_hash
                    .as_ref()
                    .map_or(String::from("None"), |h| format!("Some({})", h)),
            )
            .field(
                "validator_sets",
                &hex_encoded_validator_sets(&self.validator_sets),
            )
            .field("full_masternode_list", &self.full_masternode_list)
            .field("hpmn_masternode_list", &self.hpmn_masternode_list)
            .field("initialization_information", &self.genesis_block_info)
            .field(
                "chain_lock_validating_quorums",
                &self.chain_lock_validating_quorums,
            )
            .field(
                "instant_lock_validating_quorums",
                &self.instant_lock_validating_quorums,
            )
            .finish()
    }
}

fn hex_encoded_validator_sets(validator_sets: &IndexMap<QuorumHash, ValidatorSet>) -> String {
    let entries = validator_sets
        .iter()
        .map(|(k, v)| format!("{:?}: {:?}", k.to_string(), v))
        .collect::<Vec<_>>();
    format!("{:?}", entries)
}

/// Platform state
#[derive(Clone, Debug, Encode, Decode)]
pub struct PlatformStateForSavingV0 {
    /// Information about the genesis block
    pub genesis_block_info: Option<BlockInfo>,
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
    /// current validator set quorums
    /// The validator set quorums are a subset of the quorums, but they also contain the list of
    /// all members
    #[bincode(with_serde)]
    pub validator_sets: Vec<(Bytes32, ValidatorSet)>,

    /// The quorums used for validating chain locks
    pub chain_lock_validating_quorums: SignatureVerificationQuorumSetForSaving,

    /// The quorums used for validating instant locks
    pub instant_lock_validating_quorums: SignatureVerificationQuorumSetForSaving,

    /// current full masternode list
    pub full_masternode_list: BTreeMap<Bytes32, Masternode>,

    /// current HPMN masternode list
    pub hpmn_masternode_list: BTreeMap<Bytes32, Masternode>,

    /// previous FeeVersions
    pub previous_fee_versions: CachedEpochIndexFeeVersions,
}

impl TryFrom<PlatformStateV0> for PlatformStateForSavingV0 {
    type Error = Error;

    fn try_from(value: PlatformStateV0) -> Result<Self, Self::Error> {
        let platform_version = value.current_platform_version()?;
        Ok(PlatformStateForSavingV0 {
            genesis_block_info: value.genesis_block_info,
            last_committed_block_info: value.last_committed_block_info,
            current_protocol_version_in_consensus: value.current_protocol_version_in_consensus,
            next_epoch_protocol_version: value.next_epoch_protocol_version,
            current_validator_set_quorum_hash: value
                .current_validator_set_quorum_hash
                .to_byte_array()
                .into(),
            next_validator_set_quorum_hash: value
                .next_validator_set_quorum_hash
                .map(|quorum_hash| quorum_hash.to_byte_array().into()),
            validator_sets: value
                .validator_sets
                .into_iter()
                .map(|(k, v)| (k.to_byte_array().into(), v))
                .collect(),
            chain_lock_validating_quorums: value.chain_lock_validating_quorums.into(),
            instant_lock_validating_quorums: value.instant_lock_validating_quorums.into(),
            full_masternode_list: value
                .full_masternode_list
                .into_iter()
                .map(|(k, v)| {
                    Ok((
                        k.to_byte_array().into(),
                        v.try_into_platform_versioned(platform_version)?,
                    ))
                })
                .collect::<Result<BTreeMap<Bytes32, Masternode>, Error>>()?,
            hpmn_masternode_list: value
                .hpmn_masternode_list
                .into_iter()
                .map(|(k, v)| {
                    Ok((
                        k.to_byte_array().into(),
                        v.try_into_platform_versioned(platform_version)?,
                    ))
                })
                .collect::<Result<BTreeMap<Bytes32, Masternode>, Error>>()?,
            previous_fee_versions: value.previous_fee_versions,
        })
    }
}

impl From<PlatformStateForSavingV0> for PlatformStateV0 {
    fn from(value: PlatformStateForSavingV0) -> Self {
        PlatformStateV0 {
            genesis_block_info: value.genesis_block_info,
            last_committed_block_info: value.last_committed_block_info,
            current_protocol_version_in_consensus: value.current_protocol_version_in_consensus,
            next_epoch_protocol_version: value.next_epoch_protocol_version,
            current_validator_set_quorum_hash: QuorumHash::from_byte_array(
                value.current_validator_set_quorum_hash.to_buffer(),
            ),
            next_validator_set_quorum_hash: value
                .next_validator_set_quorum_hash
                .map(|bytes| QuorumHash::from_byte_array(bytes.to_buffer())),
            patched_platform_version: None,
            validator_sets: value
                .validator_sets
                .into_iter()
                .map(|(k, v)| (QuorumHash::from_byte_array(k.to_buffer()), v))
                .collect(),
            chain_lock_validating_quorums: value.chain_lock_validating_quorums.into(),
            instant_lock_validating_quorums: value.instant_lock_validating_quorums.into(),
            full_masternode_list: value
                .full_masternode_list
                .into_iter()
                .map(|(k, v)| (ProTxHash::from_byte_array(k.to_buffer()), v.into()))
                .collect(),
            hpmn_masternode_list: value
                .hpmn_masternode_list
                .into_iter()
                .map(|(k, v)| (ProTxHash::from_byte_array(k.to_buffer()), v.into()))
                .collect(),
            previous_fee_versions: value.previous_fee_versions,
        }
    }
}

impl PlatformStateV0 {
    /// The default state at init chain
    pub(super) fn default_with_protocol_versions(
        current_protocol_version_in_consensus: ProtocolVersion,
        next_epoch_protocol_version: ProtocolVersion,
        config: &PlatformConfig,
    ) -> Result<PlatformStateV0, Error> {
        let platform_version = PlatformVersion::get(current_protocol_version_in_consensus)?;

        let state = PlatformStateV0 {
            last_committed_block_info: None,
            current_protocol_version_in_consensus,
            next_epoch_protocol_version,
            current_validator_set_quorum_hash: QuorumHash::all_zeros(),
            next_validator_set_quorum_hash: None,
            patched_platform_version: None,
            validator_sets: Default::default(),
            chain_lock_validating_quorums: SignatureVerificationQuorumSet::new(
                &config.chain_lock,
                platform_version,
            )?,
            instant_lock_validating_quorums: SignatureVerificationQuorumSet::new(
                &config.instant_lock,
                platform_version,
            )?,
            full_masternode_list: Default::default(),
            hpmn_masternode_list: Default::default(),
            genesis_block_info: None,
            previous_fee_versions: Default::default(),
        };

        Ok(state)
    }
}

/// Masternode list Changes
#[derive(Debug, Clone)]
pub struct MasternodeListChanges {
    /// The new masternodes
    pub new_masternodes: Vec<ProTxHash>,
    /// The removed masternodes
    pub removed_masternodes: Vec<ProTxHash>,
    /// The banned masternodes
    pub banned_masternodes: Vec<ProTxHash>,
    /// The unbanned masternodes
    pub unbanned_masternodes: Vec<ProTxHash>,
    /// the new masternodes that come in as banned
    pub new_banned_masternodes: Vec<ProTxHash>,
}

pub(super) trait PlatformStateV0PrivateMethods {
    /// Set patched platform version. It's using to fix urgent bugs as not a part of normal upgrade process
    /// The patched version returns from the public current_platform_version getter in case if present.
    fn set_patched_platform_version(&mut self, version: Option<&'static PlatformVersion>);
}

/// Platform state methods introduced in version 0 of Platform State Struct
pub trait PlatformStateV0Methods {
    /// The last block height or 0 for genesis
    fn last_committed_block_height(&self) -> u64;
    /// The height of the platform, only committed blocks increase height
    fn last_committed_known_block_height_or(&self, default: u64) -> u64;
    /// The height of the core blockchain that Platform knows about through chain locks
    fn last_committed_core_height(&self) -> u32;
    /// The height of the core blockchain that Platform knows about through chain locks
    fn last_committed_known_core_height_or(&self, default: u32) -> u32;
    /// The last block time in milliseconds
    fn last_committed_block_time_ms(&self) -> Option<u64>;
    /// The last quorum hash
    fn last_committed_quorum_hash(&self) -> [u8; 32];
    /// The last block proposer pro tx hash
    fn last_committed_block_proposer_pro_tx_hash(&self) -> [u8; 32];
    /// The last block signature
    fn last_committed_block_signature(&self) -> [u8; 96];
    /// The last block app hash
    fn last_committed_block_app_hash(&self) -> Option<[u8; 32]>;
    /// The last block round
    fn last_committed_block_round(&self) -> u32;
    /// The current epoch
    fn last_committed_block_epoch(&self) -> Epoch;
    /// HPMN list len
    fn hpmn_list_len(&self) -> usize;
    /// Get the current quorum
    fn current_validator_set(&self) -> Result<&ValidatorSet, Error>;
    /// Returns information about the last committed block.
    fn last_committed_block_info(&self) -> &Option<ExtendedBlockInfo>;
    /// Returns the current protocol version that is in consensus.
    fn current_protocol_version_in_consensus(&self) -> ProtocolVersion;
    /// Patched platform version. Used to fix urgent bugs as not part of normal upgrade process.
    /// The patched version returns from the public current_platform_version getter in case if present.
    fn patched_platform_version(&self) -> Option<&'static PlatformVersion>;
    /// Get the current platform version or patched if present
    fn current_platform_version(&self) -> Result<&'static PlatformVersion, Error> {
        self.patched_platform_version().map(Ok).unwrap_or_else(|| {
            PlatformVersion::get(self.current_protocol_version_in_consensus()).map_err(Error::from)
        })
    }
    /// Returns the upcoming protocol version for the next epoch.
    fn next_epoch_protocol_version(&self) -> ProtocolVersion;

    /// Returns the quorum hash of the current validator set.
    fn current_validator_set_quorum_hash(&self) -> QuorumHash;

    /// Returns the quorum hash of the next validator set, if it exists.
    fn next_validator_set_quorum_hash(&self) -> &Option<QuorumHash>;

    /// Returns the quorum hash of the next validator set, if it exists and replaces current value with none.
    fn take_next_validator_set_quorum_hash(&mut self) -> Option<QuorumHash>;

    /// Returns the current validator sets.
    fn validator_sets(&self) -> &IndexMap<QuorumHash, ValidatorSet>;

    /// Returns the quorums used to validate chain locks.
    fn chain_lock_validating_quorums(&self) -> &SignatureVerificationQuorumSet;

    /// Returns quorums used to validate instant locks.
    fn instant_lock_validating_quorums(&self) -> &SignatureVerificationQuorumSet;

    /// Returns the full list of masternodes.
    fn full_masternode_list(&self) -> &BTreeMap<ProTxHash, MasternodeListItem>;

    /// Returns the list of high performance masternodes.
    fn hpmn_masternode_list(&self) -> &BTreeMap<ProTxHash, MasternodeListItem>;

    /// Returns information about the platform initialization state, if it exists.
    fn genesis_block_info(&self) -> Option<&BlockInfo>;

    /// Returns the last committed block info if present or the genesis block info if not or default one
    fn last_block_info(&self) -> &BlockInfo;

    /// Sets the last committed block info.
    fn set_last_committed_block_info(&mut self, info: Option<ExtendedBlockInfo>);

    /// Sets the current protocol version in consensus.
    fn set_current_protocol_version_in_consensus(&mut self, version: ProtocolVersion);

    /// Sets the next epoch protocol version.
    fn set_next_epoch_protocol_version(&mut self, version: ProtocolVersion);

    /// Sets the current validator set quorum hash.
    fn set_current_validator_set_quorum_hash(&mut self, hash: QuorumHash);

    /// Sets the next validator set quorum hash.
    fn set_next_validator_set_quorum_hash(&mut self, hash: Option<QuorumHash>);

    /// Sets the current validator sets.
    fn set_validator_sets(&mut self, sets: IndexMap<QuorumHash, ValidatorSet>);

    /// Sets the current chain lock validating quorums.
    fn set_chain_lock_validating_quorums(&mut self, quorums: SignatureVerificationQuorumSet);

    /// Sets the current instant lock validating quorums.
    fn set_instant_lock_validating_quorums(&mut self, quorums: SignatureVerificationQuorumSet);

    /// Sets the full masternode list.
    fn set_full_masternode_list(&mut self, list: BTreeMap<ProTxHash, MasternodeListItem>);

    /// Sets the list of high performance masternodes.
    fn set_hpmn_masternode_list(&mut self, list: BTreeMap<ProTxHash, MasternodeListItem>);
    /// Sets the platform initialization information.
    fn set_genesis_block_info(&mut self, info: Option<BlockInfo>);

    /// Returns a mutable reference to the last committed block info.
    fn last_committed_block_info_mut(&mut self) -> &mut Option<ExtendedBlockInfo>;

    /// Returns a mutable reference to the current protocol version in consensus.
    fn current_protocol_version_in_consensus_mut(&mut self) -> &mut ProtocolVersion;

    /// Returns a mutable reference to the next epoch protocol version.
    fn next_epoch_protocol_version_mut(&mut self) -> &mut ProtocolVersion;

    /// Returns a mutable reference to the current validator set quorum hash.
    fn current_validator_set_quorum_hash_mut(&mut self) -> &mut QuorumHash;

    /// Returns a mutable reference to the next validator set quorum hash.
    fn next_validator_set_quorum_hash_mut(&mut self) -> &mut Option<QuorumHash>;

    /// Returns a mutable reference to the current validator sets.
    fn validator_sets_mut(&mut self) -> &mut IndexMap<QuorumHash, ValidatorSet>;

    /// Returns a mutable reference to the chain lock validating quorums.
    fn chain_lock_validating_quorums_mut(&mut self) -> &mut SignatureVerificationQuorumSet;

    /// Returns a mutable reference to the instant lock validating quorums.
    fn instant_lock_validating_quorums_mut(&mut self) -> &mut SignatureVerificationQuorumSet;

    /// Returns a mutable reference to the full masternode list.
    fn full_masternode_list_mut(&mut self) -> &mut BTreeMap<ProTxHash, MasternodeListItem>;

    /// Returns a mutable reference to the list of high performance masternodes.
    fn hpmn_masternode_list_mut(&mut self) -> &mut BTreeMap<ProTxHash, MasternodeListItem>;

    /// The epoch ref
    fn last_committed_block_epoch_ref(&self) -> &Epoch;
    /// The last block id hash
    fn last_committed_block_id_hash(&self) -> [u8; 32];

    /// Returns reference to the previous feeversions
    fn previous_fee_versions(&self) -> &CachedEpochIndexFeeVersions;

    /// Returns a mutable reference to the previous feeversions
    fn previous_fee_versions_mut(&mut self) -> &mut CachedEpochIndexFeeVersions;

    /// The changes in the full masternode list between two platform states
    fn full_masternode_list_changes(&self, previous: &Self) -> MasternodeListChanges
    where
        Self: Sized;

    /// The changes in the high performance masternode list (evonodes) between two platform states
    fn hpmn_masternode_list_changes(&self, previous: &Self) -> MasternodeListChanges
    where
        Self: Sized;
}

impl PlatformStateV0PrivateMethods for PlatformStateV0 {
    /// Set patched platform version. It's using to fix urgent bugs as not a part of normal upgrade process
    /// The patched version returns from the public current_platform_version getter in case if present.
    fn set_patched_platform_version(&mut self, version: Option<&'static PlatformVersion>) {
        self.patched_platform_version = version;
    }
}

impl PlatformStateV0Methods for PlatformStateV0 {
    /// The last block height or 0 for genesis
    fn last_committed_block_height(&self) -> u64 {
        self.last_committed_block_info
            .as_ref()
            .map(|block_info| block_info.basic_info().height)
            .unwrap_or_default()
    }

    /// The height of the platform, only committed blocks increase height
    fn last_committed_known_block_height_or(&self, default: u64) -> u64 {
        self.last_committed_block_info
            .as_ref()
            .map(|block_info| block_info.basic_info().height)
            .unwrap_or(default)
    }

    /// The height of the core blockchain that Platform knows about through chain locks
    fn last_committed_core_height(&self) -> u32 {
        self.last_committed_block_info
            .as_ref()
            .map(|block_info| block_info.basic_info().core_height)
            .unwrap_or_else(|| {
                self.genesis_block_info
                    .as_ref()
                    .map(|initialization_information| initialization_information.core_height)
                    .unwrap_or_default()
            })
    }

    /// The height of the core blockchain that Platform knows about through chain locks
    fn last_committed_known_core_height_or(&self, default: u32) -> u32 {
        self.last_committed_block_info
            .as_ref()
            .map(|block_info| block_info.basic_info().core_height)
            .unwrap_or_else(|| {
                self.genesis_block_info
                    .as_ref()
                    .map(|block_info| block_info.core_height)
                    .unwrap_or(default)
            })
    }

    /// The last block time in milliseconds
    fn last_committed_block_time_ms(&self) -> Option<u64> {
        self.last_committed_block_info
            .as_ref()
            .map(|block_info| block_info.basic_info().time_ms)
    }

    /// The last quorum hash
    fn last_committed_quorum_hash(&self) -> [u8; 32] {
        self.last_committed_block_info
            .as_ref()
            .map(|block_info| *block_info.quorum_hash())
            .unwrap_or_default()
    }

    /// The last committed block proposer's pro tx hash
    fn last_committed_block_proposer_pro_tx_hash(&self) -> [u8; 32] {
        self.last_committed_block_info
            .as_ref()
            .map(|block_info| *block_info.proposer_pro_tx_hash())
            .unwrap_or_default()
    }

    /// The last block signature
    fn last_committed_block_signature(&self) -> [u8; 96] {
        self.last_committed_block_info
            .as_ref()
            .map(|block_info| *block_info.signature())
            .unwrap_or([0u8; 96])
    }

    /// The last block app hash
    fn last_committed_block_app_hash(&self) -> Option<[u8; 32]> {
        self.last_committed_block_info
            .as_ref()
            .map(|block_info| *block_info.app_hash())
    }

    /// The last block round
    fn last_committed_block_round(&self) -> u32 {
        self.last_committed_block_info
            .as_ref()
            .map(|block_info| block_info.round())
            .unwrap_or_default()
    }

    /// The current epoch
    fn last_committed_block_epoch(&self) -> Epoch {
        self.last_committed_block_info
            .as_ref()
            .map(|block_info| block_info.basic_info().epoch)
            .unwrap_or_default()
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
                format!("current_validator_set: current validator quorum hash {} not in current known validator sets {} last committed block is {} (we might be processing new block)", self.current_validator_set_quorum_hash.to_string(), self.validator_sets.keys().into_iter().map(|quorum_hash| quorum_hash.to_string()).join(" | "),
                        self.last_committed_block_info.as_ref().map(|block_info| block_info.basic_info().height).unwrap_or_default()),
            )))
    }

    /// Returns information about the last committed block.
    fn last_committed_block_info(&self) -> &Option<ExtendedBlockInfo> {
        &self.last_committed_block_info
    }

    /// Get the current protocol version in consensus
    fn current_protocol_version_in_consensus(&self) -> ProtocolVersion {
        self.current_protocol_version_in_consensus
    }

    /// Patched platform version. Used to fix urgent bugs as not part of normal upgrade process.
    /// The patched version returns from the public current_platform_version getter in case if present.
    fn patched_platform_version(&self) -> Option<&'static PlatformVersion> {
        self.patched_platform_version
    }

    /// Returns the upcoming protocol version for the next epoch.
    fn next_epoch_protocol_version(&self) -> ProtocolVersion {
        self.next_epoch_protocol_version
    }

    /// Returns the quorum hash of the current validator set.
    fn current_validator_set_quorum_hash(&self) -> QuorumHash {
        self.current_validator_set_quorum_hash
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

    /// Returns the quorums used to validate chain locks.
    fn chain_lock_validating_quorums(&self) -> &SignatureVerificationQuorumSet {
        &self.chain_lock_validating_quorums
    }

    /// Returns the quorums used to validate instant locks.
    fn instant_lock_validating_quorums(&self) -> &SignatureVerificationQuorumSet {
        &self.instant_lock_validating_quorums
    }

    /// Returns the full list of masternodes.
    fn full_masternode_list(&self) -> &BTreeMap<ProTxHash, MasternodeListItem> {
        &self.full_masternode_list
    }

    /// Returns the list of high performance masternodes.
    fn hpmn_masternode_list(&self) -> &BTreeMap<ProTxHash, MasternodeListItem> {
        &self.hpmn_masternode_list
    }

    /// Returns information about the platform initialization state, if it exists.
    fn genesis_block_info(&self) -> Option<&BlockInfo> {
        self.genesis_block_info.as_ref()
    }

    fn last_block_info(&self) -> &BlockInfo {
        self.last_committed_block_info
            .as_ref()
            .map(|b| b.basic_info())
            .unwrap_or_else(|| {
                self.genesis_block_info
                    .as_ref()
                    .unwrap_or(&DEFAULT_BLOCK_INFO)
            })
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

    /// Sets the current chain lock validating quorums.
    fn set_chain_lock_validating_quorums(&mut self, quorums: SignatureVerificationQuorumSet) {
        self.chain_lock_validating_quorums = quorums;
    }

    /// Sets the current instant lock validating quorums.
    fn set_instant_lock_validating_quorums(&mut self, quorums: SignatureVerificationQuorumSet) {
        self.instant_lock_validating_quorums = quorums;
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
    fn set_genesis_block_info(&mut self, info: Option<BlockInfo>) {
        self.genesis_block_info = info;
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

    fn current_validator_set_quorum_hash_mut(&mut self) -> &mut QuorumHash {
        &mut self.current_validator_set_quorum_hash
    }

    fn next_validator_set_quorum_hash_mut(&mut self) -> &mut Option<QuorumHash> {
        &mut self.next_validator_set_quorum_hash
    }

    fn validator_sets_mut(&mut self) -> &mut IndexMap<QuorumHash, ValidatorSet> {
        &mut self.validator_sets
    }

    fn chain_lock_validating_quorums_mut(&mut self) -> &mut SignatureVerificationQuorumSet {
        &mut self.chain_lock_validating_quorums
    }

    fn instant_lock_validating_quorums_mut(&mut self) -> &mut SignatureVerificationQuorumSet {
        &mut self.instant_lock_validating_quorums
    }

    fn full_masternode_list_mut(&mut self) -> &mut BTreeMap<ProTxHash, MasternodeListItem> {
        &mut self.full_masternode_list
    }

    fn hpmn_masternode_list_mut(&mut self) -> &mut BTreeMap<ProTxHash, MasternodeListItem> {
        &mut self.hpmn_masternode_list
    }

    fn last_committed_block_epoch_ref(&self) -> &Epoch {
        self.last_committed_block_info
            .as_ref()
            .map(|block_info| &block_info.basic_info().epoch)
            .unwrap_or(&EPOCH_0)
    }

    /// The last block id hash
    fn last_committed_block_id_hash(&self) -> [u8; 32] {
        self.last_committed_block_info
            .as_ref()
            .map(|block_info| *block_info.block_id_hash())
            .unwrap_or_default()
    }

    /// Returns a reference to the previous feeversions
    fn previous_fee_versions(&self) -> &CachedEpochIndexFeeVersions {
        &self.previous_fee_versions
    }

    /// Returns a mutable reference to the previous feeversions
    fn previous_fee_versions_mut(&mut self) -> &mut CachedEpochIndexFeeVersions {
        &mut self.previous_fee_versions
    }

    fn full_masternode_list_changes(&self, previous: &PlatformStateV0) -> MasternodeListChanges {
        let mut new_masternodes = Vec::new();
        let mut removed_masternodes = Vec::new();
        let mut banned_masternodes = Vec::new();
        let mut unbanned_masternodes = Vec::new();
        let mut new_banned_masternodes = Vec::new();

        // Check for new, banned/unbanned, and new banned masternodes
        for (pro_tx_hash, current_item) in &self.full_masternode_list {
            if let Some(previous_item) = previous.full_masternode_list.get(pro_tx_hash) {
                let current_ban_height = current_item.state.pose_ban_height;
                let previous_ban_height = previous_item.state.pose_ban_height;

                if current_ban_height.is_some() && previous_ban_height.is_none() {
                    // Masternode was banned
                    banned_masternodes.push(*pro_tx_hash);
                    if previous_item.state.pose_ban_height.is_none() {
                        // New banned masternode
                        new_banned_masternodes.push(*pro_tx_hash);
                    }
                } else if current_ban_height.is_none() && previous_ban_height.is_some() {
                    // Masternode was unbanned
                    unbanned_masternodes.push(*pro_tx_hash);
                }
            } else {
                // New masternode
                new_masternodes.push(*pro_tx_hash);
                if current_item.state.pose_ban_height.is_some() {
                    // New banned masternode
                    new_banned_masternodes.push(*pro_tx_hash);
                }
            }
        }

        // Check for removed masternodes
        for pro_tx_hash in previous.full_masternode_list.keys() {
            if !self.full_masternode_list.contains_key(pro_tx_hash) {
                removed_masternodes.push(*pro_tx_hash);
            }
        }

        MasternodeListChanges {
            new_masternodes,
            removed_masternodes,
            banned_masternodes,
            unbanned_masternodes,
            new_banned_masternodes,
        }
    }

    fn hpmn_masternode_list_changes(&self, previous: &PlatformStateV0) -> MasternodeListChanges {
        let mut new_masternodes = Vec::new();
        let mut removed_masternodes = Vec::new();
        let mut banned_masternodes = Vec::new();
        let mut unbanned_masternodes = Vec::new();
        let mut new_banned_masternodes = Vec::new();

        // Check for new, banned/unbanned, and new banned masternodes
        for (pro_tx_hash, current_item) in &self.hpmn_masternode_list {
            if let Some(previous_item) = previous.hpmn_masternode_list.get(pro_tx_hash) {
                let current_ban_height = current_item.state.pose_ban_height;
                let previous_ban_height = previous_item.state.pose_ban_height;

                if current_ban_height.is_some() && previous_ban_height.is_none() {
                    // Masternode was banned
                    banned_masternodes.push(*pro_tx_hash);
                    if previous_item.state.pose_ban_height.is_none() {
                        // New banned masternode
                        new_banned_masternodes.push(*pro_tx_hash);
                    }
                } else if current_ban_height.is_none() && previous_ban_height.is_some() {
                    // Masternode was unbanned
                    unbanned_masternodes.push(*pro_tx_hash);
                }
            } else {
                // New masternode
                new_masternodes.push(*pro_tx_hash);
                if current_item.state.pose_ban_height.is_some() {
                    // New banned masternode
                    new_banned_masternodes.push(*pro_tx_hash);
                }
            }
        }

        // Check for removed masternodes
        for pro_tx_hash in previous.hpmn_masternode_list.keys() {
            if !self.hpmn_masternode_list.contains_key(pro_tx_hash) {
                removed_masternodes.push(*pro_tx_hash);
            }
        }

        MasternodeListChanges {
            new_masternodes,
            removed_masternodes,
            banned_masternodes,
            unbanned_masternodes,
            new_banned_masternodes,
        }
    }
}
