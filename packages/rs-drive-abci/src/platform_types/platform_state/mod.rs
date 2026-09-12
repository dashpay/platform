mod accessors;
mod masternode_list_changes;
mod platform_state_for_saving;

use crate::error::Error;

use crate::platform_types::validator_set::ValidatorSet;
use derive_more::From;
use dpp::bincode::config;
use dpp::block::extended_block_info::ExtendedBlockInfo;
use dpp::dashcore::{ProTxHash, QuorumHash};
use dpp::serialization::{PlatformDeserializableFromVersionedStructure, PlatformSerializable};
use dpp::util::deserializer::ProtocolVersion;

use dpp::version::{PlatformVersion, TryFromPlatformVersioned, TryIntoPlatformVersioned};
use dpp::ProtocolError;
use indexmap::IndexMap;

use crate::config::PlatformConfig;
use crate::error::execution::ExecutionError;
pub use crate::platform_types::platform_state::accessors::PlatformStateV0Methods;
use crate::platform_types::platform_state::platform_state_for_saving::v1::PlatformStateForSavingV1;
use crate::platform_types::platform_state::platform_state_for_saving::PlatformStateForSaving;
use crate::platform_types::signature_verification_quorum_set::SignatureVerificationQuorumSet;
use dpp::block::block_info::BlockInfo;
use dpp::dashcore::hashes::Hash;
use dpp::dashcore_rpc::json::MasternodeListItem;
use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
use dpp::fee::epoch::GENESIS_EPOCH_INDEX;
use dpp::util::hash::hash_double;
use dpp::version::fee::FeeVersion;
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
            .finish()
    }
}

impl PlatformState {
    /// Get the state fingerprint
    pub fn fingerprint(&self) -> Result<[u8; 32], Error> {
        Ok(hash_double(self.serialize_to_bytes()?))
    }
    /// The default state at init chain
    pub fn default_with_protocol_versions(
        current_protocol_version_in_consensus: ProtocolVersion,
        next_epoch_protocol_version: ProtocolVersion,
        config: &PlatformConfig,
    ) -> Result<PlatformState, Error> {
        let platform_version = PlatformVersion::get(current_protocol_version_in_consensus)?;

        // Record the genesis fee generation. The epoch-change hook only records a
        // generation on the first non-genesis epoch change, so without this entry
        // bytes stored in epoch 0 of a network whose genesis schedule is not
        // generation 1 would be refunded at generation 1 rates forever (the lookup
        // falls back to the first registered generation below the earliest entry).
        // Unobservable on every existing network: their genesis generation is 1,
        // which is exactly the fallback, and the map lives in the saved state, not
        // in the app hash. Saved states created before this entry was recorded
        // keep working through the same fallback. The value is the registry entry
        // rather than the schedule reference so the in-memory map equals the map
        // after a saved-state round trip, which resolves numbers through the
        // registry.
        let genesis_fee_version = FeeVersion::get(platform_version.fee_version.fee_version_number)?;

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
            previous_fee_versions: CachedEpochIndexFeeVersions::from([(
                GENESIS_EPOCH_INDEX,
                genesis_fee_version,
            )]),
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
            self.clone().try_into_platform_versioned(platform_version)?;
        bincode::encode_to_vec(platform_state_for_saving, config).map_err(|e| {
            ProtocolError::PlatformSerializationError(format!(
                "unable to serialize PlatformState: {}",
                e
            ))
            .into()
        })
    }
}

