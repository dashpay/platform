//! Structure 1 store: the record every block, the large collections as entries.

use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::entry_changes::EntryChanges;
use crate::platform_types::platform_state::platform_state_for_saving::v2::{
    serialize_masternode_entry, serialize_validator_set_entry, PlatformStateForSavingV2,
};
use crate::platform_types::platform_state::platform_state_for_saving::PlatformStateForSaving;
use crate::platform_types::platform_state::PlatformState;
use dpp::bincode::config;
use dpp::dashcore::hashes::Hash;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use drive::drive::platform_state::PlatformStateEntryKind;
use drive::query::TransactionArg;
use std::collections::BTreeSet;

impl<C> Platform<C> {
    /// Stores the state as a structure 1 record plus one aux entry per
    /// masternode and per validator set.
    ///
    /// Without its masternode list and validator sets the record is a few
    /// kilobytes, so it is written every block. Of the entries only the ones
    /// the block changed are written or deleted, which the state tracks; a
    /// state that cannot say what changed has every entry written and every
    /// stale one deleted. Everything is written in the block's transaction, so
    /// a reader never sees a record and entries that disagree.
    pub(super) fn store_platform_state_v1(
        &self,
        state: &PlatformState,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        #[cfg(feature = "testing-config")]
        let should_store = self.config.testing_configs.store_platform_state;
        #[cfg(not(feature = "testing-config"))]
        let should_store = true;

        if should_store {
            self.store_entries(
                PlatformStateEntryKind::Masternodes,
                &state.masternode_changes,
                state
                    .full_masternode_list
                    .keys()
                    .map(|hash| hash.to_byte_array()),
                |key| {
                    state
                        .full_masternode_list
                        .get(key)
                        .map(|masternode| serialize_masternode_entry(masternode, platform_version))
                        .transpose()
                },
                transaction,
                platform_version,
            )?;

            self.store_entries(
                PlatformStateEntryKind::ValidatorSets,
                &state.validator_set_changes,
                state.validator_sets.keys().map(|hash| hash.to_byte_array()),
                |key| {
                    state
                        .validator_sets
                        .get(key)
                        .map(serialize_validator_set_entry)
                        .transpose()
                },
                transaction,
                platform_version,
            )?;

            // Always structure 1 here: this is the store that writes the entries
            // the record depends on.
            let record = PlatformStateForSaving::V2(PlatformStateForSavingV2::from(state));
            let bytes = bincode::encode_to_vec(
                record,
                config::standard().with_big_endian().with_no_limit(),
            )
            .map_err(|e| {
                ProtocolError::PlatformSerializationError(format!(
                    "unable to serialize PlatformState: {e}"
                ))
            })?;
            self.drive
                .store_platform_state_bytes(&bytes, transaction, platform_version)
                .map_err(Error::Drive)?;
        }

        // We need to persist new protocol version as well be able to read block state
        self.drive
            .store_current_protocol_version(platform_version.protocol_version, transaction)?;

        Ok(())
    }

