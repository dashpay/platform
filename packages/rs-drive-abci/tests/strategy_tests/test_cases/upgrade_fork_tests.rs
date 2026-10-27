#[cfg(test)]
mod tests {
    use crate::addresses_with_balance::AddressesWithBalance;
    use crate::execution::{continue_chain_for_strategy, run_chain_for_strategy, GENESIS_TIME_MS};
    use crate::strategy::{
        ChainExecutionOutcome, ChainExecutionParameters, CoreHeightIncrease, NetworkStrategy,
        StrategyRandomness, UpgradingInfo,
    };
    use dash_platform_macros::stack_size;
    use dpp::block::epoch::Epoch;
    use dpp::block::extended_block_info::v0::ExtendedBlockInfoV0Getters;
    use dpp::block::extended_epoch_info::v0::ExtendedEpochInfoV0Getters;
    use dpp::core_types::validator_set::v0::ValidatorSetV0Getters;
    use dpp::dashcore::hashes::Hash;
    use dpp::dashcore::Network::Regtest;
    use dpp::dashcore::{BlockHash, ChainLock, ProTxHash};
    use dpp::dashcore_rpc::dashcore_rpc_json::{
        DMNStateDiff, MasternodeAddresses, MasternodeListDiff,
    };
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
    use drive_abci::platform_types::masternode::v0::MasternodeV0;
    use drive_abci::platform_types::platform::Platform;
    use drive_abci::platform_types::platform_state::PlatformStateV0Methods;
    use drive_abci::rpc::core::MockCoreRPCLike;
    use drive_abci::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
    use platform_version::version::mocks::v2_test::TEST_PROTOCOL_VERSION_2;
    use platform_version::version::mocks::v3_test::TEST_PROTOCOL_VERSION_3;
    use platform_version::version::INITIAL_PROTOCOL_VERSION;
    use std::collections::BTreeMap;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::{Arc, Mutex};
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

