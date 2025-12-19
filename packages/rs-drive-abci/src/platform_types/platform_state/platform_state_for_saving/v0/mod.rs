use crate::platform_types::masternode::Masternode;
use crate::platform_types::platform_state::PlatformState;
use crate::platform_types::signature_verification_quorum_set::SignatureVerificationQuorumSetForSaving;
use bincode::Encode;
use dpp::block::block_info::BlockInfo;
use dpp::block::extended_block_info::ExtendedBlockInfo;
use dpp::dashcore::hashes::Hash;
use dpp::dashcore::{ProTxHash, QuorumHash};
use dpp::fee::default_costs::CachedEpochIndexFeeVersionsFieldsBeforeVersion4;
use dpp::platform_serialization::de::Decode;
use dpp::platform_value::Bytes32;
use dpp::util::deserializer::ProtocolVersion;
use dpp::version::fee::FeeVersion;
use std::collections::BTreeMap;

mod old_structures;

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
    pub validator_sets: Vec<(Bytes32, old_structures::OldStructureValidatorSet)>,

    /// The quorums used for validating chain locks
    pub chain_lock_validating_quorums: SignatureVerificationQuorumSetForSaving,

    /// The quorums used for validating instant locks
    pub instant_lock_validating_quorums: SignatureVerificationQuorumSetForSaving,

    /// current full masternode list
    pub full_masternode_list: BTreeMap<Bytes32, Masternode>,

    /// current HPMN masternode list
    pub hpmn_masternode_list: BTreeMap<Bytes32, Masternode>,

    /// previous FeeVersions
    pub previous_fee_versions: CachedEpochIndexFeeVersionsFieldsBeforeVersion4,
}

impl From<PlatformStateForSavingV0> for PlatformState {
    fn from(value: PlatformStateForSavingV0) -> Self {
        PlatformState {
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
                .map(|(k, v)| (QuorumHash::from_byte_array(k.to_buffer()), v.into()))
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
            previous_fee_versions: value
                .previous_fee_versions
                .into_keys()
                .map(|epoch_index| (epoch_index, FeeVersion::first()))
                .collect(),
        }
    }
}