    /// Writes and deletes the entries of one collection according to `changes`.
    ///
    /// `current_keys` are the members the state holds now and `serialize`
    /// produces a member's entry bytes, `None` when it is no longer held.
    fn store_entries<K>(
        &self,
        kind: PlatformStateEntryKind,
        changes: &EntryChanges<K>,
        current_keys: impl Iterator<Item = [u8; 32]>,
        serialize: impl Fn(&K) -> Result<Option<Vec<u8>>, Error>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error>
    where
        K: Ord + Hash<Bytes = [u8; 32]>,
    {
        if changes.rewrite_all {
            let current: BTreeSet<[u8; 32]> = current_keys.collect();
            let stored = self
                .drive
                .fetch_platform_state_entries_bytes(kind, transaction, platform_version)
                .map_err(Error::Drive)?;
            for (key, _) in stored {
                let is_current = <[u8; 32]>::try_from(key.as_slice())
                    .map(|key| current.contains(&key))
                    .unwrap_or(false);
                if !is_current {
                    self.drive
                        .delete_platform_state_entry(kind, &key, transaction, platform_version)
                        .map_err(Error::Drive)?;
                }
            }
            for key in current {
                let member = K::from_byte_array(key);
                if let Some(bytes) = serialize(&member)? {
                    self.drive
                        .store_platform_state_entry_bytes(
                            kind,
                            &key,
                            &bytes,
                            transaction,
                            platform_version,
                        )
                        .map_err(Error::Drive)?;
                }
            }
            return Ok(());
        }

        for key in &changes.removed {
            self.drive
                .delete_platform_state_entry(
                    kind,
                    &key.to_byte_array(),
                    transaction,
                    platform_version,
                )
                .map_err(Error::Drive)?;
        }
        for key in &changes.upserted {
            if let Some(bytes) = serialize(key)? {
                self.drive
                    .store_platform_state_entry_bytes(
                        kind,
                        &key.to_byte_array(),
                        &bytes,
                        transaction,
                        platform_version,
                    )
                    .map_err(Error::Drive)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::config::{PlatformConfig, PlatformTestConfig};
    use crate::platform_types::platform::Platform;
    use crate::platform_types::platform_state::platform_state_for_saving::PlatformStateForSaving;
    use crate::platform_types::platform_state::{PlatformState, PlatformStateV0Methods};
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
    use dpp::bincode::config;
    use dpp::bls_signatures::{Bls12381G2Impl, SecretKey};
    use dpp::core_types::validator::v0::ValidatorV0;
    use dpp::core_types::validator_set::v0::{ValidatorSetV0, ValidatorSetV0Getters};
    use dpp::core_types::validator_set::ValidatorSet;
    use dpp::dashcore::hashes::Hash;
    use dpp::dashcore::PubkeyHash;
    use dpp::dashcore::{ProTxHash, QuorumHash, Txid};
    use dpp::dashcore_rpc::dashcore_rpc_json::{
        DMNState, DMNStateDiff, MasternodeListItem, MasternodeType,
    };
    use dpp::version::PlatformVersion;
    use drive::drive::platform_state::PlatformStateEntryKind;
    use drive::grovedb::Transaction;
    use rand::rngs::StdRng;
    use rand::SeedableRng;
    use std::collections::{BTreeMap, BTreeSet};

    fn platform() -> TempPlatform<MockCoreRPCLike> {
        let platform = TestPlatformBuilder::new()
            .with_config(PlatformConfig {
                testing_configs: PlatformTestConfig {
                    store_platform_state: true,
                    ..PlatformTestConfig::default_minimal_verifications()
                },
                ..Default::default()
            })
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();
        assert_eq!(
            PlatformVersion::latest()
                .drive_abci
                .methods
                .platform_state_storage
                .store_platform_state,
            1,
            "these tests are about the structure 1 store"
        );
        platform
    }

    fn pro_tx_hash(byte: u8) -> ProTxHash {
        ProTxHash::from_byte_array([byte; 32])
    }

    fn masternode(byte: u8, node_type: MasternodeType) -> MasternodeListItem {
        MasternodeListItem {
            node_type,
            pro_tx_hash: pro_tx_hash(byte),
            collateral_hash: Txid::from_byte_array([0u8; 32]),
            collateral_index: 0,
            collateral_address: [0u8; 20],
            operator_reward: 0.0,
            state: DMNState {
                service: "1.2.3.4:1234".parse().expect("socket address"),
                registered_height: 0,
                pose_revived_height: None,
                pose_ban_height: None,
                revocation_reason: 0,
                owner_address: [byte; 20],
                voting_address: [0u8; 20],
                payout_address: [0u8; 20],
                pub_key_operator: vec![0u8; 48],
                operator_payout_address: None,
                platform_node_id: None,
                platform_p2p_port: None,
                platform_http_port: None,
            },
        }
    }

    /// A validator set under quorum hash `[byte; 32]` with one member and a
    /// threshold key derived from `byte`.
    fn validator_set(byte: u8, core_height: u32) -> ValidatorSet {
        let mut rng = StdRng::seed_from_u64(byte as u64);
        let threshold_public_key = SecretKey::<Bls12381G2Impl>::random(&mut rng).public_key();
        let member = ValidatorV0 {
            pro_tx_hash: pro_tx_hash(byte),
            public_key: None,
            node_ip: "1.2.3.4".to_string(),
            node_id: PubkeyHash::from_byte_array([byte; 20]),
            core_port: 9999,
            platform_http_port: 443,
            platform_p2p_port: 26656,
            is_banned: false,
        };
        ValidatorSet::V0(ValidatorSetV0 {
            quorum_hash: QuorumHash::from_byte_array([byte; 32]),
            quorum_index: None,
            core_height,
            members: BTreeMap::from([(pro_tx_hash(byte), member)]),
            threshold_public_key,
        })
    }

    /// Stores `state` the way the block end does and publishes it.
    fn store(
        platform: &TempPlatform<MockCoreRPCLike>,
        mut state: PlatformState,
        transaction: &Transaction,
    ) {
        platform
            .store_platform_state(&state, Some(transaction), PlatformVersion::latest())
            .expect("store platform state");
        state.mark_saved();
        platform.state.store(std::sync::Arc::new(state));
    }

    fn reload(
        platform: &TempPlatform<MockCoreRPCLike>,
        transaction: &Transaction,
    ) -> PlatformState {
        Platform::<MockCoreRPCLike>::fetch_platform_state(
            &platform.drive,
            Some(transaction),
            PlatformVersion::latest(),
        )
        .expect("fetch platform state")
        .expect("a state was stored")
    }

    fn entry_keys(
        platform: &TempPlatform<MockCoreRPCLike>,
        kind: PlatformStateEntryKind,
        transaction: &Transaction,
    ) -> Vec<Vec<u8>> {
        platform
            .drive
            .fetch_platform_state_entries_bytes(kind, Some(transaction), PlatformVersion::latest())
            .expect("fetch entries")
            .into_iter()
            .map(|(key, _)| key)
            .collect()
    }

    fn assert_same_state(reloaded: &PlatformState, expected: &PlatformState) {
        assert_eq!(
            reloaded
                .serialize_standalone_to_bytes()
                .expect("serialize reloaded"),
            expected
                .serialize_standalone_to_bytes()
                .expect("serialize expected"),
            "the reloaded state must equal the stored one field for field"
        );
    }

    fn current_state(platform: &TempPlatform<MockCoreRPCLike>) -> PlatformState {
        platform.state.load().as_ref().clone()
    }

    #[test]
    fn v1_writes_a_structure_1_record_with_one_entry_per_member_and_reads_it_back() {
        let platform = platform();
        let transaction = platform.drive.grove.start_transaction();

        let mut state = current_state(&platform);
        state.insert_masternode(masternode(0x11, MasternodeType::Regular));
        state.insert_masternode(masternode(0x22, MasternodeType::Evo));
        for (byte, core_height) in [(0xAA, 5), (0xBB, 6)] {
            let validator_set = validator_set(byte, core_height);
            state.insert_validator_set(*validator_set.quorum_hash(), validator_set);
        }
        let expected_order: Vec<QuorumHash> = state.validator_sets().keys().copied().collect();
        assert_eq!(expected_order.len(), 2);
        store(&platform, state, &transaction);

        let bytes = platform
            .drive
            .fetch_platform_state_bytes(Some(&transaction), PlatformVersion::latest())
            .expect("fetch record")
            .expect("a record was stored");
        let record: PlatformStateForSaving = bincode::decode_from_slice(
            &bytes,
            config::standard().with_big_endian().with_no_limit(),
        )
        .expect("decode record")
        .0;
        assert!(
            matches!(record, PlatformStateForSaving::V2(_)),
            "the record is structure 1"
        );
        assert!(
            bytes.len() < 64 * 1024,
            "the record no longer carries the collections: {} bytes",
            bytes.len()
        );
        assert_eq!(
            entry_keys(&platform, PlatformStateEntryKind::Masternodes, &transaction),
            vec![[0x11u8; 32].to_vec(), [0x22u8; 32].to_vec()]
        );
        assert_eq!(
            entry_keys(
                &platform,
                PlatformStateEntryKind::ValidatorSets,
                &transaction
            )
            .len(),
            2
        );

        let reloaded = reload(&platform, &transaction);
        assert_same_state(&reloaded, &current_state(&platform));
        assert_eq!(
            reloaded.hpmn_masternode_list().keys().collect::<Vec<_>>(),
            vec![&pro_tx_hash(0x22)],
            "the HPMN list is rebuilt as the Evo subset"
        );
        assert_eq!(
            reloaded
                .validator_sets()
                .keys()
                .copied()
                .collect::<Vec<_>>(),
            expected_order,
            "the validator set order comes back from the record"
        );
        assert!(reloaded.masternode_changes.is_empty());
        assert!(reloaded.validator_set_changes.is_empty());
        assert!(reloaded.heavy_fields_dirty);
    }

    #[test]
    fn v1_writes_and_deletes_only_the_entries_a_block_changed() {
        let platform = platform();
        let transaction = platform.drive.grove.start_transaction();

        let mut state = current_state(&platform);
        state.insert_masternode(masternode(0x11, MasternodeType::Regular));
        state.insert_masternode(masternode(0x22, MasternodeType::Evo));
        for (byte, core_height) in [(0xAA, 5), (0xBB, 6), (0xCC, 7)] {
            let validator_set = validator_set(byte, core_height);
            state.insert_validator_set(*validator_set.quorum_hash(), validator_set);
        }
        store(&platform, state, &transaction);

        // Block 2: one masternode's service changes, one validator set leaves.
        let mut state = current_state(&platform);
        assert!(state.apply_masternode_state_diff(
            &pro_tx_hash(0x11),
            &DMNStateDiff {
                service: Some("5.6.7.8:5678".parse().expect("socket address")),
                registered_height: None,
                last_paid_height: None,
                consecutive_payments: None,
                pose_penalty: None,
                pose_revived_height: None,
                pose_ban_height: None,
                revocation_reason: None,
                owner_address: None,
                voting_address: None,
                payout_address: None,
                pub_key_operator: None,
                operator_payout_address: None,
                platform_node_id: None,
                platform_p2p_port: None,
                platform_http_port: None,
            },
        ));
        let removed_quorum = QuorumHash::from_byte_array([0xAA; 32]);
        assert!(state.remove_validator_set(&removed_quorum).is_some());
        assert_eq!(
            state.masternode_changes.upserted,
            BTreeSet::from([pro_tx_hash(0x11)])
        );
        assert!(state.masternode_changes.removed.is_empty());
        assert!(!state.masternode_changes.rewrite_all);
        assert_eq!(
            state.validator_set_changes.removed,
            BTreeSet::from([removed_quorum])
        );
        assert!(!state.validator_set_changes.rewrite_all);
        let expected_order: Vec<QuorumHash> = state.validator_sets().keys().copied().collect();
        store(&platform, state, &transaction);

        let reloaded = reload(&platform, &transaction);
        assert_same_state(&reloaded, &current_state(&platform));
        assert_eq!(
            reloaded.full_masternode_list()[&pro_tx_hash(0x11)]
                .state
                .service
                .to_string(),
            "5.6.7.8:5678"
        );
        assert_eq!(
            reloaded
                .validator_sets()
                .keys()
                .copied()
                .collect::<Vec<_>>(),
            expected_order,
            "removing a validator set keeps the order of the rest"
        );
        assert_eq!(
            entry_keys(
                &platform,
                PlatformStateEntryKind::ValidatorSets,
                &transaction
            )
            .len(),
            2,
            "the removed validator set's entry is gone"
        );

        // Block 3: a masternode is removed and the validator sets are reordered.
        let mut state = current_state(&platform);
        assert!(state.remove_masternode(&pro_tx_hash(0x22)).is_some());
        state.sort_validator_sets_by(&mut |a, b| a.core_height().cmp(&b.core_height()));
        assert!(
            state.validator_set_changes.is_empty(),
            "reordering rewrites no entry: the order lives in the record"
        );
        let expected_order: Vec<QuorumHash> = state.validator_sets().keys().copied().collect();
        store(&platform, state, &transaction);

        let reloaded = reload(&platform, &transaction);
        assert_same_state(&reloaded, &current_state(&platform));
        assert_eq!(
            entry_keys(&platform, PlatformStateEntryKind::Masternodes, &transaction),
            vec![[0x11u8; 32].to_vec()],
            "the removed masternode's entry is gone"
        );
        assert!(reloaded.hpmn_masternode_list().is_empty());
        assert_eq!(
            reloaded
                .validator_sets()
                .keys()
                .copied()
                .collect::<Vec<_>>(),
            expected_order,
            "the new order comes back from the record"
        );
    }

    /// A change made through a whole-collection accessor rewrites every entry
    /// and deletes the entries of members that are gone.
    #[test]
    fn v1_rewrites_every_entry_when_the_state_cannot_say_what_changed() {
        let platform = platform();
        let transaction = platform.drive.grove.start_transaction();

        let mut state = current_state(&platform);
        state.insert_masternode(masternode(0x11, MasternodeType::Regular));
        state.insert_masternode(masternode(0x22, MasternodeType::Evo));
        store(&platform, state, &transaction);

        let mut state = current_state(&platform);
        state.full_masternode_list_mut().remove(&pro_tx_hash(0x22));
        state.hpmn_masternode_list_mut().remove(&pro_tx_hash(0x22));
        state
            .full_masternode_list_mut()
            .insert(pro_tx_hash(0x33), masternode(0x33, MasternodeType::Regular));
        assert!(state.masternode_changes.rewrite_all);
        store(&platform, state, &transaction);

        assert_eq!(
            entry_keys(&platform, PlatformStateEntryKind::Masternodes, &transaction),
            vec![[0x11u8; 32].to_vec(), [0x33u8; 32].to_vec()],
            "stale entries are deleted and every current member is written"
        );
        let reloaded = reload(&platform, &transaction);
        assert_same_state(&reloaded, &current_state(&platform));
    }

    /// A structure 1 record cannot be read as a record alone; the standalone
    /// snapshot a checkpoint writes can.
    #[test]
    fn a_structure_1_record_alone_is_refused_but_the_standalone_snapshot_reads_back() {
        use dpp::serialization::{
            PlatformDeserializableFromVersionedStructureTrusted, PlatformSerializable,
        };

        let platform = platform();
        let mut state = current_state(&platform);
        state.insert_masternode(masternode(0x11, MasternodeType::Regular));

        let record = state.serialize_to_bytes().expect("serialize record");
        assert!(
            PlatformState::versioned_deserialize_trusted(&record, PlatformVersion::latest())
                .is_err()
        );

        let snapshot = state
            .serialize_standalone_to_bytes()
            .expect("serialize snapshot");
        let restored =
            PlatformState::versioned_deserialize_trusted(&snapshot, PlatformVersion::latest())
                .expect("the snapshot carries the whole state");
        assert_same_state(&restored, &state);
        assert!(
            restored.masternode_changes.rewrite_all,
            "a state restored from a snapshot has no entries on disk"
        );
    }
}
