//! Saved platform state, structure 1: the record without its two large
//! collections. The masternode list and the validator sets are kept as one aux
//! entry per member ([`PlatformStateEntryKind`]), so a change to one member is
//! one small write and the record itself is small enough to write every block.

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::masternode::Masternode;
use crate::platform_types::platform_state::entry_changes::EntryChanges;
use crate::platform_types::platform_state::PlatformState;
use crate::platform_types::signature_verification_quorum_set::SignatureVerificationQuorumSetForSaving;
use bincode::Encode;
use dpp::bincode::config;
use dpp::block::block_info::BlockInfo;
use dpp::block::extended_block_info::ExtendedBlockInfo;
use dpp::core_types::validator_set::ValidatorSet;
use dpp::dashcore::hashes::Hash;
use dpp::dashcore::{ProTxHash, QuorumHash};
use dpp::dashcore_rpc::dashcore_rpc_json::{MasternodeListItem, MasternodeType};
use dpp::fee::default_costs::EpochIndexFeeVersionsForStorage;
use dpp::platform_serialization::de::Decode;
use dpp::platform_value::Bytes32;
use dpp::util::deserializer::ProtocolVersion;
use dpp::version::fee::FeeVersion;
use dpp::version::{PlatformVersion, TryIntoPlatformVersioned};
use dpp::ProtocolError;
use drive::drive::platform_state::{PlatformStateEntry, PlatformStateEntryKind};
use indexmap::IndexMap;
use std::collections::BTreeMap;

/// Platform state record, structure 1.
#[derive(Clone, Debug, Encode, Decode)]
pub struct PlatformStateForSavingV2 {
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
    /// The quorum hashes of the validator sets, in the state's order. The sets
    /// themselves are one entry each under
    /// [`PlatformStateEntryKind::ValidatorSets`]. The order is part of the
    /// state: rotation picks the next validator set by position.
    pub validator_set_quorum_hashes: Vec<Bytes32>,
    /// The quorums used for validating chain locks
    pub chain_lock_validating_quorums: SignatureVerificationQuorumSetForSaving,
    /// The quorums used for validating instant locks
    pub instant_lock_validating_quorums: SignatureVerificationQuorumSetForSaving,
    /// previous FeeVersions
    pub previous_fee_versions: EpochIndexFeeVersionsForStorage,
}

impl From<&PlatformState> for PlatformStateForSavingV2 {
    fn from(value: &PlatformState) -> Self {
        PlatformStateForSavingV2 {
            genesis_block_info: value.genesis_block_info,
            last_committed_block_info: value.last_committed_block_info.clone(),
            current_protocol_version_in_consensus: value.current_protocol_version_in_consensus,
            next_epoch_protocol_version: value.next_epoch_protocol_version,
            current_validator_set_quorum_hash: value
                .current_validator_set_quorum_hash
                .to_byte_array()
                .into(),
            next_validator_set_quorum_hash: value
                .next_validator_set_quorum_hash
                .map(|quorum_hash| quorum_hash.to_byte_array().into()),
            validator_set_quorum_hashes: value
                .validator_sets
                .keys()
                .map(|quorum_hash| quorum_hash.to_byte_array().into())
                .collect(),
            chain_lock_validating_quorums: value.chain_lock_validating_quorums.clone().into(),
            instant_lock_validating_quorums: value.instant_lock_validating_quorums.clone().into(),
            previous_fee_versions: value
                .previous_fee_versions
                .iter()
                .map(|(epoch_index, fee_version)| (*epoch_index, fee_version.fee_version_number))
                .collect(),
        }
    }
}

fn corrupted(message: impl Into<String>) -> Error {
    Error::Execution(ExecutionError::CorruptedCachedState(message.into()))
}

fn entry_config() -> config::Configuration<config::BigEndian, config::Varint, config::NoLimit> {
    config::standard().with_big_endian().with_no_limit()
}

fn hash_from_entry_key<H: Hash<Bytes = [u8; 32]>>(
    kind: PlatformStateEntryKind,
    key: &[u8],
) -> Result<H, Error> {
    let bytes: [u8; 32] = key.try_into().map_err(|_| {
        corrupted(format!(
            "{kind:?} entry key has {} bytes, expected 32",
            key.len()
        ))
    })?;
    Ok(H::from_byte_array(bytes))
}

/// The bytes of a masternode's entry under [`PlatformStateEntryKind::Masternodes`].
pub fn serialize_masternode_entry(
    masternode: &MasternodeListItem,
    platform_version: &PlatformVersion,
) -> Result<Vec<u8>, Error> {
    let masternode: Masternode = masternode
        .clone()
        .try_into_platform_versioned(platform_version)?;
    bincode::encode_to_vec(masternode, entry_config()).map_err(|e| {
        ProtocolError::PlatformSerializationError(format!(
            "unable to serialize masternode entry: {e}"
        ))
        .into()
    })
}

/// The masternode a [`serialize_masternode_entry`] entry holds.
pub fn deserialize_masternode_entry(bytes: &[u8]) -> Result<MasternodeListItem, Error> {
    bincode::decode_from_slice::<Masternode, _>(bytes, entry_config())
        .map(|(masternode, _)| masternode.into())
        .map_err(|e| {
            ProtocolError::PlatformDeserializationError(format!(
                "unable to deserialize masternode entry: {e}"
            ))
            .into()
        })
}

