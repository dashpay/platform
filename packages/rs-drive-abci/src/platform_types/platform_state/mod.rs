mod accessors;
pub mod entry_changes;
mod masternode_list_changes;
/// The saved forms of the platform state, one structure per version.
pub mod platform_state_for_saving;
pub mod recent;

use crate::error::Error;

use crate::platform_types::validator_set::ValidatorSet;
use derive_more::From;
use dpp::bincode::config;
use dpp::block::extended_block_info::ExtendedBlockInfo;
use dpp::dashcore::{ProTxHash, QuorumHash};
use dpp::serialization::{
    PlatformDeserializableFromVersionedStructureTrusted, PlatformSerializable,
};
use dpp::util::deserializer::ProtocolVersion;

use dpp::version::{PlatformVersion, TryFromPlatformVersioned, TryIntoPlatformVersioned};
use dpp::ProtocolError;
use indexmap::IndexMap;

use crate::config::PlatformConfig;
use crate::error::execution::ExecutionError;
pub use crate::platform_types::platform_state::accessors::PlatformStateV0Methods;
use crate::platform_types::platform_state::entry_changes::EntryChanges;
use crate::platform_types::platform_state::platform_state_for_saving::v1::PlatformStateForSavingV1;
use crate::platform_types::platform_state::platform_state_for_saving::v2::PlatformStateForSavingV2;
use crate::platform_types::platform_state::platform_state_for_saving::PlatformStateForSaving;
use crate::platform_types::signature_verification_quorum_set::SignatureVerificationQuorumSet;
use dpp::block::block_info::BlockInfo;
use dpp::dashcore::hashes::Hash;
use dpp::dashcore_rpc::json::MasternodeListItem;
use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
use dpp::util::hash::hash_double;
use std::collections::BTreeMap;
use std::fmt::{Debug, Formatter};

/// Platform state
#[derive(Clone)]
pub struct PlatformState {
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

    /// previous FeeVersions
    pub previous_fee_versions: CachedEpochIndexFeeVersions,

    /// True when a field carried only by the full saved record has changed since
    /// the state was last written in full. The masternode lists, validator sets
    /// and quorum sets are over a megabyte on mainnet and change on a minority of
    /// blocks, so the full record is rewritten only when this is set; every block
    /// still writes the small record holding the block info and quorum hashes.
    /// Not part of the saved record: a state read back from disk starts dirty.
    ///
    /// Drives the structure 0 store only. Under structure 1 the record is small
    /// and written every block, and the large collections track their own
    /// changes below.
    pub heavy_fields_dirty: bool,

    /// Which masternode entries the next store writes or deletes. Not part of
    /// the saved record.
    pub masternode_changes: EntryChanges<ProTxHash>,

    /// Which validator set entries the next store writes or deletes. Not part
    /// of the saved record.
    pub validator_set_changes: EntryChanges<QuorumHash>,
}

fn hex_encoded_validator_sets(validator_sets: &IndexMap<QuorumHash, ValidatorSet>) -> String {
    let entries = validator_sets
        .iter()
        .map(|(k, v)| format!("{:?}: {:?}", k.to_string(), v))
        .collect::<Vec<_>>();
    format!("{:?}", entries)
}

impl Debug for PlatformState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlatformState")
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
            .field("previous_fee_versions", &self.previous_fee_versions)
            .field(
                "chain_lock_validating_quorums",
                &self.chain_lock_validating_quorums,
            )
            .field(
                "instant_lock_validating_quorums",
                &self.instant_lock_validating_quorums,
            )
            .field("heavy_fields_dirty", &self.heavy_fields_dirty)
            .field("masternode_changes", &self.masternode_changes)
            .field("validator_set_changes", &self.validator_set_changes)
            .finish()
    }
}

impl PlatformState {
    /// Get the state fingerprint: a hash over the whole state, the per-entry
    /// collections included.
    pub fn fingerprint(&self) -> Result<[u8; 32], Error> {
        Ok(hash_double(self.serialize_standalone_to_bytes()?))
    }

