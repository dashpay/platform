#[cfg(test)]
mod tests {
    use crate::addresses_with_balance::AddressesWithBalance;
    use crate::execution::{continue_chain_for_strategy, run_chain_for_strategy, GENESIS_TIME_MS};
    use crate::strategy::{
        ChainExecutionOutcome, ChainExecutionParameters, FailureStrategy, NetworkStrategy,
        StrategyRandomness, UpgradingInfo,
    };
    use dash_platform_macros::stack_size;
    use dpp::block::epoch::Epoch;
    use dpp::block::extended_block_info::v0::ExtendedBlockInfoV0Getters;
    use dpp::block::extended_epoch_info::v0::ExtendedEpochInfoV0Getters;
    use dpp::dashcore::hashes::Hash;
    use dpp::dashcore::Network::Regtest;
    use dpp::dashcore::{BlockHash, ChainLock};
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
    use dpp::data_contracts::SystemDataContract;
    use dpp::version::PlatformVersion;
    use dpp::version::ProtocolVersion;
    use drive::config::DriveConfig;
    use drive::query::proposer_block_count_query::ProposerQueryType;
    use drive_abci::abci::app::FullAbciApplication;
    use drive_abci::config::{
        ChainLockConfig, ExecutionConfig, InstantLockConfig, PlatformConfig, PlatformTestConfig,
        ValidatorSetConfig,
    };
    use drive_abci::logging::LogLevel;
    use drive_abci::platform_types::platform::Platform;
    use drive_abci::platform_types::platform_state::PlatformStateV0Methods;
    use drive_abci::rpc::core::MockCoreRPCLike;
    use drive_abci::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
    use platform_version::version::mocks::v2_test::TEST_PROTOCOL_VERSION_2;
    use platform_version::version::mocks::v3_test::TEST_PROTOCOL_VERSION_3;
    use platform_version::version::INITIAL_PROTOCOL_VERSION;
    use std::collections::BTreeMap;
    use strategy_tests::{IdentityInsertInfo, StartAddresses, StartIdentities, Strategy};