    /// The app-connect contract only comes into existence through the upgrade to protocol
    /// version 14 (or a version-14 genesis). Driven through real blocks: absent while the
    /// chain still runs 13, written by the activation block, served by the system contract
    /// cache from 14 only, and still there after Drive is closed and reopened.
    #[stack_size(4 * 1024 * 1024)]
    #[test]
    async fn run_chain_v13_to_v14_registers_the_app_connect_contract() {
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
                current_protocol_version: 13,
                proposed_protocol_versions_with_weight: vec![(14, 1)],
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
            .with_initial_protocol_version(13)
            .build_with_mock_rpc();

        let platform_version_13 = PlatformVersion::get(13).expect("platform version 13");
        let platform_version_14 = PlatformVersion::get(14).expect("platform version 14");
        let app_connect_id = SystemDataContract::AppConnect.id();

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
        assert_eq!(state.current_protocol_version_in_consensus(), 13);
        assert_eq!(state.next_epoch_protocol_version(), 13);
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
        assert_eq!(state.current_protocol_version_in_consensus(), 13);
        assert_eq!(state.next_epoch_protocol_version(), 14);
        let block_start = state
            .last_committed_block_info()
            .as_ref()
            .expect("expected committed block info")
            .basic_info()
            .height
            + 1;
        drop(state);

        // Locked in but not yet active: nothing has written the contract.
        assert!(abci_app
            .platform
            .drive
            .fetch_contract(
                app_connect_id.to_buffer(),
                None,
                None,
                None,
                platform_version_13,
            )
            .value
            .expect("query the app-connect contract before activation")
            .is_none());
        assert!(abci_app
            .platform
            .drive
            .cache
            .system_data_contracts
            .find_by_id(app_connect_id, platform_version_13)
            .expect("expected the pre-activation lookup to succeed")
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
        assert_eq!(state.current_protocol_version_in_consensus(), 14);
        assert_eq!(state.next_epoch_protocol_version(), 14);
        drop(state);

        let stored = abci_app
            .platform
            .drive
            .fetch_contract(
                app_connect_id.to_buffer(),
                None,
                None,
                None,
                platform_version_14,
            )
            .value
            .expect("fetch the app-connect contract after activation")
            .expect("the activation block must write the app-connect contract");
        assert_eq!(stored.contract.id(), app_connect_id);
        assert!(stored
            .contract
            .document_type_for_name("loginKeyResponse")
            .is_ok());
        assert!(abci_app
            .platform
            .drive
            .cache
            .system_data_contracts
            .find_by_id(app_connect_id, platform_version_14)
            .expect("expected the post-activation lookup to succeed")
            .is_some());
        assert!(abci_app
            .platform
            .drive
            .cache
            .system_data_contracts
            .find_by_id(app_connect_id, platform_version_13)
            .expect("an old-version lookup must still succeed")
            .is_none());
        drop(abci_app);

        let TempPlatform {
            platform: platform_before_restart,
            tempdir,
        } = platform;
        drop(platform_before_restart);

        let reopened_platform = TempPlatform::open_with_tempdir(tempdir, config);
        let state = reopened_platform.state.load();
        assert_eq!(state.current_protocol_version_in_consensus(), 14);
        drop(state);
        assert!(reopened_platform
            .drive
            .fetch_contract(
                app_connect_id.to_buffer(),
                None,
                None,
                None,
                platform_version_14,
            )
            .value
            .expect("fetch the app-connect contract after restart")
            .is_some());
        assert!(reopened_platform
            .drive
            .cache
            .system_data_contracts
            .find_by_id(app_connect_id, platform_version_14)
            .expect("expected the lookup after restart to succeed")
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

    /// Nested addresses in an old node's memory are absent from its persisted masternode
    /// entries. Activation and subsequent partial address diffs must therefore start from
    /// the same stored ports whether the node kept running or restarted before activation.
    #[stack_size(4 * 1024 * 1024)]
    #[test]
    async fn should_preserve_masternode_ports_across_activation_and_restart() {
        const BLOCKS_PER_EPOCH: u64 = 60;
        const NEW_P2P_PORT: u16 = 29999;
        let previous_version = PlatformVersion::get(13).expect("protocol version 13");
        let latest_version = PlatformVersion::latest();
        let strategy = NetworkStrategy {
            total_hpmns: 50,
            validator_quorum_count: 24,
            chain_lock_quorum_count: 24,
            upgrading_info: Some(UpgradingInfo {
                current_protocol_version: previous_version.protocol_version,
                proposed_protocol_versions_with_weight: vec![(latest_version.protocol_version, 1)],
                upgrade_three_quarters_life: 0.0,
            }),
            // Allocate mock Core block hashes through height 2; the explicit chain-lock
            // expectation below controls when that second height becomes visible.
            core_height_increase: CoreHeightIncrease::KnownCoreHeightIncreases(vec![1, 2]),
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
        let mut observations = Vec::new();
        for restart_before_activation in [false, true] {
            let mut platform = TestPlatformBuilder::new()
                .with_config(config.clone())
                .with_initial_protocol_version(previous_version.protocol_version)
                .build_with_mock_rpc();
            let core_height = Arc::new(AtomicU32::new(1));
            let reported_core_height = Arc::clone(&core_height);
            platform
                .core_rpc
                .expect_get_best_chain_lock()
                .returning(move || {
                    Ok(ChainLock {
                        block_height: reported_core_height.load(Ordering::SeqCst),
                        block_hash: BlockHash::from_byte_array([1; 32]),
                        signature: [2; 96].into(),
                    })
                });
            let updated_masternode = Arc::new(Mutex::new(None::<ProTxHash>));
            let rpc_updated_masternode = Arc::clone(&updated_masternode);
            platform
                .core_rpc
                .expect_get_protx_diff_with_masternodes()
                .withf(|base_height, height| *base_height == Some(1) && *height == 2)
                .returning(move |_, _| {
                    let pro_tx_hash = rpc_updated_masternode
                        .lock()
                        .expect("masternode fixture lock")
                        .expect("masternode fixture selected before Core advances");
                    let state_diff: DMNStateDiff = serde_json::from_value(serde_json::json!({
                        "addresses": { "platform_p2p": [format!("1.2.3.4:{NEW_P2P_PORT}")] }
                    }))
                    .expect("Core address diff");
                    Ok(MasternodeListDiff {
                        base_height: 1,
                        block_height: 2,
                        added_mns: vec![],
                        removed_mns: vec![],
                        updated_mns: vec![(pro_tx_hash, state_diff)],
                    })
                });

            let before_activation = run_chain_for_strategy(
                &mut platform,
                2 * BLOCKS_PER_EPOCH,
                strategy.clone(),
                config.clone(),
                13,
                &mut None,
                &mut None,
            )
            .await;
            let committed_before_activation =
                committed_upgrade_state(before_activation.abci_app.platform);
            assert_eq!(committed_before_activation.epoch_index, 1);
            assert_eq!(committed_before_activation.current_protocol_version, 13);
            assert_eq!(
                committed_before_activation.next_epoch_protocol_version,
                latest_version.protocol_version
            );
            let activation_parameters = continuation_parameters(&before_activation, 1);
            let quorum_hash = before_activation.current_validator_quorum_hash;
            let current_platform = before_activation.abci_app.platform;
            let mut state = current_platform.state.load().as_ref().clone();
            let pro_tx_hash = *state.validator_sets()[&quorum_hash]
                .members()
                .keys()
                .next()
                .expect("populated validator set");
            *updated_masternode.lock().expect("masternode fixture lock") = Some(pro_tx_hash);
            #[allow(deprecated)]
            let original_ports = {
                let masternode = &state.full_masternode_list()[&pro_tx_hash];
                (
                    masternode
                        .state
                        .legacy_platform_p2p_port
                        .expect("legacy P2P port") as u16,
                    masternode
                        .state
                        .legacy_platform_http_port
                        .expect("legacy HTTPS port") as u16,
                )
            };
            let transient_addresses = Some(MasternodeAddresses {
                core_p2p: vec!["1.2.3.4:9999".to_string()],
                platform_p2p: vec!["1.2.3.4:36656".to_string()],
                platform_https: vec!["1.2.3.4:8443".to_string()],
            });
            assert_ne!(original_ports, (36656, 8443));
            // Populate the raw old-protocol state deliberately: its RPC representation
            // retains nested addresses, while the historical stored representation does not.
            state
                .full_masternode_list_mut()
                .get_mut(&pro_tx_hash)
                .expect("masternode")
                .state
                .addresses = transient_addresses.clone();
            state
                .hpmn_masternode_list_mut()
                .get_mut(&pro_tx_hash)
                .expect("evonode")
                .state
                .addresses = transient_addresses;
            let root_before_fixture = current_platform
                .drive
                .grove
                .root_hash(None, &previous_version.drive.grove_version)
                .unwrap()
                .expect("stored root");
            current_platform
                .store_platform_state(&state, None, previous_version)
                .expect("persist old-protocol masternode fixture");
            assert_eq!(
                current_platform
                    .drive
                    .grove
                    .root_hash(None, &previous_version.drive.grove_version)
                    .unwrap()
                    .expect("stored root after fixture"),
                root_before_fixture,
                "transient old-protocol addresses must not change persisted state"
            );
            current_platform.state.store(Arc::new(state));
            drop(before_activation);

            if restart_before_activation {
                let TempPlatform {
                    platform: mut old_platform,
                    tempdir,
                } = platform;
                let core_rpc = std::mem::take(&mut old_platform.core_rpc);
                drop(old_platform);
                platform = TempPlatform::open_with_tempdir(tempdir, config.clone());
                platform.platform.core_rpc = core_rpc;
                assert!(platform.state.load().full_masternode_list()[&pro_tx_hash]
                    .state
                    .addresses
                    .is_none());
            }
            assert_eq!(
                committed_upgrade_state(&platform),
                committed_before_activation
            );

            let activation = continue_chain_for_strategy(
                FullAbciApplication::new(&platform),
                activation_parameters,
                strategy.clone(),
                config.clone(),
                StrategyRandomness::SeedEntropy(18),
            )
            .await;
            let activated = committed_upgrade_state(&platform);
            assert_eq!(activated.epoch_index, 2);
            assert_eq!(
                activated.current_protocol_version,
                latest_version.protocol_version
            );
            assert_eq!(
                activated.next_epoch_protocol_version,
                latest_version.protocol_version
            );
            assert_eq!(
                platform.state.load().last_committed_core_height(),
                1,
                "activation must also work without a Core-height advance"
            );
            let ports = |platform: &Platform<MockCoreRPCLike>| {
                let state = platform.state.load();
                let validator = &state.validator_sets()[&quorum_hash].members()[&pro_tx_hash];
                (validator.platform_p2p_port, validator.platform_http_port)
            };
            assert_eq!(ports(&platform), original_ports);
            let after_activation_parameters = continuation_parameters(&activation, 1);
            drop(activation);
            core_height.store(2, Ordering::SeqCst);
            let after_update = continue_chain_for_strategy(
                FullAbciApplication::new(&platform),
                after_activation_parameters,
                strategy.clone(),
                config.clone(),
                StrategyRandomness::SeedEntropy(19),
            )
            .await;
            assert_eq!(platform.state.load().last_committed_core_height(), 2);
            assert_eq!(
                ports(&platform),
                (NEW_P2P_PORT, original_ports.1),
                "a partial new-protocol diff must not restore transient pre-upgrade HTTPS ports"
            );
            let state = platform.state.load();
            #[allow(deprecated)]
            let stored_ports = {
                let masternode = &state.full_masternode_list()[&pro_tx_hash];
                (
                    masternode.state.legacy_platform_p2p_port,
                    masternode.state.legacy_platform_http_port,
                )
            };
            assert_eq!(
                stored_ports,
                (
                    Some(u32::from(NEW_P2P_PORT)),
                    Some(u32::from(original_ports.1))
                )
            );
            observations.push((
                committed_before_activation,
                activated,
                committed_upgrade_state(&platform),
                state
                    .full_masternode_list()
                    .values()
                    .cloned()
                    .map(MasternodeV0::from)
                    .collect::<Vec<_>>(),
                state.validator_sets().clone(),
            ));
            drop(state);
            drop(after_update);
            let TempPlatform {
                platform: old_platform,
                tempdir,
            } = platform;
            drop(old_platform);
            let reopened = TempPlatform::open_with_tempdir(tempdir, config.clone());
            let observed = observations.last().expect("recorded committed state");
            assert_eq!(committed_upgrade_state(&reopened), observed.2);
            assert_eq!(ports(&reopened), (NEW_P2P_PORT, original_ports.1));
            let state = reopened.state.load();
            assert_eq!(
                state
                    .full_masternode_list()
                    .values()
                    .cloned()
                    .map(MasternodeV0::from)
                    .collect::<Vec<_>>(),
                observed.3,
                "post-activation restart must preserve all stored masternodes"
            );
            assert_eq!(state.validator_sets(), &observed.4);
        }
        assert_eq!(observations[0], observations[1],
            "live and restarted nodes must commit equal roots, stored masternodes and validator sets");
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