impl PlatformDeserializableFromVersionedStructure for PlatformState {
    fn versioned_deserialize(
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
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "PlatformStateForSaving::try_from_platform_versioned(PlatformState)"
                    .to_string(),
                known_versions: vec![0],
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
                    0 => Ok(PlatformState::from(v1)),
                    version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                        method:
                            "PlatformState::try_from_platform_versioned(PlatformStateForSavingV1)"
                                .to_string(),
                        known_versions: vec![0],
                        received: version,
                    })),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod fee_history {
        use super::*;
        use crate::config::PlatformConfig;
        use dpp::block::epoch::Epoch;
        use dpp::fee::default_costs::{EpochCosts, KnownCostItem};
        use platform_version::version::mocks::fee_doubled_storage_test::{
            TEST_FEE_VERSION_DOUBLED_STORAGE, TEST_FEE_VERSION_NUMBER_DOUBLED_STORAGE,
        };
        use platform_version::version::mocks::v4_test::{
            TEST_PLATFORM_V4, TEST_PROTOCOL_VERSION_4,
        };

        const COST_ITEMS: [KnownCostItem; 5] = [
            KnownCostItem::StorageDiskUsageCreditPerByte,
            KnownCostItem::StorageProcessingCreditPerByte,
            KnownCostItem::StorageLoadCreditPerByte,
            KnownCostItem::NonStorageLoadCreditPerByte,
            KnownCostItem::StorageSeekCost,
        ];

        fn fresh_state(protocol_version: ProtocolVersion) -> PlatformState {
            PlatformState::default_with_protocol_versions(
                protocol_version,
                protocol_version,
                &PlatformConfig::default(),
            )
            .expect("expected a default platform state")
        }

        #[test]
        fn should_record_the_genesis_fee_generation_in_a_fresh_state() {
            let platform_version = PlatformVersion::latest();
            let state = fresh_state(platform_version.protocol_version);

            let expected = FeeVersion::get(platform_version.fee_version.fee_version_number)
                .expect("the genesis schedule's number is registered");
            assert_eq!(
                state.previous_fee_versions,
                CachedEpochIndexFeeVersions::from([(GENESIS_EPOCH_INDEX, expected)])
            );

            for epoch_index in [GENESIS_EPOCH_INDEX, 7] {
                let epoch = Epoch::new(epoch_index).expect("epoch");
                assert_eq!(
                    epoch.active_fee_version(&state.previous_fee_versions),
                    expected,
                    "epoch {epoch_index} must resolve to the genesis generation"
                );
            }
        }

        #[test]
        fn should_record_the_genesis_fee_generation_of_a_mock_version() {
            let state = fresh_state(TEST_PROTOCOL_VERSION_4);

            assert_eq!(
                state.previous_fee_versions,
                CachedEpochIndexFeeVersions::from([(
                    GENESIS_EPOCH_INDEX,
                    &TEST_FEE_VERSION_DOUBLED_STORAGE
                )])
            );
            let epoch = Epoch::new(GENESIS_EPOCH_INDEX).expect("epoch");
            assert_eq!(
                epoch.cost_for_known_cost_item(
                    &state.previous_fee_versions,
                    KnownCostItem::StorageDiskUsageCreditPerByte
                ),
                2 * PlatformVersion::latest()
                    .fee_version
                    .storage
                    .storage_disk_usage_credit_per_byte,
                "genesis-epoch bytes of a chain started at the mock are priced at the doubled rate"
            );
        }

        #[test]
        fn should_round_trip_the_test_fee_generation_number_through_saved_state() {
            let mut state = fresh_state(TEST_PROTOCOL_VERSION_4);
            let first = &PlatformVersion::latest().fee_version;
            state.previous_fee_versions = CachedEpochIndexFeeVersions::from([
                (GENESIS_EPOCH_INDEX, first.as_static()),
                (3, &TEST_FEE_VERSION_DOUBLED_STORAGE),
            ]);

            let bytes = state.serialize_to_bytes().expect("state serializes");
            let restored = PlatformState::versioned_deserialize(&bytes, &TEST_PLATFORM_V4)
                .expect("state with a test fee generation number deserializes");

            let numbers = |state: &PlatformState| {
                state
                    .previous_fee_versions
                    .iter()
                    .map(|(epoch_index, fee_version)| {
                        (*epoch_index, fee_version.fee_version_number)
                    })
                    .collect::<Vec<_>>()
            };
            assert_eq!(numbers(&restored), numbers(&state));
            assert_eq!(
                numbers(&restored),
                vec![
                    (GENESIS_EPOCH_INDEX, first.fee_version_number),
                    (3, TEST_FEE_VERSION_NUMBER_DOUBLED_STORAGE)
                ]
            );

            for epoch_index in 0..=5 {
                let epoch = Epoch::new(epoch_index).expect("epoch");
                for cost_item in COST_ITEMS {
                    assert_eq!(
                        epoch.cost_for_known_cost_item(&restored.previous_fee_versions, cost_item),
                        epoch.cost_for_known_cost_item(&state.previous_fee_versions, cost_item),
                        "epoch {epoch_index} costs must survive the saved-state round trip"
                    );
                }
            }
        }
    }

    mod versioned_deserialize {
        use super::*;
        use crate::test::fixture::platform_state::{
            PLATFORM_STATE_V3_TESTNET, PLATFORM_STATE_V8_DEVNET,
        };
        use platform_version::version::v3::PLATFORM_V3;
        use platform_version::version::v9::PLATFORM_V9;
        use std::ops::Deref;

        #[test]
        fn should_deserialize_state_stored_in_version_0_from_testnet() {
            let serialized_state =
                hex::decode(PLATFORM_STATE_V3_TESTNET.deref()).expect("failed to decode hex");

            PlatformState::versioned_deserialize(&serialized_state, &PLATFORM_V3)
                .expect("failed to deserialize state");
        }

        #[test]
        fn should_deserialize_state_stored_in_version_8_from_devnet() {
            let serialized_state =
                hex::decode(PLATFORM_STATE_V8_DEVNET.deref()).expect("failed to decode hex");

            PlatformState::versioned_deserialize(&serialized_state, &PLATFORM_V9)
                .expect("failed to deserialize state");
        }
    }
}