/// The bytes of a validator set's entry under [`PlatformStateEntryKind::ValidatorSets`].
pub fn serialize_validator_set_entry(validator_set: &ValidatorSet) -> Result<Vec<u8>, Error> {
    bincode::encode_to_vec(validator_set, entry_config()).map_err(|e| {
        ProtocolError::PlatformSerializationError(format!(
            "unable to serialize validator set entry: {e}"
        ))
        .into()
    })
}

/// The validator set a [`serialize_validator_set_entry`] entry holds.
pub fn deserialize_validator_set_entry(bytes: &[u8]) -> Result<ValidatorSet, Error> {
    bincode::decode_from_slice::<ValidatorSet, _>(bytes, entry_config())
        .map(|(validator_set, _)| validator_set)
        .map_err(|e| {
            ProtocolError::PlatformDeserializationError(format!(
                "unable to deserialize validator set entry: {e}"
            ))
            .into()
        })
}

impl PlatformStateForSavingV2 {
    /// Rebuilds the state from the record and the entries read from disk, as
    /// `(key, bytes)` pairs with the collection prefix removed.
    ///
    /// The HPMN list is not stored: it is the Evo subset of the full list. A
    /// validator set the record lists without an entry is corruption; an entry
    /// the record does not list is ignored with a warning.
    pub fn into_platform_state(
        self,
        masternode_entries: Vec<PlatformStateEntry>,
        validator_set_entries: Vec<PlatformStateEntry>,
    ) -> Result<PlatformState, Error> {
        let mut full_masternode_list = BTreeMap::new();
        let mut hpmn_masternode_list = BTreeMap::new();
        for (key, bytes) in masternode_entries {
            let pro_tx_hash: ProTxHash =
                hash_from_entry_key(PlatformStateEntryKind::Masternodes, &key)?;
            let masternode = deserialize_masternode_entry(&bytes)?;
            if masternode.pro_tx_hash != pro_tx_hash {
                return Err(corrupted(format!(
                    "masternode entry {pro_tx_hash} holds masternode {}",
                    masternode.pro_tx_hash
                )));
            }
            if masternode.node_type == MasternodeType::Evo {
                hpmn_masternode_list.insert(pro_tx_hash, masternode.clone());
            }
            full_masternode_list.insert(pro_tx_hash, masternode);
        }

        let mut stored_validator_sets: BTreeMap<QuorumHash, ValidatorSet> = validator_set_entries
            .into_iter()
            .map(|(key, bytes)| {
                let quorum_hash: QuorumHash =
                    hash_from_entry_key(PlatformStateEntryKind::ValidatorSets, &key)?;
                Ok((quorum_hash, deserialize_validator_set_entry(&bytes)?))
            })
            .collect::<Result<_, Error>>()?;

        let mut validator_sets = IndexMap::with_capacity(self.validator_set_quorum_hashes.len());
        for quorum_hash in self.validator_set_quorum_hashes {
            let quorum_hash = QuorumHash::from_byte_array(quorum_hash.to_buffer());
            let validator_set = stored_validator_sets.remove(&quorum_hash).ok_or_else(|| {
                corrupted(format!(
                    "the saved platform state lists validator set {quorum_hash} but has no entry for it"
                ))
            })?;
            validator_sets.insert(quorum_hash, validator_set);
        }
        if !stored_validator_sets.is_empty() {
            tracing::warn!(
                quorum_hashes = ?stored_validator_sets.keys().map(ToString::to_string).collect::<Vec<_>>(),
                "validator set entries not listed by the saved platform state; ignored"
            );
        }

        Ok(PlatformState {
            genesis_block_info: self.genesis_block_info,
            last_committed_block_info: self.last_committed_block_info,
            current_protocol_version_in_consensus: self.current_protocol_version_in_consensus,
            next_epoch_protocol_version: self.next_epoch_protocol_version,
            current_validator_set_quorum_hash: QuorumHash::from_byte_array(
                self.current_validator_set_quorum_hash.to_buffer(),
            ),
            next_validator_set_quorum_hash: self
                .next_validator_set_quorum_hash
                .map(|bytes| QuorumHash::from_byte_array(bytes.to_buffer())),
            validator_sets,
            chain_lock_validating_quorums: self.chain_lock_validating_quorums.into(),
            instant_lock_validating_quorums: self.instant_lock_validating_quorums.into(),
            full_masternode_list,
            hpmn_masternode_list,
            previous_fee_versions: self
                .previous_fee_versions
                .into_iter()
                .map(|(epoch_index, fee_version_number)| {
                    (
                        epoch_index,
                        FeeVersion::get(fee_version_number)
                            .expect("expected fee version number to exist"),
                    )
                })
                .collect(),
            // The record was written under structure 1, so its entries are on
            // disk and match it: nothing is pending. The flag only drives the
            // structure 0 store, which starts from a full write after a restart.
            heavy_fields_dirty: true,
            masternode_changes: EntryChanges::default(),
            validator_set_changes: EntryChanges::default(),
        })
    }
}