    /// The whole state in one record, for a standalone snapshot such as the
    /// file written beside a checkpoint. Structure 1 keeps the masternode list
    /// and the validator sets as entries in the database, so a structure 1
    /// record cannot be read on its own; this always writes structure 0.
    pub fn serialize_standalone_to_bytes(&self) -> Result<Vec<u8>, Error> {
        let config = config::standard().with_big_endian().with_no_limit();
        let saving_v1: PlatformStateForSavingV1 = self.try_into()?;
        bincode::encode_to_vec(PlatformStateForSaving::V1(saving_v1), config).map_err(|e| {
            ProtocolError::PlatformSerializationError(format!(
                "unable to serialize PlatformState: {}",
                e
            ))
            .into()
        })
    }

    /// Nothing on disk can be trusted to match this state: the next store
    /// rewrites the full record and every entry of the per-entry collections.
    pub fn mark_all_unsaved(&mut self) {
        self.heavy_fields_dirty = true;
        self.masternode_changes.mark_rewrite_all();
        self.validator_set_changes.mark_rewrite_all();
    }

    /// What the store just wrote is now what is on disk for this state.
    pub fn mark_saved(&mut self) {
        self.heavy_fields_dirty = false;
        self.masternode_changes.clear();
        self.validator_set_changes.clear();
    }
    /// The default state at init chain
    pub fn default_with_protocol_versions(
        current_protocol_version_in_consensus: ProtocolVersion,
        next_epoch_protocol_version: ProtocolVersion,
        config: &PlatformConfig,
    ) -> Result<PlatformState, Error> {
        let platform_version = PlatformVersion::get(current_protocol_version_in_consensus)?;

        let state = PlatformState {
            last_committed_block_info: None,
            current_protocol_version_in_consensus,
            next_epoch_protocol_version,
            current_validator_set_quorum_hash: QuorumHash::all_zeros(),
            next_validator_set_quorum_hash: None,
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
            heavy_fields_dirty: true,
            masternode_changes: EntryChanges::all(),
            validator_set_changes: EntryChanges::all(),
        };

        Ok(state)
    }
}

impl PlatformSerializable for PlatformState {
    type Error = Error;

    fn serialize_to_bytes(&self) -> Result<Vec<u8>, Self::Error> {
        let platform_version = self.current_platform_version()?;
        let config = config::standard().with_big_endian().with_no_limit();
        let platform_state_for_saving: PlatformStateForSaving =
            self.try_into_platform_versioned(platform_version)?;
        bincode::encode_to_vec(platform_state_for_saving, config).map_err(|e| {
            ProtocolError::PlatformSerializationError(format!(
                "unable to serialize PlatformState: {}",
                e
            ))
            .into()
        })
    }
}