    #[stack_size(4 * 1024 * 1024)]
    #[test]
    #[ignore] // Long-running: runs in nightly CI only
    async fn run_chain_version_upgrade() {
        let platform_version = PlatformVersion::first();
        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
                start_addresses: StartAddresses::default(),
                identity_inserts: IdentityInsertInfo::default(),
                identity_contract_nonce_gaps: None,
                signer: None,
            },
            total_hpmns: 460,
            extra_normal_mns: 0,
            validator_quorum_count: 24,
            chain_lock_quorum_count: 24,
            upgrading_info: Some(UpgradingInfo {
                current_protocol_version: 1,
                proposed_protocol_versions_with_weight: vec![(TEST_PROTOCOL_VERSION_2, 1)],
                upgrade_three_quarters_life: 0.1,
            }),
            proposer_strategy: Default::default(),
            rotate_quorums: false,
            failure_testing: None,
            query_testing: None,
            verify_state_transition_results: false,
            ..Default::default()
        };
        let twenty_minutes_in_ms = 1000 * 60 * 20;
        let mut config = PlatformConfig {
            validator_set: ValidatorSetConfig::default_100_67(),
            chain_lock: ChainLockConfig::default_100_67(),
            instant_lock: InstantLockConfig::default_100_67(),
            execution: ExecutionConfig {
                verify_sum_trees: true,
                epoch_time_length_s: 1576800,
                ..Default::default()
            },
            drive: DriveConfig {
                epochs_per_era: 20,
                ..Default::default()
            },
            block_spacing_ms: twenty_minutes_in_ms,
            testing_configs: PlatformTestConfig::default_minimal_verifications(),
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .with_initial_protocol_version(INITIAL_PROTOCOL_VERSION)
            .build_with_mock_rpc();
        platform
            .core_rpc
            .expect_get_best_chain_lock()
            .returning(move || {
                Ok(ChainLock {
                    block_height: 10,
                    block_hash: BlockHash::from_byte_array([1; 32]),
                    signature: [2; 96].into(),
                })
            });
        let ChainExecutionOutcome {
            abci_app,
            proposers,
            validator_quorums: quorums,
            current_validator_quorum_hash: current_quorum_hash,
            current_proposer_versions,
            end_time_ms,
            identity_nonce_counter,
            identity_contract_nonce_counter,
            instant_lock_quorums,
            ..
        } = run_chain_for_strategy(
            &mut platform,
            1300,
            strategy.clone(),
            config.clone(),
            13,
            &mut None,
            &mut None,
        )
        .await;

        let platform = abci_app.platform;
        let state = platform.state.load();

        {
            let counter = platform.drive.cache.protocol_versions_counter.read();
            platform
                .drive
                .fetch_versions_with_counter(None, &platform_version.drive)
                .expect("expected to get versions");

            assert_eq!(
                state
                    .last_committed_block_info()
                    .as_ref()
                    .unwrap()
                    .basic_info()
                    .epoch
                    .index,
                0
            );
            assert_eq!(state.current_protocol_version_in_consensus(), 1);
            assert_eq!(
                (
                    counter.get(&1).unwrap(),
                    counter.get(&TEST_PROTOCOL_VERSION_2).unwrap()
                ),
                (Some(&11), Some(&435))
            );
            //most nodes were hit (63 were not)
        }

        // we did not yet hit the epoch change
        // let's go a little longer

        let hour_in_ms = 1000 * 60 * 60;
        let block_start = state
            .last_committed_block_info()
            .as_ref()
            .unwrap()
            .basic_info()
            .height
            + 1;

        //speed things up
        config.block_spacing_ms = hour_in_ms;

        let ChainExecutionOutcome {
            abci_app,
            proposers,
            validator_quorums: quorums,
            current_validator_quorum_hash: current_quorum_hash,
            end_time_ms,
            identity_nonce_counter,
            identity_contract_nonce_counter,
            instant_lock_quorums,
            ..
        } = continue_chain_for_strategy(
            abci_app,
            ChainExecutionParameters {
                block_start,
                core_height_start: 1,
                block_count: 200,
                proposers,
                validator_quorums: quorums,
                current_validator_quorum_hash: current_quorum_hash,
                current_proposer_versions: Some(current_proposer_versions.clone()),
                current_identity_nonce_counter: identity_nonce_counter,
                current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                current_votes: BTreeMap::default(),
                start_time_ms: 1681094380000,
                current_time_ms: end_time_ms,
                instant_lock_quorums,
                current_identities: Vec::new(),
                current_addresses_with_balance: AddressesWithBalance::default(),
            },
            strategy.clone(),
            config.clone(),
            StrategyRandomness::SeedEntropy(7),
        )
        .await;

        let state = platform.state.load();
        {
            let counter = &platform.drive.cache.protocol_versions_counter.read();
            assert_eq!(
                state
                    .last_committed_block_info()
                    .as_ref()
                    .unwrap()
                    .basic_info()
                    .epoch
                    .index,
                1
            );
            assert_eq!(state.current_protocol_version_in_consensus(), 1);
            assert_eq!(state.next_epoch_protocol_version(), TEST_PROTOCOL_VERSION_2);
            assert_eq!(counter.get(&1).unwrap(), None); //no one has proposed 1 yet
            assert_eq!(counter.get(&TEST_PROTOCOL_VERSION_2).unwrap(), Some(&179));
        }

        // we locked in
        // let's go a little longer to see activation

        let block_start = state
            .last_committed_block_info()
            .as_ref()
            .unwrap()
            .basic_info()
            .height
            + 1;

        let ChainExecutionOutcome { .. } = continue_chain_for_strategy(
            abci_app,
            ChainExecutionParameters {
                block_start,
                core_height_start: 1,
                block_count: 400,
                proposers,
                validator_quorums: quorums,
                current_validator_quorum_hash: current_quorum_hash,
                current_proposer_versions: Some(current_proposer_versions),
                current_identity_nonce_counter: identity_nonce_counter,
                current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                current_votes: BTreeMap::default(),
                start_time_ms: 1681094380000,
                current_time_ms: end_time_ms,
                instant_lock_quorums,
                current_identities: Vec::new(),
                current_addresses_with_balance: AddressesWithBalance::default(),
            },
            strategy,
            config,
            StrategyRandomness::SeedEntropy(18),
        )
        .await;

        let state = platform.state.load();

        {
            let counter = &platform.drive.cache.protocol_versions_counter.read();
            assert_eq!(
                state
                    .last_committed_block_info()
                    .as_ref()
                    .unwrap()
                    .basic_info()
                    .epoch
                    .index,
                2
            );
            assert_eq!(
                state.current_protocol_version_in_consensus(),
                TEST_PROTOCOL_VERSION_2
            );
            assert_eq!(state.next_epoch_protocol_version(), TEST_PROTOCOL_VERSION_2);
            assert_eq!(counter.get(&1).unwrap(), None); //no one has proposed 1 yet
            assert_eq!(counter.get(&TEST_PROTOCOL_VERSION_2).unwrap(), Some(&147));
        }

        let epoch_proposers_2 = platform
            .drive
            .fetch_epoch_proposers(
                &Epoch::new(2).unwrap(),
                ProposerQueryType::ByRange(None, None),
                None,
                platform_version,
            )
            .expect("expected to get epoch proposers");
        assert_eq!(epoch_proposers_2.len(), 147);

        // Epochs 0 and 1 have been paid out, so their proposers trees
        // were deleted by add_mark_as_paid_operations (DeleteChildren
        // properly cleans up the tree and its contents).
        let epoch_proposers_1 = platform
            .drive
            .fetch_epoch_proposers(
                &Epoch::new(1).unwrap(),
                ProposerQueryType::ByRange(None, None),
                None,
                platform_version,
            )
            .expect("expected to get epoch proposers");
        assert_eq!(epoch_proposers_1.len(), 0);

        let epoch_proposers_0 = platform
            .drive
            .fetch_epoch_proposers(
                &Epoch::new(0).unwrap(),
                ProposerQueryType::ByRange(None, None),
                None,
                platform_version,
            )
            .expect("expected to get epoch proposers");
        assert_eq!(epoch_proposers_0.len(), 0);
    }

    #[stack_size(4 * 1024 * 1024)]
    #[test]
    async fn run_chain_quick_version_upgrade() {
        let platform_version = PlatformVersion::first();
        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
                start_addresses: StartAddresses::default(),
                identity_inserts: IdentityInsertInfo::default(),
                identity_contract_nonce_gaps: None,
                signer: None,
            },
            total_hpmns: 50,
            extra_normal_mns: 0,
            validator_quorum_count: 24,
            chain_lock_quorum_count: 24,
            upgrading_info: Some(UpgradingInfo {
                current_protocol_version: 1,
                proposed_protocol_versions_with_weight: vec![(TEST_PROTOCOL_VERSION_2, 1)],
                upgrade_three_quarters_life: 0.2,
            }),
            proposer_strategy: Default::default(),
            rotate_quorums: false,
            failure_testing: None,
            query_testing: None,
            verify_state_transition_results: false,
            ..Default::default()
        };
        let one_hour_in_s = 60 * 60;
        let thirty_seconds_in_ms = 1000 * 30;
        let config = PlatformConfig {
            validator_set: ValidatorSetConfig {
                quorum_size: 30,
                ..Default::default()
            },
            chain_lock: ChainLockConfig::default_100_67(),
            instant_lock: InstantLockConfig::default_100_67(),
            execution: ExecutionConfig {
                verify_sum_trees: true,
                epoch_time_length_s: one_hour_in_s,
                ..Default::default()
            },
            block_spacing_ms: thirty_seconds_in_ms,
            testing_configs: PlatformTestConfig::default_minimal_verifications(),
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .with_initial_protocol_version(INITIAL_PROTOCOL_VERSION)
            .build_with_mock_rpc();
        platform
            .core_rpc
            .expect_get_best_chain_lock()
            .returning(move || {
                Ok(ChainLock {
                    block_height: 10,
                    block_hash: BlockHash::from_byte_array([1; 32]),
                    signature: [2; 96].into(),
                })
            });
        let ChainExecutionOutcome {
            abci_app,
            proposers,
            validator_quorums: quorums,
            current_validator_quorum_hash: current_quorum_hash,
            current_proposer_versions,
            end_time_ms,
            identity_nonce_counter,
            identity_contract_nonce_counter,
            instant_lock_quorums,
            ..
        } = run_chain_for_strategy(
            &mut platform,
            120,
            strategy.clone(),
            config.clone(),
            13,
            &mut None,
            &mut None,
        )
        .await;

        let platform = abci_app.platform;
        let state = platform.state.load();

        {
            let counter = &platform.drive.cache.protocol_versions_counter.read();
            platform
                .drive
                .fetch_versions_with_counter(None, &platform_version.drive)
                .expect("expected to get versions");

            assert_eq!(
                state
                    .last_committed_block_info()
                    .as_ref()
                    .unwrap()
                    .basic_info()
                    .epoch
                    .index,
                0
            );
            assert_eq!(state.last_committed_block_epoch().index, 0);
            assert_eq!(state.current_protocol_version_in_consensus(), 1);
            assert_eq!(state.next_epoch_protocol_version(), 1);
            assert_eq!(
                (
                    counter.get(&1).unwrap(),
                    counter.get(&TEST_PROTOCOL_VERSION_2).unwrap()
                ),
                (Some(&6), Some(&44))
            );
            //most nodes were hit (63 were not)
        }

        let platform = abci_app.platform;

        let block_start = state
            .last_committed_block_info()
            .as_ref()
            .unwrap()
            .basic_info()
            .height
            + 1;

        let ChainExecutionOutcome {
            abci_app,
            proposers,
            validator_quorums: quorums,
            current_validator_quorum_hash: current_quorum_hash,
            end_time_ms,
            identity_nonce_counter,
            identity_contract_nonce_counter,
            instant_lock_quorums,
            ..
        } = continue_chain_for_strategy(
            abci_app,
            ChainExecutionParameters {
                block_start,
                core_height_start: 1,
                block_count: 1,
                proposers,
                validator_quorums: quorums,
                current_validator_quorum_hash: current_quorum_hash,
                current_proposer_versions: Some(current_proposer_versions.clone()),
                current_identity_nonce_counter: identity_nonce_counter,
                current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                current_votes: BTreeMap::default(),
                start_time_ms: 1681094380000,
                current_time_ms: end_time_ms,
                instant_lock_quorums,
                current_identities: Vec::new(),
                current_addresses_with_balance: AddressesWithBalance::default(),
            },
            strategy.clone(),
            config.clone(),
            StrategyRandomness::SeedEntropy(7),
        )
        .await;

        let state = platform.state.load();
        {
            let counter = &platform.drive.cache.protocol_versions_counter.read();
            assert_eq!(
                state
                    .last_committed_block_info()
                    .as_ref()
                    .unwrap()
                    .basic_info()
                    .epoch
                    .index,
                1
            );
            assert_eq!(state.last_committed_block_epoch().index, 1);
            assert_eq!(state.current_protocol_version_in_consensus(), 1);
            assert_eq!(state.next_epoch_protocol_version(), TEST_PROTOCOL_VERSION_2);
            assert_eq!(counter.get(&1).unwrap(), None); //no one has proposed 1 yet
            assert_eq!(counter.get(&TEST_PROTOCOL_VERSION_2).unwrap(), Some(&1));
        }

        // we locked in
        // let's go 120 blocks more to see activation

        let block_start = state
            .last_committed_block_info()
            .as_ref()
            .unwrap()
            .basic_info()
            .height
            + 1;
        let ChainExecutionOutcome { .. } = continue_chain_for_strategy(
            abci_app,
            ChainExecutionParameters {
                block_start,
                core_height_start: 1,
                block_count: 120,
                proposers,
                validator_quorums: quorums,
                current_validator_quorum_hash: current_quorum_hash,
                current_proposer_versions: Some(current_proposer_versions),
                current_identity_nonce_counter: identity_nonce_counter,
                current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                current_votes: BTreeMap::default(),
                start_time_ms: 1681094380000,
                current_time_ms: end_time_ms,
                instant_lock_quorums,
                current_identities: Vec::new(),
                current_addresses_with_balance: AddressesWithBalance::default(),
            },
            strategy,
            config,
            StrategyRandomness::SeedEntropy(18),
        )
        .await;
        let state = platform.state.load();
        {
            let counter = &platform.drive.cache.protocol_versions_counter.read();
            assert_eq!(
                state
                    .last_committed_block_info()
                    .as_ref()
                    .unwrap()
                    .basic_info()
                    .epoch
                    .index,
                2
            );
            assert_eq!(
                state.current_protocol_version_in_consensus(),
                TEST_PROTOCOL_VERSION_2
            );
            assert_eq!(state.last_committed_block_epoch().index, 2);
            assert_eq!(state.next_epoch_protocol_version(), TEST_PROTOCOL_VERSION_2);
            assert_eq!(counter.get(&1).unwrap(), None); //no one has proposed 1 yet
            assert_eq!(counter.get(&TEST_PROTOCOL_VERSION_2).unwrap(), Some(&1));
        }
    }

    #[stack_size(4 * 1024 * 1024)]
    #[test]
    async fn run_chain_v14_to_v15_migrates_document_history_and_survives_restart() {
        use dpp::data_contract::document_type::random_document::{
            DocumentFieldFillSize, DocumentFieldFillType,
        };
        use dpp::tests::json_document::json_document_to_created_contract;
        use strategy_tests::frequency::Frequency;
        use strategy_tests::operations::{DocumentAction, DocumentOp, Operation, OperationType};
        let old_version = PlatformVersion::get(14).unwrap();
        let created_contract = json_document_to_created_contract(
            "tests/supporting_files/contract/note/note-contract-keep-history.json",
            1,
            false,
            old_version,
        )
        .unwrap();
        let make_operation = |action| Operation {
            op_type: OperationType::Document(DocumentOp {
                contract: created_contract.data_contract().clone(),
                document_type: created_contract
                    .data_contract()
                    .document_type_for_name("note")
                    .unwrap()
                    .to_owned_document_type(),
                action,
            }),
            frequency: Frequency {
                times_per_block_range: 1..2,
                chance_per_block: None,
            },
        };
        let operations = vec![
            make_operation(DocumentAction::DocumentActionInsertRandom(
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
            )),
            make_operation(DocumentAction::DocumentActionReplaceRandom),
        ];
        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![(created_contract, None)],
                operations,
                start_identities: StartIdentities::default(),
                start_addresses: StartAddresses::default(),
                identity_inserts: IdentityInsertInfo {
                    frequency: Frequency {
                        times_per_block_range: 1..2,
                        chance_per_block: None,
                    },
                    ..Default::default()
                },
                identity_contract_nonce_gaps: None,
                signer: None,
            },
            total_hpmns: 50,
            extra_normal_mns: 0,
            validator_quorum_count: 24,
            chain_lock_quorum_count: 24,
            upgrading_info: Some(UpgradingInfo {
                current_protocol_version: 14,
                proposed_protocol_versions_with_weight: vec![(15, 1)],
                upgrade_three_quarters_life: 0.0,
            }),
            proposer_strategy: Default::default(),
            rotate_quorums: false,
            failure_testing: None,
            query_testing: None,
            verify_state_transition_results: false,
            ..Default::default()
        };
        let config = PlatformConfig {
            validator_set: ValidatorSetConfig {
                quorum_size: 30,
                ..Default::default()
            },
            chain_lock: ChainLockConfig::default_100_67(),
            instant_lock: InstantLockConfig::default_100_67(),
            execution: ExecutionConfig {
                verify_sum_trees: true,
                epoch_time_length_s: 60,
                ..Default::default()
            },
            block_spacing_ms: 1_000,
            testing_configs: PlatformTestConfig {
                store_platform_state: true,
                ..PlatformTestConfig::default_minimal_verifications()
            },
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .with_initial_protocol_version(14)
            .build_with_mock_rpc();

        let ChainExecutionOutcome {
            abci_app,
            proposers,
            validator_quorums,
            current_validator_quorum_hash,
            current_proposer_versions,
            end_time_ms,
            identity_nonce_counter,
            identity_contract_nonce_counter,
            instant_lock_quorums,
            mut strategy,
            signer,
            state_transition_results_per_block,
            ..
        } = run_chain_for_strategy(
            &mut platform,
            60,
            strategy.clone(),
            config.clone(),
            14,
            &mut None,
            &mut None,
        )
        .await;

        for results in state_transition_results_per_block.values() {
            for (_, result) in results {
                assert_eq!(result.code, 0);
            }
        }
        let contract = strategy.strategy.start_contracts[0]
            .0
            .data_contract()
            .clone();
        strategy.strategy.operations.retain(|operation| {
            matches!(
                operation.op_type,
                OperationType::Document(DocumentOp {
                    action: DocumentAction::DocumentActionReplaceRandom,
                    ..
                })
            )
        });
        strategy.strategy.identity_inserts = IdentityInsertInfo::default();
        strategy.strategy.signer = Some(signer);
        let primary_path = vec![
            vec![drive::drive::RootTree::DataContractDocuments as u8],
            contract.id().to_vec(),
            vec![1],
            b"note".to_vec(),
            vec![0],
        ];
        let read_entries = |path: Vec<Vec<u8>>| {
            let mut query = drive::grovedb::Query::new();
            query.insert_all();
            abci_app
                .platform
                .drive
                .grove
                .query_raw(
                    &drive::grovedb::PathQuery::new(
                        path,
                        drive::grovedb::SizedQuery::new(query, None, None),
                    ),
                    false,
                    true,
                    true,
                    drive::query::QueryResultType::QueryKeyElementPairResultType,
                    None,
                    &old_version.drive.grove_version,
                )
                .value
                .unwrap()
                .0
                .to_key_elements()
        };
        let (document_id, expected_revisions) = read_entries(primary_path.clone())
            .into_iter()
            .map(|(id, _)| {
                let mut path = primary_path.clone();
                path.push(id.clone());
                let count = read_entries(path).len() - 1;
                (id, count)
            })
            .max_by_key(|(_, count)| *count)
            .expect("strategy must create historical documents");
        assert!(
            expected_revisions > 1,
            "strategy must replace existing documents"
        );

        let state = abci_app.platform.state.load();
        assert_eq!(state.last_committed_block_epoch().index, 0);
        assert_eq!(state.current_protocol_version_in_consensus(), 14);
        assert_eq!(state.next_epoch_protocol_version(), 14);
        let block_start = state
            .last_committed_block_info()
            .as_ref()
            .expect("expected committed block info")
            .basic_info()
            .height
            + 1;
        drop(state);

        let ChainExecutionOutcome {
            abci_app,
            proposers,
            validator_quorums,
            current_validator_quorum_hash,
            end_time_ms,
            identity_nonce_counter,
            identity_contract_nonce_counter,
            instant_lock_quorums,
            state_transition_results_per_block,
            ..
        } = continue_chain_for_strategy(
            abci_app,
            ChainExecutionParameters {
                block_start,
                core_height_start: 1,
                block_count: 1,
                proposers,
                validator_quorums,
                current_validator_quorum_hash,
                current_proposer_versions: Some(current_proposer_versions.clone()),
                current_identity_nonce_counter: identity_nonce_counter,
                current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                current_votes: BTreeMap::default(),
                start_time_ms: 1681094380000,
                current_time_ms: end_time_ms,
                instant_lock_quorums,
                current_identities: Vec::new(),
                current_addresses_with_balance: AddressesWithBalance::default(),
            },
            strategy.clone(),
            config.clone(),
            StrategyRandomness::SeedEntropy(7),
        )
        .await;

        let lock_in_results = state_transition_results_per_block
            .values()
            .next()
            .expect("the lock-in block must execute its replacement");
        assert_eq!(lock_in_results.len(), 1);
        assert_eq!(lock_in_results[0].1.code, 0);

        let read_entries = |path: Vec<Vec<u8>>| {
            let mut query = drive::grovedb::Query::new();
            query.insert_all();
            abci_app
                .platform
                .drive
                .grove
                .query_raw(
                    &drive::grovedb::PathQuery::new(
                        path,
                        drive::grovedb::SizedQuery::new(query, None, None),
                    ),
                    false,
                    true,
                    true,
                    drive::query::QueryResultType::QueryKeyElementPairResultType,
                    None,
                    &old_version.drive.grove_version,
                )
                .value
                .unwrap()
                .0
                .to_key_elements()
        };
        let legacy_revisions_before_activation = read_entries(primary_path.clone())
            .into_iter()
            .map(|(id, _)| {
                let mut path = primary_path.clone();
                path.push(id);
                read_entries(path).len() - 1
            })
            .sum::<usize>();

        let type_path = primary_path[..primary_path.len() - 1].to_vec();
        let history_tree_key = vec![drive::drive::document::paths::DOCUMENT_HISTORY_TREE_KEY];
        let has_history_tree = |drive: &drive::drive::Drive| {
            let type_path: Vec<&[u8]> = type_path.iter().map(Vec::as_slice).collect();
            drive
                .grove
                .has_raw(
                    type_path.as_slice(),
                    &history_tree_key,
                    None,
                    &old_version.drive.grove_version,
                )
                .value
                .unwrap()
        };
        assert!(
            !has_history_tree(&abci_app.platform.drive),
            "lock-in writes must stay in the per-document subtree layout"
        );

        let state = abci_app.platform.state.load();
        assert_eq!(state.last_committed_block_epoch().index, 1);
        assert_eq!(state.current_protocol_version_in_consensus(), 14);
        assert_eq!(state.next_epoch_protocol_version(), 15);
        let block_start = state
            .last_committed_block_info()
            .as_ref()
            .expect("expected committed block info")
            .basic_info()
            .height
            + 1;
        drop(state);

        drop(abci_app);
        let TempPlatform {
            platform: mut platform_before_activation_restart,
            tempdir,
        } = platform;
        let core_rpc = std::mem::take(&mut platform_before_activation_restart.core_rpc);
        drop(platform_before_activation_restart);
        platform = TempPlatform::open_with_tempdir(tempdir, config.clone());
        platform.platform.core_rpc = core_rpc;
        let state = platform.state.load();
        assert_eq!(state.last_committed_block_epoch().index, 1);
        assert_eq!(state.current_protocol_version_in_consensus(), 14);
        assert_eq!(state.next_epoch_protocol_version(), 15);
        drop(state);
        let abci_app = FullAbciApplication::new(&platform.platform);

        let (abci_app, state_transition_results_per_block, successful_pre_activation_replacements) = {
            let before_activation = continue_chain_for_strategy(
                abci_app,
                ChainExecutionParameters {
                    block_start,
                    core_height_start: 1,
                    block_count: 59,
                    proposers,
                    validator_quorums,
                    current_validator_quorum_hash,
                    current_proposer_versions: Some(current_proposer_versions),
                    current_identity_nonce_counter: identity_nonce_counter,
                    current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                    current_votes: BTreeMap::default(),
                    start_time_ms: 1681094380000,
                    current_time_ms: end_time_ms,
                    instant_lock_quorums,
                    current_identities: Vec::new(),
                    current_addresses_with_balance: AddressesWithBalance::default(),
                },
                strategy.clone(),
                config.clone(),
                StrategyRandomness::SeedEntropy(18),
            )
            .await;

            let successful_pre_activation_replacements = before_activation
                .state_transition_results_per_block
                .values()
                .flat_map(|results| results.iter())
                .inspect(|(_, result)| assert_eq!(result.code, 0))
                .count();
            assert_eq!(successful_pre_activation_replacements, 59);
            assert_eq!(
                before_activation
                    .abci_app
                    .platform
                    .state
                    .load()
                    .current_protocol_version_in_consensus(),
                14
            );
            assert!(
                !has_history_tree(&before_activation.abci_app.platform.drive),
                "protocol 14 blocks must not create the history tree"
            );

            let activation_parameters = continuation_parameters(&before_activation, 1);
            let ChainExecutionOutcome { abci_app, .. } = before_activation;
            let mut activation_strategy = strategy;
            activation_strategy.failure_testing = Some(FailureStrategy {
                rounds_before_successful_block: Some(1),
                ..Default::default()
            });
            let ChainExecutionOutcome {
                abci_app,
                state_transition_results_per_block,
                ..
            } = continue_chain_for_strategy(
                abci_app,
                activation_parameters,
                activation_strategy,
                config.clone(),
                StrategyRandomness::SeedEntropy(19),
            )
            .await;
            (
                abci_app,
                state_transition_results_per_block,
                successful_pre_activation_replacements,
            )
        };

        let activation_results = state_transition_results_per_block
            .iter()
            .max_by_key(|(height, _)| *height)
            .map(|(_, results)| results)
            .expect("the activation run must execute replacements");
        assert_eq!(activation_results.len(), 1);
        assert_eq!(activation_results[0].1.code, 0);
        let successful_activation_replacements = state_transition_results_per_block
            .values()
            .flat_map(|results| results.iter())
            .inspect(|(_, result)| assert_eq!(result.code, 0))
            .count();
        assert_eq!(successful_activation_replacements, 1);
        let successful_activation_run_replacements =
            successful_pre_activation_replacements + successful_activation_replacements;

        let state = abci_app.platform.state.load();
        assert_eq!(state.last_committed_block_epoch().index, 2);
        assert_eq!(state.current_protocol_version_in_consensus(), 15);
        assert_eq!(state.next_epoch_protocol_version(), 15);
        drop(state);
        assert!(
            has_history_tree(&abci_app.platform.drive),
            "the activation block must create the per-type history tree"
        );
        let new_version = PlatformVersion::get(15).unwrap();
        let path = drive::drive::document::paths::document_history_path(
            contract.id().as_slice(),
            "note",
            document_id.as_slice(),
        );
        let mut query = drive::grovedb::Query::new();
        query.insert_all();
        let query = drive::grovedb::PathQuery::new(
            path,
            drive::grovedb::SizedQuery::new(query, None, None),
        );
        let (entries, _) = abci_app
            .platform
            .drive
            .grove
            .query_raw(
                &query,
                false,
                true,
                true,
                drive::query::QueryResultType::QueryKeyElementPairResultType,
                None,
                &new_version.drive.grove_version,
            )
            .value
            .unwrap();
        let migrated_revisions = entries.elements.len();
        assert!(migrated_revisions >= expected_revisions);
        assert!(entries
            .to_key_elements()
            .iter()
            .all(|(key, _)| key.len() == 16));

        let read_entries = |path: Vec<Vec<u8>>| {
            let mut query = drive::grovedb::Query::new();
            query.insert_all();
            abci_app
                .platform
                .drive
                .grove
                .query_raw(
                    &drive::grovedb::PathQuery::new(
                        path,
                        drive::grovedb::SizedQuery::new(query, None, None),
                    ),
                    false,
                    true,
                    true,
                    drive::query::QueryResultType::QueryKeyElementPairResultType,
                    None,
                    &new_version.drive.grove_version,
                )
                .value
                .unwrap()
                .0
                .to_key_elements()
        };
        let total_migrated_revisions = read_entries(primary_path.clone())
            .into_iter()
            .map(|(id, _)| {
                read_entries(drive::drive::document::paths::document_history_path(
                    contract.id().as_slice(),
                    "note",
                    id.as_slice(),
                ))
                .len()
            })
            .sum::<usize>();
        assert_eq!(
            total_migrated_revisions,
            legacy_revisions_before_activation + successful_activation_run_replacements,
            "v14 replacements must migrate before the first v15 replacement is written"
        );

        use dpp::block::block_info::BlockInfo;
        use dpp::document::{DocumentV0Getters, DocumentV0Setters};
        use drive::query::document_history_drive_query::{
            DocumentHistoryDriveQuery, DocumentHistoryFilter,
        };
        use drive::util::object_size_info::{
            DocumentAndContractInfo, DocumentInfo, OwnedDocumentInfo,
        };

        let document_type = contract.document_type_for_name("note").unwrap();
        let query = |id, filter, limit| DocumentHistoryDriveQuery {
            contract_id: contract.id().to_buffer(),
            document_type_name: "note".into(),
            document_id: id,
            filter,
            limit,
        };
        let verify = |query: &DocumentHistoryDriveQuery| {
            let page = abci_app
                .platform
                .drive
                .fetch_document_history(query, document_type, None, new_version)
                .unwrap();
            let proof = abci_app
                .platform
                .drive
                .prove_document_history(query, document_type, None, new_version)
                .unwrap();
            let (proved_root, verified) = drive::drive::Drive::verify_document_history(
                query,
                &proof,
                document_type,
                new_version,
            )
            .unwrap();
            assert_eq!(
                proved_root,
                abci_app
                    .platform
                    .drive
                    .grove
                    .root_hash(None, &new_version.drive.grove_version)
                    .value
                    .unwrap()
            );
            assert_eq!(verified, page);
            page
        };

        let current_query = query(
            document_id.clone().try_into().unwrap(),
            DocumentHistoryFilter::Revision(migrated_revisions as u64),
            Some(1),
        );
        let migrated_document = verify(&current_query)
            .entries
            .into_iter()
            .next()
            .expect("the migrated document's current revision must be queryable")
            .document;
        let migrated_revision = migrated_document.revision().unwrap();
        assert_eq!(migrated_revision, migrated_revisions as u64);
        let migrated_page = verify(&query(
            document_id.clone().try_into().unwrap(),
            DocumentHistoryFilter::StartAtRevision(migrated_revision),
            Some(1),
        ));
        assert_eq!(
            migrated_page
                .entries
                .iter()
                .map(|entry| entry.revision)
                .collect::<Vec<_>>(),
            [migrated_revision]
        );

        let post_activation_time = GENESIS_TIME_MS + 1_000_000;
        // The migrated document keeps accepting updates under the v15 layout,
        // written with the storage flags its signed transitions gave it.
        let pointer_flags = abci_app
            .platform
            .drive
            .grove
            .get_raw(
                primary_path
                    .iter()
                    .map(Vec::as_slice)
                    .collect::<Vec<_>>()
                    .as_slice()
                    .into(),
                document_id.as_slice(),
                None,
                &new_version.drive.grove_version,
            )
            .value
            .unwrap()
            .get_flags()
            .clone();
        let storage_flags =
            drive::util::storage_flags::StorageFlags::map_some_element_flags_ref(&pointer_flags)
                .unwrap()
                .map(std::borrow::Cow::Owned);
        let mut updated_document = migrated_document.clone();
        updated_document.set_revision(Some(migrated_revision + 1));
        updated_document.set("message", "updated after activation".into());
        abci_app
            .platform
            .drive
            .update_document_for_contract(
                &updated_document,
                &contract,
                document_type,
                None,
                BlockInfo::default_with_time(post_activation_time),
                true,
                storage_flags,
                None,
                new_version,
                None,
            )
            .expect("a migrated document must remain updatable");
        let updated_page = verify(&query(
            document_id.clone().try_into().unwrap(),
            DocumentHistoryFilter::Revision(migrated_revision + 1),
            Some(1),
        ));
        assert_eq!(updated_page.entries.len(), 1);
        assert_eq!(updated_page.entries[0].time_ms, post_activation_time);
        assert_eq!(updated_page.entries[0].document, updated_document);
        assert_eq!(
            updated_page
                .lifecycle
                .as_ref()
                .map(|l| l.remaining_revisions),
            Some(migrated_revision + 1)
        );

        // Every index reference rewritten by the migration resolves: a proved
        // query through the index returns every current document.
        let indexed_query = drive::query::DriveDocumentQuery::from_sql_expr(
            "select * from note order by tag asc",
            &contract,
            None,
            new_version,
        )
        .unwrap();
        let current_documents = read_entries(primary_path.clone()).len();
        let (index_proof, _) = indexed_query
            .clone()
            .execute_with_proof(&abci_app.platform.drive, None, None, new_version)
            .unwrap();
        let (_, index_verified) = indexed_query
            .verify_proof(&index_proof, new_version)
            .unwrap();
        assert_eq!(index_verified.len(), current_documents);
        assert!(index_verified
            .iter()
            .any(|document| document.id() == updated_document.id()
                && document.revision() == Some(migrated_revision + 1)));

        let fresh_id = (0..=u8::MAX)
            .map(|seed| [seed; 32])
            .find(|candidate| {
                !abci_app
                    .platform
                    .drive
                    .grove
                    .has_raw(
                        primary_path.as_slice(),
                        candidate,
                        None,
                        &new_version.drive.grove_version,
                    )
                    .unwrap()
                    .unwrap()
            })
            .expect("a deterministic unused document id must exist");
        let mut fresh_document = migrated_document.clone();
        fresh_document.set_id(fresh_id.into());
        fresh_document.set_revision(Some(1));
        abci_app
            .platform
            .drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentInfo::DocumentRefInfo((&fresh_document, None)),
                        owner_id: None,
                    },
                    contract: &contract,
                    document_type,
                },
                false,
                BlockInfo::default_with_time(post_activation_time),
                true,
                None,
                new_version,
                None,
            )
            .expect("an existing v14 contract must accept a new v15 document");
        fresh_document.set_revision(Some(2));
        abci_app
            .platform
            .drive
            .update_document_for_contract(
                &fresh_document,
                &contract,
                document_type,
                None,
                BlockInfo::default_with_time(post_activation_time),
                true,
                None,
                None,
                new_version,
                None,
            )
            .expect("a document inserted after activation must remain updatable");
        let fresh_query = query(
            fresh_id,
            DocumentHistoryFilter::StartAtTime(post_activation_time),
            None,
        );
        let fresh_page = verify(&fresh_query);
        assert_eq!(
            fresh_page
                .entries
                .iter()
                .map(|entry| (entry.time_ms, entry.revision))
                .collect::<Vec<_>>(),
            [(post_activation_time, 1), (post_activation_time, 2)]
        );
        let cursor_page = verify(&query(
            fresh_id,
            DocumentHistoryFilter::StartAfter {
                time_ms: post_activation_time,
                revision: 1,
            },
            Some(1),
        ));
        assert_eq!(cursor_page.entries[0].revision, 2);

        let root = abci_app
            .platform
            .drive
            .grove
            .root_hash(None, &new_version.drive.grove_version)
            .value
            .unwrap();
        drop(abci_app);
        let TempPlatform {
            platform: old_platform,
            tempdir,
        } = platform;
        drop(old_platform);
        let restarted = TempPlatform::open_with_tempdir(tempdir, config);
        assert_eq!(
            restarted
                .state
                .load()
                .current_protocol_version_in_consensus(),
            15
        );
        assert_eq!(restarted.state.load().next_epoch_protocol_version(), 15);
        let restarted_page = restarted
            .drive
            .fetch_document_history(&fresh_query, document_type, None, new_version)
            .unwrap();
        let restarted_proof = restarted
            .drive
            .prove_document_history(&fresh_query, document_type, None, new_version)
            .unwrap();
        let (_, restarted_verified) = drive::drive::Drive::verify_document_history(
            &fresh_query,
            &restarted_proof,
            document_type,
            new_version,
        )
        .unwrap();
        assert_eq!(restarted_verified, restarted_page);
        assert_eq!(restarted_page.entries.len(), 2);
        assert_eq!(
            restarted
                .drive
                .grove
                .root_hash(None, &new_version.drive.grove_version)
                .value
                .unwrap(),
            root
        );

        // The lifecycle works over migrated storage, not only over history
        // written after activation. A contract registered before protocol 14
        // cannot declare `canBeErased` — the grammar that admits the keyword
        // did not exist then — so this exercises the storage layer directly:
        // whether a delete and an erase read and rewrite revision keys the
        // migration produced.
        use drive::drive::document::lifecycle::DocumentLifecycleState;
        use drive::util::batch::{DocumentOperationType, DriveOperation};
        use drive::util::object_size_info::{DataContractInfo, DocumentTypeInfo};

        let document_id = dpp::identifier::Identifier::from_bytes(&document_id).unwrap();
        let document_type = contract.document_type_for_name("note").unwrap();
        let contract_info = || DataContractInfo::BorrowedDataContract(&contract);
        let document_type_info = || DocumentTypeInfo::DocumentTypeName("note".to_string());
        let apply = |operation: DriveOperation, time_ms: u64| {
            restarted
                .drive
                .apply_drive_operations(
                    vec![operation],
                    true,
                    &dpp::block::block_info::BlockInfo::default_with_time(time_ms),
                    None,
                    new_version,
                    None,
                )
                .expect("expected to apply the operation")
        };
        let lifecycle = || {
            restarted
                .drive
                .fetch_document_lifecycle(
                    &contract,
                    document_type,
                    document_id,
                    None,
                    None,
                    new_version,
                )
                .expect("expected to read the lifecycle")
                .0
        };

        apply(
            DriveOperation::DocumentOperation(DocumentOperationType::DeleteDocument {
                document_id,
                deleter_id: Some(dpp::identifier::Identifier::new([3u8; 32])),
                contract_info: contract_info(),
                document_type_info: document_type_info(),
            }),
            1_681_094_400_000,
        );
        let state = lifecycle();
        assert!(
            matches!(state, DocumentLifecycleState::Deleted(_)),
            "a migrated document must be deletable, got {state:?}"
        );

        apply(
            DriveOperation::DocumentOperation(DocumentOperationType::EraseDocument {
                document_id,
                contract_info: contract_info(),
                document_type_info: document_type_info(),
            }),
            1_681_094_500_000,
        );
        let state = lifecycle();
        assert!(
            matches!(state, DocumentLifecycleState::Absent),
            "erasing every migrated revision must free the id, got {state:?}"
        );
    }

    #[stack_size(4 * 1024 * 1024)]
    #[test]
    async fn run_chain_v12_to_v13_locks_in_before_activation() {
        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
                start_addresses: StartAddresses::default(),
                identity_inserts: IdentityInsertInfo::default(),
                identity_contract_nonce_gaps: None,
                signer: None,
            },
            total_hpmns: 50,
            extra_normal_mns: 0,
            validator_quorum_count: 24,
            chain_lock_quorum_count: 24,
            upgrading_info: Some(UpgradingInfo {
                current_protocol_version: 12,
                proposed_protocol_versions_with_weight: vec![(13, 1)],
                upgrade_three_quarters_life: 0.0,
            }),
            proposer_strategy: Default::default(),
            rotate_quorums: false,
            failure_testing: None,
            query_testing: None,
            verify_state_transition_results: false,
            ..Default::default()
        };
        let config = PlatformConfig {
            validator_set: ValidatorSetConfig {
                quorum_size: 30,
                ..Default::default()
            },
            chain_lock: ChainLockConfig::default_100_67(),
            instant_lock: InstantLockConfig::default_100_67(),
            execution: ExecutionConfig {
                verify_sum_trees: true,
                epoch_time_length_s: 60,
                ..Default::default()
            },
            block_spacing_ms: 1_000,
            testing_configs: PlatformTestConfig {
                store_platform_state: true,
                ..PlatformTestConfig::default_minimal_verifications()
            },
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .with_initial_protocol_version(12)
            .build_with_mock_rpc();

        let ChainExecutionOutcome {
            abci_app,
            proposers,
            validator_quorums,
            current_validator_quorum_hash,
            current_proposer_versions,
            end_time_ms,
            identity_nonce_counter,
            identity_contract_nonce_counter,
            instant_lock_quorums,
            ..
        } = run_chain_for_strategy(
            &mut platform,
            60,
            strategy.clone(),
            config.clone(),
            13,
            &mut None,
            &mut None,
        )
        .await;

        let state = abci_app.platform.state.load();
        assert_eq!(state.last_committed_block_epoch().index, 0);
        assert_eq!(state.current_protocol_version_in_consensus(), 12);
        assert_eq!(state.next_epoch_protocol_version(), 12);
        let block_start = state
            .last_committed_block_info()
            .as_ref()
            .expect("expected committed block info")
            .basic_info()
            .height
            + 1;
        drop(state);

        let ChainExecutionOutcome {
            abci_app,
            proposers,
            validator_quorums,
            current_validator_quorum_hash,
            end_time_ms,
            identity_nonce_counter,
            identity_contract_nonce_counter,
            instant_lock_quorums,
            ..
        } = continue_chain_for_strategy(
            abci_app,
            ChainExecutionParameters {
                block_start,
                core_height_start: 1,
                block_count: 1,
                proposers,
                validator_quorums,
                current_validator_quorum_hash,
                current_proposer_versions: Some(current_proposer_versions.clone()),
                current_identity_nonce_counter: identity_nonce_counter,
                current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                current_votes: BTreeMap::default(),
                start_time_ms: 1681094380000,
                current_time_ms: end_time_ms,
                instant_lock_quorums,
                current_identities: Vec::new(),
                current_addresses_with_balance: AddressesWithBalance::default(),
            },
            strategy.clone(),
            config.clone(),
            StrategyRandomness::SeedEntropy(7),
        )
        .await;

        let state = abci_app.platform.state.load();
        assert_eq!(state.last_committed_block_epoch().index, 1);
        assert_eq!(state.current_protocol_version_in_consensus(), 12);
        assert_eq!(state.next_epoch_protocol_version(), 13);
        let block_start = state
            .last_committed_block_info()
            .as_ref()
            .expect("expected committed block info")
            .basic_info()
            .height
            + 1;
        drop(state);

        let platform_version_12 = PlatformVersion::get(12).expect("platform version 12");
        let persisted_dpns_v12 = abci_app
            .platform
            .drive
            .fetch_contract(
                SystemDataContract::DPNS.id().to_buffer(),
                None,
                None,
                None,
                platform_version_12,
            )
            .value
            .expect("fetch persisted DPNS before activation")
            .expect("DPNS must be persisted before activation");
        let persisted_domain_v12 = persisted_dpns_v12
            .contract
            .document_type_for_name("domain")
            .expect("DPNS must contain its domain document type");
        assert!(!persisted_domain_v12.documents_keep_transfer_history());
        assert!(!persisted_domain_v12.documents_keep_purchase_history());
        assert!(!persisted_domain_v12.documents_keep_pricing_history());
        assert!(abci_app
            .platform
            .drive
            .fetch_contract(
                SystemDataContract::DocumentHistory.id().to_buffer(),
                None,
                None,
                None,
                platform_version_12,
            )
            .value
            .expect("query persisted Document History before activation")
            .is_none());

        drop(abci_app);
        let TempPlatform {
            platform: mut platform_before_activation_restart,
            tempdir,
        } = platform;
        let core_rpc = std::mem::take(&mut platform_before_activation_restart.core_rpc);
        drop(platform_before_activation_restart);
        platform = TempPlatform::open_with_tempdir(tempdir, config.clone());
        platform.platform.core_rpc = core_rpc;
        let state = platform.state.load();
        assert_eq!(state.last_committed_block_epoch().index, 1);
        assert_eq!(state.current_protocol_version_in_consensus(), 12);
        assert_eq!(state.next_epoch_protocol_version(), 13);
        drop(state);
        let abci_app = FullAbciApplication::new(&platform.platform);

        let ChainExecutionOutcome { abci_app, .. } = continue_chain_for_strategy(
            abci_app,
            ChainExecutionParameters {
                block_start,
                core_height_start: 1,
                block_count: 60,
                proposers,
                validator_quorums,
                current_validator_quorum_hash,
                current_proposer_versions: Some(current_proposer_versions),
                current_identity_nonce_counter: identity_nonce_counter,
                current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                current_votes: BTreeMap::default(),
                start_time_ms: 1681094380000,
                current_time_ms: end_time_ms,
                instant_lock_quorums,
                current_identities: Vec::new(),
                current_addresses_with_balance: AddressesWithBalance::default(),
            },
            strategy,
            config.clone(),
            StrategyRandomness::SeedEntropy(18),
        )
        .await;

        let state = abci_app.platform.state.load();
        assert_eq!(state.last_committed_block_epoch().index, 2);
        assert_eq!(state.current_protocol_version_in_consensus(), 13);
        assert_eq!(state.next_epoch_protocol_version(), 13);
        drop(state);
        let platform_version_13 = PlatformVersion::get(13).expect("platform version 13");
        let persisted_dpns_v13 = abci_app
            .platform
            .drive
            .fetch_contract(
                SystemDataContract::DPNS.id().to_buffer(),
                None,
                None,
                None,
                platform_version_13,
            )
            .value
            .expect("fetch persisted DPNS after activation")
            .expect("DPNS must remain persisted after activation");
        let persisted_domain_v13 = persisted_dpns_v13
            .contract
            .document_type_for_name("domain")
            .expect("DPNS must contain its domain document type");
        assert!(persisted_domain_v13.documents_keep_transfer_history());
        assert!(persisted_domain_v13.documents_keep_purchase_history());
        assert!(persisted_domain_v13.documents_keep_pricing_history());
        assert!(abci_app
            .platform
            .drive
            .fetch_contract(
                SystemDataContract::DocumentHistory.id().to_buffer(),
                None,
                None,
                None,
                platform_version_13,
            )
            .value
            .expect("fetch persisted Document History after activation")
            .is_some());
        let dpns_v13 = abci_app
            .platform
            .drive
            .cache
            .system_data_contracts
            .find_by_id(SystemDataContract::DPNS.id(), platform_version_13)
            .expect("expected the DPNS lookup to succeed")
            .expect("the public activation path must cache DPNS v2");
        let domain = dpns_v13
            .document_type_for_name("domain")
            .expect("DPNS must contain its domain document type");
        assert!(domain.documents_keep_transfer_history());
        assert!(domain.documents_keep_purchase_history());
        assert!(domain.documents_keep_pricing_history());
        assert!(abci_app
            .platform
            .drive
            .cache
            .system_data_contracts
            .find_by_id(
                SystemDataContract::DocumentHistory.id(),
                platform_version_13
            )
            .expect("expected the document history lookup to succeed")
            .is_some());
        drop(abci_app);

        let TempPlatform {
            platform: platform_before_restart,
            tempdir,
        } = platform;
        drop(platform_before_restart);

        let reopened_platform = TempPlatform::open_with_tempdir(tempdir, config);
        let state = reopened_platform.state.load();
        assert_eq!(state.last_committed_block_epoch().index, 2);
        assert_eq!(state.current_protocol_version_in_consensus(), 13);
        assert_eq!(state.next_epoch_protocol_version(), 13);
        drop(state);
        let reopened_persisted_dpns_v13 = reopened_platform
            .drive
            .fetch_contract(
                SystemDataContract::DPNS.id().to_buffer(),
                None,
                None,
                None,
                platform_version_13,
            )
            .value
            .expect("fetch persisted DPNS after restart")
            .expect("DPNS must remain persisted after restart");
        let reopened_persisted_domain_v13 = reopened_persisted_dpns_v13
            .contract
            .document_type_for_name("domain")
            .expect("DPNS must contain its domain document type");
        assert!(reopened_persisted_domain_v13.documents_keep_transfer_history());
        assert!(reopened_persisted_domain_v13.documents_keep_purchase_history());
        assert!(reopened_persisted_domain_v13.documents_keep_pricing_history());
        assert!(reopened_platform
            .drive
            .fetch_contract(
                SystemDataContract::DocumentHistory.id().to_buffer(),
                None,
                None,
                None,
                platform_version_13,
            )
            .value
            .expect("fetch persisted Document History after restart")
            .is_some());
        let reopened_dpns_v13 = reopened_platform
            .drive
            .cache
            .system_data_contracts
            .find_by_id(SystemDataContract::DPNS.id(), platform_version_13)
            .expect("expected the DPNS lookup to succeed")
            .expect("restart must reconstruct DPNS v2");
        let reopened_domain = reopened_dpns_v13
            .document_type_for_name("domain")
            .expect("DPNS must contain its domain document type");
        assert!(reopened_domain.documents_keep_transfer_history());
        assert!(reopened_domain.documents_keep_purchase_history());
        assert!(reopened_domain.documents_keep_pricing_history());
        let reopened_dpns_v12 = reopened_platform
            .drive
            .cache
            .system_data_contracts
            .find_by_id(SystemDataContract::DPNS.id(), platform_version_12)
            .expect("expected the DPNS lookup to succeed")
            .expect("restart must materialize explicitly requested DPNS v1");
        let reopened_domain_v12 = reopened_dpns_v12
            .document_type_for_name("domain")
            .expect("DPNS must contain its domain document type");
        assert!(!reopened_domain_v12.documents_keep_transfer_history());
        assert!(!reopened_domain_v12.documents_keep_purchase_history());
        assert!(!reopened_domain_v12.documents_keep_pricing_history());
        assert!(reopened_platform
            .drive
            .cache
            .system_data_contracts
            .find_by_id(
                SystemDataContract::DocumentHistory.id(),
                platform_version_13
            )
            .expect("expected the document history lookup to succeed")
            .is_some());
    }

    /// What a node has committed about the protocol upgrade, captured to compare a node that
    /// never restarted with one whose Drive was closed and reopened.
    #[derive(Debug, PartialEq)]
    struct CommittedUpgradeState {
        height: u64,
        epoch_index: u16,
        current_protocol_version: ProtocolVersion,
        next_epoch_protocol_version: ProtocolVersion,
        app_hash: Option<[u8; 32]>,
    }

    fn committed_upgrade_state(platform: &Platform<MockCoreRPCLike>) -> CommittedUpgradeState {
        let state = platform.state.load();
        CommittedUpgradeState {
            height: state.last_committed_block_height(),
            epoch_index: state.last_committed_block_epoch().index,
            current_protocol_version: state.current_protocol_version_in_consensus(),
            next_epoch_protocol_version: state.next_epoch_protocol_version(),
            app_hash: state.last_committed_block_app_hash(),
        }
    }

    /// Asserts that the votes persisted in Drive for `protocol_version` reach the count the
    /// epoch tally requires, so the next epoch boundary must lock that version in.
    fn assert_persisted_votes_reach_upgrade_threshold(
        platform: &Platform<MockCoreRPCLike>,
        protocol_version: ProtocolVersion,
    ) {
        let state = platform.state.load();
        let platform_version = state
            .current_platform_version()
            .expect("expected the current platform version");
        let required_votes = 1 + state.hpmn_active_list_len() as u64
            * platform_version
                .drive_abci
                .methods
                .protocol_upgrade
                .protocol_version_upgrade_percentage_needed
            / 100;
        let persisted_votes = platform
            .drive
            .fetch_versions_with_counter(None, &platform_version.drive)
            .expect("expected to fetch the persisted protocol version votes");
        let votes = persisted_votes
            .get(&protocol_version)
            .copied()
            .unwrap_or_default();
        assert!(
            votes >= required_votes,
            "expected at least {required_votes} persisted votes for protocol version {protocol_version}, found {votes}"
        );
    }

    /// Parameters that continue the chain of `outcome` for `block_count` more blocks with the
    /// same proposers, quorums and upgrade schedule.
    fn continuation_parameters(
        outcome: &ChainExecutionOutcome,
        block_count: u64,
    ) -> ChainExecutionParameters {
        let block_start = outcome
            .abci_app
            .platform
            .state
            .load()
            .last_committed_block_height()
            + 1;
        ChainExecutionParameters {
            block_start,
            core_height_start: 1,
            block_count,
            proposers: outcome.proposers.clone(),
            validator_quorums: outcome.validator_quorums.clone(),
            current_validator_quorum_hash: outcome.current_validator_quorum_hash,
            current_proposer_versions: Some(outcome.current_proposer_versions.clone()),
            current_identity_nonce_counter: outcome.identity_nonce_counter.clone(),
            current_identity_contract_nonce_counter: outcome
                .identity_contract_nonce_counter
                .clone(),
            current_votes: BTreeMap::default(),
            start_time_ms: GENESIS_TIME_MS,
            current_time_ms: outcome.end_time_ms,
            instant_lock_quorums: outcome.instant_lock_quorums.clone(),
            current_identities: Vec::new(),
            current_addresses_with_balance: AddressesWithBalance::default(),
        }
    }

    /// A node whose first block after a restart is an epoch boundary must tally the same
    /// persisted upgrade votes as the peers that never restarted.
    ///
    /// The epoch tally reads the protocol version counter cache directly, and it runs before
    /// the block records its first vote, which is what loaded the cache from Drive. A reopened
    /// Drive therefore tallied an empty cache: it kept `next = current` while the warm peers
    /// locked in the new version. The epoch tree records the next version, so the app hashes
    /// diverged on the lock-in block itself, and one epoch later the warm peers activated the
    /// new version while the reopened node did not.
    #[stack_size(4 * 1024 * 1024)]
    #[test]
    async fn run_chain_reopened_drive_at_epoch_boundary_locks_in_the_same_version_as_a_warm_node() {
        const CURRENT_PROTOCOL_VERSION: ProtocolVersion = 13;
        const NEXT_PROTOCOL_VERSION: ProtocolVersion = 14;
        const BLOCKS_PER_EPOCH: u64 = 60;
        const CHAIN_SEED: u64 = 13;
        const LOCK_IN_SEED: u64 = 7;
        const ACTIVATION_SEED: u64 = 18;

        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
                start_addresses: StartAddresses::default(),
                identity_inserts: IdentityInsertInfo::default(),
                identity_contract_nonce_gaps: None,
                signer: None,
            },
            total_hpmns: 50,
            extra_normal_mns: 0,
            validator_quorum_count: 24,
            chain_lock_quorum_count: 24,
            upgrading_info: Some(UpgradingInfo {
                current_protocol_version: CURRENT_PROTOCOL_VERSION,
                proposed_protocol_versions_with_weight: vec![(NEXT_PROTOCOL_VERSION, 1)],
                upgrade_three_quarters_life: 0.0,
            }),
            proposer_strategy: Default::default(),
            rotate_quorums: false,
            failure_testing: None,
            query_testing: None,
            verify_state_transition_results: false,
            ..Default::default()
        };
        let config = PlatformConfig {
            validator_set: ValidatorSetConfig {
                quorum_size: 30,
                ..Default::default()
            },
            chain_lock: ChainLockConfig::default_100_67(),
            instant_lock: InstantLockConfig::default_100_67(),
            execution: ExecutionConfig {
                verify_sum_trees: true,
                epoch_time_length_s: BLOCKS_PER_EPOCH,
                ..Default::default()
            },
            block_spacing_ms: 1_000,
            testing_configs: PlatformTestConfig {
                store_platform_state: true,
                ..PlatformTestConfig::default_minimal_verifications()
            },
            ..Default::default()
        };

        // The warm node runs the whole timeline without restarting: epoch 0 collects the votes,
        // the first block of epoch 1 locks the next version in and the first block of epoch 2
        // activates it.
        let (warm_before_lock_in, warm_at_lock_in, warm_at_activation) = {
            let mut warm_platform = TestPlatformBuilder::new()
                .with_config(config.clone())
                .with_initial_protocol_version(CURRENT_PROTOCOL_VERSION)
                .build_with_mock_rpc();
            let warm_epoch_zero = run_chain_for_strategy(
                &mut warm_platform,
                BLOCKS_PER_EPOCH,
                strategy.clone(),
                config.clone(),
                CHAIN_SEED,
                &mut None,
                &mut None,
            )
            .await;
            let warm_before_lock_in = committed_upgrade_state(warm_epoch_zero.abci_app.platform);
            assert_eq!(warm_before_lock_in.epoch_index, 0);
            assert_eq!(
                warm_before_lock_in.current_protocol_version,
                CURRENT_PROTOCOL_VERSION
            );
            assert_eq!(
                warm_before_lock_in.next_epoch_protocol_version,
                CURRENT_PROTOCOL_VERSION
            );
            assert_persisted_votes_reach_upgrade_threshold(
                warm_epoch_zero.abci_app.platform,
                NEXT_PROTOCOL_VERSION,
            );

            let lock_in_parameters = continuation_parameters(&warm_epoch_zero, 1);
            let ChainExecutionOutcome { abci_app, .. } = warm_epoch_zero;
            let warm_lock_in = continue_chain_for_strategy(
                abci_app,
                lock_in_parameters,
                strategy.clone(),
                config.clone(),
                StrategyRandomness::SeedEntropy(LOCK_IN_SEED),
            )
            .await;
            let warm_at_lock_in = committed_upgrade_state(warm_lock_in.abci_app.platform);
            assert_eq!(warm_at_lock_in.epoch_index, 1);
            assert_eq!(
                warm_at_lock_in.current_protocol_version,
                CURRENT_PROTOCOL_VERSION
            );
            assert_eq!(
                warm_at_lock_in.next_epoch_protocol_version,
                NEXT_PROTOCOL_VERSION
            );

            let activation_parameters = continuation_parameters(&warm_lock_in, BLOCKS_PER_EPOCH);
            let ChainExecutionOutcome { abci_app, .. } = warm_lock_in;
            let warm_activation = continue_chain_for_strategy(
                abci_app,
                activation_parameters,
                strategy.clone(),
                config.clone(),
                StrategyRandomness::SeedEntropy(ACTIVATION_SEED),
            )
            .await;
            let warm_at_activation = committed_upgrade_state(warm_activation.abci_app.platform);
            assert_eq!(warm_at_activation.epoch_index, 2);
            assert_eq!(
                warm_at_activation.current_protocol_version,
                NEXT_PROTOCOL_VERSION
            );
            assert_eq!(
                warm_at_activation.next_epoch_protocol_version,
                NEXT_PROTOCOL_VERSION
            );
            (warm_before_lock_in, warm_at_lock_in, warm_at_activation)
        };

        // The restarted node follows the identical chain to the end of epoch 0. Its Drive is
        // then closed and reopened, so the epoch boundary is the first block it processes after
        // the restart.
        let mut restarted_platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .with_initial_protocol_version(CURRENT_PROTOCOL_VERSION)
            .build_with_mock_rpc();
        let restarted_epoch_zero = run_chain_for_strategy(
            &mut restarted_platform,
            BLOCKS_PER_EPOCH,
            strategy.clone(),
            config.clone(),
            CHAIN_SEED,
            &mut None,
            &mut None,
        )
        .await;
        assert_eq!(
            committed_upgrade_state(restarted_epoch_zero.abci_app.platform),
            warm_before_lock_in,
            "both nodes must have committed the same state before the restart"
        );
        let lock_in_parameters = continuation_parameters(&restarted_epoch_zero, 1);
        drop(restarted_epoch_zero);

        let TempPlatform {
            platform: mut platform_before_restart,
            tempdir,
        } = restarted_platform;
        let core_rpc = std::mem::take(&mut platform_before_restart.core_rpc);
        drop(platform_before_restart);
        let mut restarted_platform = TempPlatform::open_with_tempdir(tempdir, config.clone());
        restarted_platform.platform.core_rpc = core_rpc;
        assert_eq!(
            committed_upgrade_state(&restarted_platform.platform),
            warm_before_lock_in,
            "reopening must restore the committed state"
        );

        let restarted_lock_in = continue_chain_for_strategy(
            FullAbciApplication::new(&restarted_platform.platform),
            lock_in_parameters,
            strategy.clone(),
            config.clone(),
            StrategyRandomness::SeedEntropy(LOCK_IN_SEED),
        )
        .await;
        assert_eq!(
            committed_upgrade_state(restarted_lock_in.abci_app.platform),
            warm_at_lock_in,
            "the reopened node must lock in the same next protocol version and app hash as the warm node"
        );

        let activation_parameters = continuation_parameters(&restarted_lock_in, BLOCKS_PER_EPOCH);
        let ChainExecutionOutcome { abci_app, .. } = restarted_lock_in;
        let restarted_activation = continue_chain_for_strategy(
            abci_app,
            activation_parameters,
            strategy,
            config,
            StrategyRandomness::SeedEntropy(ACTIVATION_SEED),
        )
        .await;
        assert_eq!(
            committed_upgrade_state(restarted_activation.abci_app.platform),
            warm_at_activation,
            "the reopened node must activate the same protocol version with the same app hash as the warm node"
        );
    }

    #[stack_size(4 * 1024 * 1024)]
    #[test]
    #[ignore] // Long-running: runs in nightly CI only
    async fn run_chain_version_upgrade_slow_upgrade() {
        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
                start_addresses: StartAddresses::default(),
                identity_inserts: IdentityInsertInfo::default(),
                identity_contract_nonce_gaps: None,
                signer: None,
            },
            total_hpmns: 120,
            extra_normal_mns: 0,
            validator_quorum_count: 200,
            upgrading_info: Some(UpgradingInfo {
                current_protocol_version: 1,
                proposed_protocol_versions_with_weight: vec![(TEST_PROTOCOL_VERSION_2, 1)],
                upgrade_three_quarters_life: 5.0, //it will take many epochs before we get enough nodes
            }),
            proposer_strategy: Default::default(),
            rotate_quorums: false,
            failure_testing: None,
            query_testing: None,
            verify_state_transition_results: false,
            ..Default::default()
        };
        let hour_in_ms = 1000 * 60 * 60;
        let config = PlatformConfig {
            network: Regtest,
            validator_set: ValidatorSetConfig {
                quorum_size: 40,
                ..Default::default()
            },
            chain_lock: ChainLockConfig::default_100_67(),
            instant_lock: InstantLockConfig::default_100_67(),
            execution: ExecutionConfig {
                verify_sum_trees: false, //faster without this
                epoch_time_length_s: 1576800,
                ..Default::default()
            },
            drive: DriveConfig {
                epochs_per_era: 20,
                ..Default::default()
            },
            block_spacing_ms: hour_in_ms,
            testing_configs: PlatformTestConfig::default_minimal_verifications(),
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .with_initial_protocol_version(INITIAL_PROTOCOL_VERSION)
            .build_with_mock_rpc();
        platform
            .core_rpc
            .expect_get_best_chain_lock()
            .returning(move || {
                Ok(ChainLock {
                    block_height: 10,
                    block_hash: BlockHash::from_byte_array([1; 32]),
                    signature: [2; 96].into(),
                })
            });

        let ChainExecutionOutcome {
            abci_app,
            proposers,
            validator_quorums: quorums,
            current_validator_quorum_hash: current_quorum_hash,
            current_proposer_versions,
            end_time_ms,
            identity_nonce_counter,
            identity_contract_nonce_counter,
            instant_lock_quorums,
            ..
        } = run_chain_for_strategy(
            &mut platform,
            2500,
            strategy.clone(),
            config.clone(),
            16,
            &mut None,
            &mut None,
        )
        .await;
        let platform = abci_app.platform;
        let state = platform.state.load();
        {
            assert_eq!(
                state
                    .last_committed_block_info()
                    .as_ref()
                    .unwrap()
                    .basic_info()
                    .epoch
                    .index,
                5
            );
            assert_eq!(state.current_protocol_version_in_consensus(), 1);
            assert_eq!(state.next_epoch_protocol_version(), 1);
            let counter = &platform.drive.cache.protocol_versions_counter.read();
            assert_eq!(
                (
                    counter.get(&1).unwrap(),
                    counter.get(&TEST_PROTOCOL_VERSION_2).unwrap()
                ),
                (Some(&39), Some(&78))
            );
        }

        // we did not yet hit the required threshold to upgrade
        // let's go a little longer

        let platform = abci_app.platform;
        let block_start = state
            .last_committed_block_info()
            .as_ref()
            .unwrap()
            .basic_info()
            .height
            + 1;
        let ChainExecutionOutcome {
            abci_app,
            proposers,
            validator_quorums: quorums,
            current_validator_quorum_hash: current_quorum_hash,
            end_time_ms,
            identity_nonce_counter,
            identity_contract_nonce_counter,
            instant_lock_quorums,
            ..
        } = continue_chain_for_strategy(
            abci_app,
            ChainExecutionParameters {
                block_start,
                core_height_start: 1,
                block_count: 1400,
                proposers,
                validator_quorums: quorums,
                current_validator_quorum_hash: current_quorum_hash,
                current_proposer_versions: Some(current_proposer_versions.clone()),
                current_identity_nonce_counter: identity_nonce_counter,
                current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                current_votes: BTreeMap::default(),
                start_time_ms: 1681094380000,
                current_time_ms: end_time_ms,
                instant_lock_quorums,
                current_identities: Vec::new(),
                current_addresses_with_balance: AddressesWithBalance::default(),
            },
            strategy.clone(),
            config.clone(),
            StrategyRandomness::SeedEntropy(7),
        )
        .await;
        let state = platform.state.load();
        {
            let counter = &platform.drive.cache.protocol_versions_counter.read();
            assert_eq!(
                (
                    state
                        .last_committed_block_info()
                        .as_ref()
                        .unwrap()
                        .basic_info()
                        .epoch
                        .index,
                    state.current_protocol_version_in_consensus(),
                    state.next_epoch_protocol_version(),
                    counter.get(&1).unwrap(),
                    counter.get(&TEST_PROTOCOL_VERSION_2).unwrap()
                ),
                (8, 1, TEST_PROTOCOL_VERSION_2, Some(&19), Some(&98))
            );
        }

        // we are now locked in, the current protocol version will change on next epoch

        let block_start = state
            .last_committed_block_info()
            .as_ref()
            .unwrap()
            .basic_info()
            .height
            + 1;
        let ChainExecutionOutcome { .. } = continue_chain_for_strategy(
            abci_app,
            ChainExecutionParameters {
                block_start,
                core_height_start: 1,
                block_count: 400,
                proposers,
                validator_quorums: quorums,
                current_validator_quorum_hash: current_quorum_hash,
                current_proposer_versions: Some(current_proposer_versions),
                current_identity_nonce_counter: identity_nonce_counter,
                current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                current_votes: BTreeMap::default(),
                start_time_ms: 1681094380000,
                current_time_ms: end_time_ms,
                instant_lock_quorums,
                current_identities: Vec::new(),
                current_addresses_with_balance: AddressesWithBalance::default(),
            },
            strategy,
            config,
            StrategyRandomness::SeedEntropy(8),
        )
        .await;

        let state = platform.state.load();

        assert_eq!(
            (
                state
                    .last_committed_block_info()
                    .as_ref()
                    .unwrap()
                    .basic_info()
                    .epoch
                    .index,
                state.current_protocol_version_in_consensus(),
                state.next_epoch_protocol_version()
            ),
            (9, TEST_PROTOCOL_VERSION_2, TEST_PROTOCOL_VERSION_2)
        );
    }

    #[stack_size(4 * 1024 * 1024)]
    #[test]
    #[ignore] // Long-running: runs in nightly CI only
    async fn run_chain_version_upgrade_slow_upgrade_quick_reversion_after_lock_in() {
        drive_abci::logging::init_for_tests(LogLevel::Silent);

        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
                start_addresses: StartAddresses::default(),
                identity_inserts: IdentityInsertInfo::default(),
                identity_contract_nonce_gaps: None,
                signer: None,
            },
            total_hpmns: 200,
            extra_normal_mns: 0,
            validator_quorum_count: 100,
            upgrading_info: Some(UpgradingInfo {
                current_protocol_version: 1,
                proposed_protocol_versions_with_weight: vec![(TEST_PROTOCOL_VERSION_2, 1)],
                upgrade_three_quarters_life: 5.0,
            }),
            proposer_strategy: Default::default(),
            rotate_quorums: false,
            failure_testing: None,
            query_testing: None,
            verify_state_transition_results: false,
            ..Default::default()
        };
        let hour_in_ms = 1000 * 60 * 60;
        let mut config = PlatformConfig {
            network: Regtest,
            validator_set: ValidatorSetConfig {
                quorum_size: 50,
                ..Default::default()
            },
            chain_lock: ChainLockConfig::default_100_67(),
            instant_lock: InstantLockConfig::default_100_67(),
            execution: ExecutionConfig {
                verify_sum_trees: true,
                epoch_time_length_s: 1576800,
                ..Default::default()
            },
            drive: DriveConfig {
                epochs_per_era: 20,
                ..Default::default()
            },
            block_spacing_ms: hour_in_ms,
            testing_configs: PlatformTestConfig::default_minimal_verifications(),
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .with_initial_protocol_version(INITIAL_PROTOCOL_VERSION)
            .build_with_mock_rpc();
        platform
            .core_rpc
            .expect_get_best_chain_lock()
            .returning(move || {
                Ok(ChainLock {
                    block_height: 10,
                    block_hash: BlockHash::from_byte_array([1; 32]),
                    signature: [2; 96].into(),
                })
            });
        let ChainExecutionOutcome {
            abci_app,
            proposers,
            validator_quorums: quorums,
            current_validator_quorum_hash: current_quorum_hash,
            current_proposer_versions,
            end_time_ms,
            identity_nonce_counter,
            identity_contract_nonce_counter,
            instant_lock_quorums,
            ..
        } = run_chain_for_strategy(
            &mut platform,
            2000,
            strategy.clone(),
            config.clone(),
            15,
            &mut None,
            &mut None,
        )
        .await;

        let platform = abci_app.platform;
        let state = platform.state.load();

        {
            assert_eq!(
                state
                    .last_committed_block_info()
                    .as_ref()
                    .unwrap()
                    .basic_info()
                    .epoch
                    .index,
                4
            );
            assert_eq!(state.current_protocol_version_in_consensus(), 1);
        }

        // we still did not yet hit the required threshold to upgrade
        // let's go a just a little longer
        let platform = abci_app.platform;
        let block_start = state
            .last_committed_block_info()
            .as_ref()
            .unwrap()
            .basic_info()
            .height
            + 1;
        let ChainExecutionOutcome {
            abci_app,
            proposers,
            validator_quorums: quorums,
            current_validator_quorum_hash: current_quorum_hash,
            end_time_ms,
            identity_nonce_counter,
            identity_contract_nonce_counter,
            instant_lock_quorums,
            ..
        } = continue_chain_for_strategy(
            abci_app,
            ChainExecutionParameters {
                block_start,
                core_height_start: 1,
                block_count: 3000,
                proposers,
                validator_quorums: quorums,
                current_validator_quorum_hash: current_quorum_hash,
                current_proposer_versions: Some(current_proposer_versions),
                current_identity_nonce_counter: identity_nonce_counter,
                current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                current_votes: BTreeMap::default(),
                start_time_ms: 1681094380000,
                current_time_ms: end_time_ms,
                instant_lock_quorums,
                current_identities: Vec::new(),
                current_addresses_with_balance: AddressesWithBalance::default(),
            },
            strategy,
            config.clone(),
            StrategyRandomness::SeedEntropy(99),
        )
        .await;
        let state = platform.state.load();
        {
            let counter = &platform.drive.cache.protocol_versions_counter.read();
            assert_eq!(
                state
                    .last_committed_block_info()
                    .as_ref()
                    .unwrap()
                    .basic_info()
                    .epoch
                    .index,
                11
            );
            assert_eq!(state.current_protocol_version_in_consensus(), 1);
            assert_eq!(state.next_epoch_protocol_version(), TEST_PROTOCOL_VERSION_2);
            assert_eq!(
                (
                    counter.get(&1).unwrap(),
                    counter.get(&TEST_PROTOCOL_VERSION_2).unwrap()
                ),
                (Some(&16), Some(&117))
            );
            //not all nodes have upgraded
        }

        // we are now locked in, the current protocol version will change on next epoch
        // however most nodes now revert

        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
                start_addresses: StartAddresses::default(),
                identity_inserts: IdentityInsertInfo::default(),
                identity_contract_nonce_gaps: None,
                signer: None,
            },
            total_hpmns: 200,
            extra_normal_mns: 0,
            validator_quorum_count: 100,
            upgrading_info: Some(UpgradingInfo {
                current_protocol_version: 2,
                proposed_protocol_versions_with_weight: vec![(1, 9), (TEST_PROTOCOL_VERSION_2, 1)],
                upgrade_three_quarters_life: 0.1,
            }),
            proposer_strategy: Default::default(),
            rotate_quorums: false,
            failure_testing: None,
            query_testing: None,
            verify_state_transition_results: false,
            ..Default::default()
        };

        let block_start = state
            .last_committed_block_info()
            .as_ref()
            .unwrap()
            .basic_info()
            .height
            + 1;
        config.block_spacing_ms = hour_in_ms / 5; //speed things up
        let ChainExecutionOutcome {
            abci_app,
            proposers,
            validator_quorums: quorums,
            current_validator_quorum_hash: current_quorum_hash,
            current_proposer_versions,
            end_time_ms,
            identity_nonce_counter,
            identity_contract_nonce_counter,
            instant_lock_quorums,
            ..
        } = continue_chain_for_strategy(
            abci_app,
            ChainExecutionParameters {
                block_start,
                core_height_start: 1,
                block_count: 2000,
                proposers,
                validator_quorums: quorums,
                current_validator_quorum_hash: current_quorum_hash,
                current_proposer_versions: None, //restart the proposer versions
                current_identity_nonce_counter: identity_nonce_counter,
                current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                current_votes: BTreeMap::default(),
                start_time_ms: 1681094380000,
                current_time_ms: end_time_ms,
                instant_lock_quorums,
                current_identities: Vec::new(),
                current_addresses_with_balance: AddressesWithBalance::default(),
            },
            strategy.clone(),
            config.clone(),
            StrategyRandomness::SeedEntropy(40),
        )
        .await;
        let state = platform.state.load();
        {
            let counter = &platform.drive.cache.protocol_versions_counter.read();
            assert_eq!(
                (
                    counter.get(&1).unwrap(),
                    counter.get(&TEST_PROTOCOL_VERSION_2).unwrap()
                ),
                (Some(&172), Some(&24))
            );
            //a lot of nodes reverted to previous version, however this won't impact things
            assert_eq!(
                state
                    .last_committed_block_info()
                    .as_ref()
                    .unwrap()
                    .basic_info()
                    .epoch
                    .index,
                12
            );
            assert_eq!(
                state.current_protocol_version_in_consensus(),
                TEST_PROTOCOL_VERSION_2
            );
            assert_eq!(state.next_epoch_protocol_version(), 1);
        }

        let block_start = state
            .last_committed_block_info()
            .as_ref()
            .unwrap()
            .basic_info()
            .height
            + 1;
        config.block_spacing_ms = hour_in_ms * 4; //let's try to move to next epoch
        let ChainExecutionOutcome { .. } = continue_chain_for_strategy(
            abci_app,
            ChainExecutionParameters {
                block_start,
                core_height_start: 1,
                block_count: 100,
                proposers,
                validator_quorums: quorums,
                current_validator_quorum_hash: current_quorum_hash,
                current_proposer_versions: Some(current_proposer_versions),
                current_identity_nonce_counter: identity_nonce_counter,
                current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                current_votes: BTreeMap::default(),
                start_time_ms: 1681094380000,
                current_time_ms: end_time_ms,
                instant_lock_quorums,
                current_identities: Vec::new(),
                current_addresses_with_balance: AddressesWithBalance::default(),
            },
            strategy,
            config,
            StrategyRandomness::SeedEntropy(40),
        )
        .await;
        let state = platform.state.load();
        {
            let counter = &platform.drive.cache.protocol_versions_counter.read();
            assert_eq!(
                (
                    counter.get(&1).unwrap(),
                    counter.get(&TEST_PROTOCOL_VERSION_2).unwrap()
                ),
                (Some(&24), Some(&2))
            );
            assert_eq!(
                state
                    .last_committed_block_info()
                    .as_ref()
                    .unwrap()
                    .basic_info()
                    .epoch
                    .index,
                13
            );
            assert_eq!(state.current_protocol_version_in_consensus(), 1);
            assert_eq!(state.next_epoch_protocol_version(), 1);
        }
    }

    #[stack_size(4 * 1024 * 1024)]
    #[test]
    #[ignore] // Long-running: runs in nightly CI only
    async fn run_chain_version_upgrade_multiple_versions() {
        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
                start_addresses: StartAddresses::default(),
                identity_inserts: IdentityInsertInfo::default(),
                identity_contract_nonce_gaps: None,
                signer: None,
            },
            total_hpmns: 200,
            extra_normal_mns: 0,
            validator_quorum_count: 100,
            upgrading_info: Some(UpgradingInfo {
                current_protocol_version: 1,
                proposed_protocol_versions_with_weight: vec![
                    (1, 3),
                    (TEST_PROTOCOL_VERSION_2, 95),
                    (TEST_PROTOCOL_VERSION_3, 4),
                ],
                upgrade_three_quarters_life: 0.75,
            }),
            proposer_strategy: Default::default(),
            rotate_quorums: false,
            failure_testing: None,
            query_testing: None,
            verify_state_transition_results: false,
            ..Default::default()
        };
        let hour_in_ms = 1000 * 60 * 60;
        let config = PlatformConfig {
            validator_set: ValidatorSetConfig {
                quorum_size: 50,
                ..Default::default()
            },
            chain_lock: ChainLockConfig::default_100_67(),
            instant_lock: InstantLockConfig::default_100_67(),
            execution: ExecutionConfig {
                verify_sum_trees: false,
                epoch_time_length_s: 1576800,
                ..Default::default()
            },
            drive: DriveConfig {
                epochs_per_era: 20,
                ..Default::default()
            },
            block_spacing_ms: hour_in_ms,
            testing_configs: PlatformTestConfig::default_minimal_verifications(),
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .with_initial_protocol_version(INITIAL_PROTOCOL_VERSION)
            .build_with_mock_rpc();
        platform
            .core_rpc
            .expect_get_best_chain_lock()
            .returning(move || {
                Ok(ChainLock {
                    block_height: 10,
                    block_hash: BlockHash::from_byte_array([1; 32]),
                    signature: [2; 96].into(),
                })
            });
        let ChainExecutionOutcome {
            abci_app,
            proposers,
            validator_quorums: quorums,
            current_validator_quorum_hash: current_quorum_hash,
            end_time_ms,
            identity_nonce_counter,
            identity_contract_nonce_counter,
            instant_lock_quorums,
            ..
        } = run_chain_for_strategy(
            &mut platform,
            1200,
            strategy,
            config.clone(),
            15,
            &mut None,
            &mut None,
        )
        .await;
        let state = abci_app.platform.state.load();
        {
            let platform = abci_app.platform;
            let counter = &platform.drive.cache.protocol_versions_counter.read();

            assert_eq!(
                (
                    state
                        .last_committed_block_info()
                        .as_ref()
                        .unwrap()
                        .basic_info()
                        .epoch
                        .index,
                    state.current_protocol_version_in_consensus(),
                    state.next_epoch_protocol_version(),
                    counter.get(&1).unwrap(),
                    counter.get(&TEST_PROTOCOL_VERSION_2).unwrap(),
                    counter.get(&TEST_PROTOCOL_VERSION_3).unwrap()
                ),
                (
                    2,
                    1,
                    TEST_PROTOCOL_VERSION_2,
                    Some(&10),
                    Some(&153),
                    Some(&8)
                )
            ); //some nodes reverted to previous version

            let epochs = platform
                .drive
                .get_epochs_infos(
                    1,
                    1,
                    true,
                    None,
                    state
                        .current_platform_version()
                        .expect("should have version"),
                )
                .expect("should return epochs");

            assert_eq!(epochs.len(), 1);
            assert_eq!(epochs[0].protocol_version(), 1);
        }

        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
                start_addresses: StartAddresses::default(),
                identity_inserts: IdentityInsertInfo::default(),
                identity_contract_nonce_gaps: None,
                signer: None,
            },
            total_hpmns: 200,
            extra_normal_mns: 0,
            validator_quorum_count: 24,
            chain_lock_quorum_count: 24,
            upgrading_info: Some(UpgradingInfo {
                current_protocol_version: 1,
                proposed_protocol_versions_with_weight: vec![
                    (TEST_PROTOCOL_VERSION_2, 3),
                    (TEST_PROTOCOL_VERSION_3, 150),
                ],
                upgrade_three_quarters_life: 0.5,
            }),
            proposer_strategy: Default::default(),
            rotate_quorums: false,
            failure_testing: None,
            query_testing: None,
            verify_state_transition_results: false,
            ..Default::default()
        };

        // we hit the required threshold to upgrade
        // let's go a little longer
        let platform = abci_app.platform;
        let block_start = state
            .last_committed_block_info()
            .as_ref()
            .unwrap()
            .basic_info()
            .height
            + 1;
        let ChainExecutionOutcome { .. } = continue_chain_for_strategy(
            abci_app,
            ChainExecutionParameters {
                block_start,
                core_height_start: 1,
                block_count: 800,
                proposers,
                validator_quorums: quorums,
                current_validator_quorum_hash: current_quorum_hash,
                current_proposer_versions: None,
                current_identity_nonce_counter: identity_nonce_counter,
                current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                current_votes: BTreeMap::default(),
                start_time_ms: 1681094380000,
                current_time_ms: end_time_ms,
                instant_lock_quorums,
                current_identities: Vec::new(),
                current_addresses_with_balance: AddressesWithBalance::default(),
            },
            strategy,
            config,
            StrategyRandomness::SeedEntropy(7),
        )
        .await;
        let state = platform.state.load();
        {
            let counter = &platform.drive.cache.protocol_versions_counter.read();
            assert_eq!(
                (
                    state
                        .last_committed_block_info()
                        .as_ref()
                        .unwrap()
                        .basic_info()
                        .epoch
                        .index,
                    state.current_protocol_version_in_consensus(),
                    state.next_epoch_protocol_version(),
                    counter.get(&1).unwrap(),
                    counter.get(&TEST_PROTOCOL_VERSION_2).unwrap(),
                    counter.get(&TEST_PROTOCOL_VERSION_3).unwrap()
                ),
                (
                    4,
                    TEST_PROTOCOL_VERSION_2,
                    TEST_PROTOCOL_VERSION_3,
                    None,
                    Some(&3),
                    Some(&149)
                )
            );

            let epochs = platform
                .drive
                .get_epochs_infos(
                    3,
                    1,
                    true,
                    None,
                    state
                        .current_platform_version()
                        .expect("should have version"),
                )
                .expect("should return epochs");

            assert_eq!(epochs.len(), 1);
            assert_eq!(epochs[0].protocol_version(), TEST_PROTOCOL_VERSION_2);
        }
    }
}
