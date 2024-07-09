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
use dpp::prelude::CachedEpochIndexFeeVersions;
use itertools::Itertools;
use std::collections::BTreeMap;
use std::fmt::{Debug, Formatter};
use crate::platform_types::platform_state::v0::PlatformStateV0;

fn hex_encoded_validator_sets(validator_sets: &IndexMap<QuorumHash, ValidatorSet>) -> String {
    let entries = validator_sets
        .iter()
        .map(|(k, v)| format!("{:?}: {:?}", k.to_string(), v))
        .collect::<Vec<_>>();
    format!("{:?}", entries)
}

/// Platform state
#[derive(Clone, Debug, Encode, Decode)]
pub struct PlatformStateForSavingV1 {
    /// Information about the genesis block
    pub genesis_block_info: Option<BlockInfo>,
    /// Information about the last block
    pub last_committed_block_info: Option<ExtendedBlockInfo>,
    /// Current Version
    pub current_protocol_version_in_consensus: String,
    /// upcoming protocol version
    pub next_epoch_protocol_version: String,
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

impl TryFrom<PlatformStateV0> for PlatformStateForSavingV1 {
    type Error = Error;

    fn try_from(value: PlatformStateV0) -> Result<Self, Self::Error> {
        let platform_version = PlatformVersion::get(value.current_protocol_version_in_consensus)?;
        Ok(PlatformStateForSavingV1 {
            genesis_block_info: value.genesis_block_info,
            last_committed_block_info: value.last_committed_block_info,
            current_protocol_version_in_consensus: value.current_protocol_version_in_consensus.to_string(),
            next_epoch_protocol_version: value.next_epoch_protocol_version.to_string(),
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

impl From<PlatformStateForSavingV1> for PlatformStateV0 {
    fn from(value: PlatformStateForSavingV1) -> Self {
        PlatformStateV0 {
            genesis_block_info: value.genesis_block_info,
            last_committed_block_info: value.last_committed_block_info,
            current_protocol_version_in_consensus: value.current_protocol_version_in_consensus.parse().unwrap_or(0),
            next_epoch_protocol_version: value.next_epoch_protocol_version.parse().unwrap_or(0),
            current_validator_set_quorum_hash: QuorumHash::from_byte_array(
                value.current_validator_set_quorum_hash.to_buffer(),
            ),
            next_validator_set_quorum_hash: value
                .next_validator_set_quorum_hash
                .map(|bytes| QuorumHash::from_byte_array(bytes.to_buffer())),
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