impl PlatformDeserializableFromVersionedStructureTrusted for PlatformState {
    fn versioned_deserialize_trusted(
        data: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized,
    {
        let config = config::standard().with_big_endian().with_no_limit();
        let platform_state_in_save_format: PlatformStateForSaving =
            bincode::decode_from_slice(data, config)
                .map_err(|e| {
                    ProtocolError::PlatformDeserializationError(format!(
                        "unable to deserialize PlatformStateForSaving: {}",
                        e
                    ))
                })?
                .0;

        platform_state_in_save_format
            .try_into_platform_versioned(platform_version)
            .map_err(|e: Error| ProtocolError::Generic(e.to_string()))
    }
}

impl TryFromPlatformVersioned<&PlatformState> for PlatformStateForSaving {
    type Error = Error;
    fn try_from_platform_versioned(
        value: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Self::Error> {
        match platform_version
            .drive_abci
            .structs
            .platform_state_for_saving_structure_default
        {
            0 => {
                let saving_v1: PlatformStateForSavingV1 = value.try_into()?;
                Ok(saving_v1.into())
            }
            1 => Ok(PlatformStateForSavingV2::from(value).into()),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "PlatformStateForSaving::try_from_platform_versioned(&PlatformState)"
                    .to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}

impl TryFromPlatformVersioned<PlatformState> for PlatformStateForSaving {
    type Error = Error;
    fn try_from_platform_versioned(
        value: PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Self::Error> {
        match platform_version
            .drive_abci
            .structs
            .platform_state_for_saving_structure_default
        {
            0 => {
                let saving_v1: PlatformStateForSavingV1 = value.try_into()?;
                Ok(saving_v1.into())
            }
            1 => Ok(PlatformStateForSavingV2::from(&value).into()),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "PlatformStateForSaving::try_from_platform_versioned(PlatformState)"
                    .to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}

impl TryFromPlatformVersioned<PlatformStateForSaving> for PlatformState {
    type Error = Error;

    fn try_from_platform_versioned(
        value: PlatformStateForSaving,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Self::Error> {
        match value {
            PlatformStateForSaving::V0(v0) => {
                match platform_version.drive_abci.structs.platform_state_structure {
                    0 => Ok(PlatformState::from(v0)),
                    version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                        method:
                            "PlatformState::try_from_platform_versioned(PlatformStateForSavingV0)"
                                .to_string(),
                        known_versions: vec![0],
                        received: version,
                    })),
                }
            }
            PlatformStateForSaving::V1(v1) => {
                match platform_version.drive_abci.structs.platform_state_structure {
                    0 => PlatformState::try_from(v1),
                    version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                        method:
                            "PlatformState::try_from_platform_versioned(PlatformStateForSavingV1)"
                                .to_string(),
                        known_versions: vec![0],
                        received: version,
                    })),
                }
            }
            // The record alone is not the state: its masternode list and
            // validator sets are entries in the database, read together with it
            // by `fetch_platform_state`.
            PlatformStateForSaving::V2(_) => {
                Err(Error::Execution(ExecutionError::CorruptedCachedState(
                    "a structure 1 platform state record is only readable together with its \
                     entries; load it through fetch_platform_state"
                        .to_string(),
                )))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod versioned_deserialize {
        use super::*;
        use crate::platform_types::platform_state::platform_state_for_saving::v2::{
            serialize_masternode_entry, serialize_validator_set_entry,
        };
        use crate::test::fixture::platform_state::{
            PLATFORM_STATE_V3_TESTNET, PLATFORM_STATE_V8_DEVNET,
        };
        use dpp::block::epoch::{Epoch, EpochIndex};
        use dpp::fee::default_costs::{EpochCosts, KnownCostItem};
        use dpp::version::fee::{FeeVersion, FEE_VERSIONS};
        use dpp::version::mocks::fee_test::{
            TEST_FEE_VERSION_DOUBLED_STORAGE_RATE, TEST_FEE_VERSION_NUMBER_DOUBLED_STORAGE_RATE,
        };
        use platform_version::version::v3::PLATFORM_V3;
        use platform_version::version::v9::PLATFORM_V9;
        use platform_version::version::LATEST_VERSION;
        use std::ops::Deref;

        /// Every `KnownCostItem` variant, with a few sizes for the two sized variants.
        fn every_known_cost_item() -> Vec<KnownCostItem> {
            let mut items = vec![
                KnownCostItem::StorageDiskUsageCreditPerByte,
                KnownCostItem::StorageProcessingCreditPerByte,
                KnownCostItem::StorageLoadCreditPerByte,
                KnownCostItem::NonStorageLoadCreditPerByte,
                KnownCostItem::StorageSeekCost,
                KnownCostItem::FetchIdentityBalanceProcessingCost,
                KnownCostItem::FetchSingleIdentityKeyProcessingCost,
                KnownCostItem::VerifySignatureEcdsaSecp256k1,
                KnownCostItem::VerifySignatureBLS12_381,
                KnownCostItem::VerifySignatureEcdsaHash160,
                KnownCostItem::VerifySignatureBip13ScriptHash,
                KnownCostItem::VerifySignatureEddsa25519Hash160,
            ];
            for size in [0, 1, 64] {
                items.push(KnownCostItem::SingleSHA256(size));
                items.push(KnownCostItem::Blake3(size));
            }
            items
        }

        fn latest_state() -> PlatformState {
            PlatformState::default_with_protocol_versions(
                LATEST_VERSION,
                LATEST_VERSION,
                &PlatformConfig::default_testnet(),
            )
            .expect("default state")
        }

        /// Through the standalone snapshot, which always writes structure 0: the
        /// structure 1 record the latest version writes per block is only readable
        /// together with its entries, which `round_trip_through_entries` covers.
        fn round_trip(state: &PlatformState) -> PlatformState {
            let bytes = state
                .serialize_standalone_to_bytes()
                .expect("serialize state");
            PlatformState::versioned_deserialize_trusted(&bytes, PlatformVersion::latest())
                .expect("deserialize state")
        }

        /// Through the structure 1 record and its per-member entries, the path
        /// a node takes on restart.
        fn round_trip_through_entries(state: &PlatformState) -> PlatformState {
            let record = PlatformStateForSavingV2::from(state);
            let masternode_entries = state
                .full_masternode_list()
                .iter()
                .map(|(pro_tx_hash, masternode)| {
                    Ok((
                        pro_tx_hash.to_byte_array().to_vec(),
                        serialize_masternode_entry(masternode, PlatformVersion::latest())?,
                    ))
                })
                .collect::<Result<Vec<_>, Error>>()
                .expect("serialize masternode entries");
            let validator_set_entries = state
                .validator_sets()
                .iter()
                .map(|(quorum_hash, validator_set)| {
                    Ok((
                        quorum_hash.to_byte_array().to_vec(),
                        serialize_validator_set_entry(validator_set)?,
                    ))
                })
                .collect::<Result<Vec<_>, Error>>()
                .expect("serialize validator set entries");
            record
                .into_platform_state(masternode_entries, validator_set_entries)
                .expect("rebuild state from record and entries")
        }

        fn assert_every_entry_is_fee_version_one(state: &PlatformState) {
            let registered = FeeVersion::get(1).expect("number 1 is registered");
            for (epoch_index, fee_version) in state.previous_fee_versions() {
                assert_eq!(
                    fee_version.fee_version_number, 1,
                    "epoch {epoch_index} must resolve to fee version number 1"
                );
                assert_eq!(*fee_version, registered);
            }
        }

        #[test]
        fn should_deserialize_state_stored_in_version_0_from_testnet() {
            let serialized_state =
                hex::decode(PLATFORM_STATE_V3_TESTNET.deref()).expect("failed to decode hex");

            PlatformState::versioned_deserialize_trusted(&serialized_state, &PLATFORM_V3)
                .expect("failed to deserialize state");
        }

        /// Serializing through the borrowed conversion must preserve the saved format.
        #[test]
        fn should_preserve_pre_change_serialization_hash() {
            let serialized_state =
                hex::decode(PLATFORM_STATE_V8_DEVNET.deref()).expect("failed to decode hex");

            let state =
                PlatformState::versioned_deserialize_trusted(&serialized_state, &PLATFORM_V9)
                    .expect("failed to deserialize state");

            // Generated with serialize_to_bytes() at pre-change commit
            // 9dfffa611a9554cb14c9464374c8de1356c1d92f, using the fixture above.
            assert_eq!(
                hex::encode(hash_double(
                    state.serialize_to_bytes().expect("borrowed serialize")
                )),
                "079e5cb38c07a9e1818a4a71a40fe8936e7c93bf2fec39c7d346afa215b76679"
            );
        }

        #[test]
        fn should_deserialize_state_stored_in_version_8_from_devnet() {
            let serialized_state =
                hex::decode(PLATFORM_STATE_V8_DEVNET.deref()).expect("failed to decode hex");

            PlatformState::versioned_deserialize_trusted(&serialized_state, &PLATFORM_V9)
                .expect("failed to deserialize state");
        }

        #[test]
        fn should_still_load_legacy_v0_states_as_fee_version_one() {
            // The pre-1.4 format stored whole fee version structs. Only number 1 existed then,
            // so every stored epoch maps to the first registered generation.
            let serialized_state =
                hex::decode(PLATFORM_STATE_V3_TESTNET.deref()).expect("failed to decode hex");
            let state =
                PlatformState::versioned_deserialize_trusted(&serialized_state, &PLATFORM_V3)
                    .expect("failed to deserialize state");

            assert_every_entry_is_fee_version_one(&state);
        }

        #[test]
        fn should_resolve_stored_numbers_in_v1_states_to_registered_fee_versions() {
            let serialized_state =
                hex::decode(PLATFORM_STATE_V8_DEVNET.deref()).expect("failed to decode hex");
            let state =
                PlatformState::versioned_deserialize_trusted(&serialized_state, &PLATFORM_V9)
                    .expect("failed to deserialize state");

            assert_every_entry_is_fee_version_one(&state);
        }

        #[test]
        fn should_round_trip_every_registered_fee_version_number_through_saved_state() {
            let mut state = latest_state();
            for registered in FEE_VERSIONS {
                let epoch_index = registered.fee_version_number as EpochIndex;
                state
                    .previous_fee_versions_mut()
                    .insert(epoch_index, registered);
            }

            let reloaded = round_trip(&state);

            assert_eq!(
                reloaded.previous_fee_versions().len(),
                FEE_VERSIONS.len(),
                "every registered number survives the round trip"
            );
            for (epoch_index, fee_version) in state.previous_fee_versions() {
                let reloaded_fee_version = reloaded
                    .previous_fee_versions()
                    .get(epoch_index)
                    .expect("epoch entry survives the round trip");
                assert_eq!(
                    reloaded_fee_version.fee_version_number,
                    fee_version.fee_version_number
                );
                assert_eq!(*reloaded_fee_version, *fee_version);
                assert_eq!(
                    *reloaded_fee_version,
                    FeeVersion::get(fee_version.fee_version_number).expect("registered")
                );
            }
        }

        #[test]
        fn should_round_trip_a_fee_version_number_that_is_not_a_registry_position() {
            // The mock generation's number lives above the test shift, so it is never a position
            // in the shipped registry. It must come back from saved state through a lookup by
            // carried number, and the reloaded entry must price storage at its own rate.
            let mut state = latest_state();
            let mock = TEST_FEE_VERSION_DOUBLED_STORAGE_RATE
                .as_static()
                .expect("mock generation is registered");
            state
                .previous_fee_versions_mut()
                .insert(0, FeeVersion::get(1).expect("registered"));
            state.previous_fee_versions_mut().insert(10, mock);

            let saving = PlatformStateForSavingV1::try_from(state.clone()).expect("saving form");
            assert_eq!(
                saving.previous_fee_versions.get(&10).copied(),
                Some(TEST_FEE_VERSION_NUMBER_DOUBLED_STORAGE_RATE),
                "the stored number is the carried number, not a position"
            );

            let reloaded = round_trip(&state);
            let reloaded_mock = reloaded
                .previous_fee_versions()
                .get(&10)
                .expect("boundary entry survives the round trip");
            assert_eq!(
                reloaded_mock.fee_version_number,
                TEST_FEE_VERSION_NUMBER_DOUBLED_STORAGE_RATE
            );
            assert_eq!(*reloaded_mock, mock);
            for epoch_index in [9, 10, 11] {
                let epoch = Epoch::new(epoch_index).expect("epoch");
                assert_eq!(
                    epoch.cost_for_known_cost_item(
                        reloaded.previous_fee_versions(),
                        KnownCostItem::StorageDiskUsageCreditPerByte
                    ),
                    epoch.cost_for_known_cost_item(
                        state.previous_fee_versions(),
                        KnownCostItem::StorageDiskUsageCreditPerByte
                    ),
                    "epoch {epoch_index} prices storage the same before and after reload"
                );
            }
            assert_eq!(
                Epoch::new(10).expect("epoch").cost_for_known_cost_item(
                    reloaded.previous_fee_versions(),
                    KnownCostItem::StorageDiskUsageCreditPerByte
                ),
                TEST_FEE_VERSION_DOUBLED_STORAGE_RATE
                    .storage
                    .storage_disk_usage_credit_per_byte
            );
        }

        #[test]
        fn should_agree_with_the_in_memory_fee_history_on_every_known_cost_item_after_reload() {
            // The epoch change hook stores a reference into the platform version table, not the
            // registry entry. After a reload the map holds the registry entry. Both must serve
            // the same values for every epoch and every cost item.
            let mut state = latest_state();
            state
                .previous_fee_versions_mut()
                .insert(1, &PlatformVersion::latest().fee_version);

            let reloaded = round_trip(&state);

            assert_eq!(
                reloaded.previous_fee_versions().keys().collect::<Vec<_>>(),
                state.previous_fee_versions().keys().collect::<Vec<_>>()
            );
            for epoch_index in 0..=3 {
                let epoch = Epoch::new(epoch_index).expect("epoch");
                for item in every_known_cost_item() {
                    assert_eq!(
                        epoch.cost_for_known_cost_item(state.previous_fee_versions(), item),
                        epoch.cost_for_known_cost_item(reloaded.previous_fee_versions(), item),
                        "epoch {epoch_index} disagrees after reload"
                    );
                }
            }
        }

        #[test]
        fn should_round_trip_every_registered_fee_version_number_through_the_structure_1_record() {
            let mut state = latest_state();
            for registered in FEE_VERSIONS {
                let epoch_index = registered.fee_version_number as EpochIndex;
                state
                    .previous_fee_versions_mut()
                    .insert(epoch_index, registered);
            }
            // The mock generation's number is not a registry position, so restoring it proves
            // the structure 1 load path resolves by carried number.
            let mock = TEST_FEE_VERSION_DOUBLED_STORAGE_RATE
                .as_static()
                .expect("mock generation is registered");
            state.previous_fee_versions_mut().insert(10, mock);

            let reloaded = round_trip_through_entries(&state);

            assert_eq!(
                reloaded.previous_fee_versions(),
                state.previous_fee_versions(),
                "every registered number survives the structure 1 round trip"
            );
            assert_eq!(
                reloaded
                    .previous_fee_versions()
                    .get(&10)
                    .map(|fee_version| fee_version.fee_version_number),
                Some(TEST_FEE_VERSION_NUMBER_DOUBLED_STORAGE_RATE)
            );
        }

        #[test]
        fn should_reject_a_structure_1_record_that_stores_an_unknown_fee_version_number() {
            let state = latest_state();
            let mut record = PlatformStateForSavingV2::from(&state);
            record.previous_fee_versions.insert(3, 99);

            let error = record
                .into_platform_state(Vec::new(), Vec::new())
                .expect_err("an unknown fee version number must not load");
            let message = error.to_string();
            assert!(message.contains("99"), "error names the number: {message}");
            assert!(
                message.contains("epoch 3"),
                "error names the epoch: {message}"
            );
        }

        #[test]
        fn should_reject_a_saved_state_that_stores_an_unknown_fee_version_number() {
            let state = latest_state();
            let mut saving = PlatformStateForSavingV1::try_from(state).expect("saving form");
            saving.previous_fee_versions.insert(3, 99);
            let config = config::standard().with_big_endian().with_no_limit();
            let bytes = bincode::encode_to_vec(PlatformStateForSaving::V1(saving), config)
                .expect("encode saving form");

            let error =
                PlatformState::versioned_deserialize_trusted(&bytes, PlatformVersion::latest())
                    .expect_err("an unknown fee version number must not load");
            let message = error.to_string();
            assert!(message.contains("99"), "error names the number: {message}");
            assert!(
                message.contains("epoch 3"),
                "error names the epoch: {message}"
            );
        }
    }
}
