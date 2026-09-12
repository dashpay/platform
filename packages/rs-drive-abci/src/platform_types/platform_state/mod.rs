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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod versioned_deserialize {
        use super::*;
        use crate::test::fixture::platform_state::{
            PLATFORM_STATE_V3_TESTNET, PLATFORM_STATE_V8_DEVNET,
        };
        use dpp::block::epoch::{Epoch, EpochIndex};
        use dpp::fee::default_costs::{EpochCosts, KnownCostItem};
        use dpp::version::fee::{FeeVersion, FEE_VERSIONS};
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

        fn round_trip(state: &PlatformState) -> PlatformState {
            let bytes = state.serialize_to_bytes().expect("serialize state");
            PlatformState::versioned_deserialize(&bytes, PlatformVersion::latest())
                .expect("deserialize state")
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

        #[test]
        fn should_still_load_legacy_v0_states_as_fee_version_one() {
            // The pre-1.4 format stored whole fee version structs. Only number 1 existed then,
            // so every stored epoch maps to the first registered generation.
            let serialized_state =
                hex::decode(PLATFORM_STATE_V3_TESTNET.deref()).expect("failed to decode hex");
            let state = PlatformState::versioned_deserialize(&serialized_state, &PLATFORM_V3)
                .expect("failed to deserialize state");

            assert_every_entry_is_fee_version_one(&state);
        }

        #[test]
        fn should_resolve_stored_numbers_in_v1_states_to_registered_fee_versions() {
            let serialized_state =
                hex::decode(PLATFORM_STATE_V8_DEVNET.deref()).expect("failed to decode hex");
            let state = PlatformState::versioned_deserialize(&serialized_state, &PLATFORM_V9)
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
        fn should_reject_a_saved_state_that_stores_an_unknown_fee_version_number() {
            let state = latest_state();
            let mut saving = PlatformStateForSavingV1::try_from(state).expect("saving form");
            saving.previous_fee_versions.insert(3, 99);
            let config = config::standard().with_big_endian().with_no_limit();
            let bytes = bincode::encode_to_vec(PlatformStateForSaving::V1(saving), config)
                .expect("encode saving form");

            let error = PlatformState::versioned_deserialize(&bytes, PlatformVersion::latest())
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
