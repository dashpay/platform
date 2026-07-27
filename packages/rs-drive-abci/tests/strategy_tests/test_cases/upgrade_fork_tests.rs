#[cfg(test)]
mod tests {
    use crate::addresses_with_balance::AddressesWithBalance;
    use crate::execution::{continue_chain_for_strategy, run_chain_for_strategy};
    use crate::strategy::{
        ChainExecutionOutcome, ChainExecutionParameters, NetworkStrategy, StrategyRandomness,
        UpgradingInfo,
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
    use drive::config::DriveConfig;
    use drive::query::proposer_block_count_query::ProposerQueryType;
    use drive_abci::abci::app::FullAbciApplication;
    use drive_abci::config::{
        ChainLockConfig, ExecutionConfig, InstantLockConfig, PlatformConfig, PlatformTestConfig,
        ValidatorSetConfig,
    };
    use drive_abci::logging::LogLevel;
    use drive_abci::platform_types::platform_state::PlatformStateV0Methods;
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
