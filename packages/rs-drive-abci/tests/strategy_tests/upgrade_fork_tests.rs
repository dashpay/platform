#[cfg(test)]
mod tests {
    use dpp::block::block_info::BlockInfo;
    use dpp::block::extended_block_info::v0::ExtendedBlockInfoV0Getters;
    use dpp::block::extended_epoch_info::v0::ExtendedEpochInfoV0Getters;
    use dpp::dashcore::hashes::Hash;
    use dpp::dashcore::{BlockHash, ChainLock};
    use dpp::version::PlatformVersion;
    use drive::drive::config::DriveConfig;

    use crate::execution::{continue_chain_for_strategy, run_chain_for_strategy};
    use crate::strategy::{
        ChainExecutionOutcome, ChainExecutionParameters, CoreHeightIncrease,
        MasternodeListChangesStrategy, NetworkStrategy, StrategyRandomness, UpgradingInfo,
    };
    use drive_abci::config::{
        ChainLockConfig, ExecutionConfig, InstantLockConfig, PlatformConfig, PlatformTestConfig,
        ValidatorSetConfig,
    };
    use drive_abci::mimic::MimicExecuteBlockOptions;
    use drive_abci::platform_types::platform_state::v0::PlatformStateV0Methods;
    use drive_abci::test::helpers::setup::TestPlatformBuilder;
    use platform_version::version::mocks::v2_test::{TEST_PLATFORM_V2, TEST_PROTOCOL_VERSION_2};
    use platform_version::version::mocks::v3_test::{TEST_PLATFORM_V3, TEST_PROTOCOL_VERSION_3};
    use platform_version::version::mocks::TEST_PROTOCOL_VERSION_SHIFT_BYTES;
    use platform_version::version::v1::PLATFORM_V1;
    use strategy_tests::frequency::Frequency;
    use strategy_tests::{IdentityInsertInfo, StartIdentities, Strategy};

    #[test]
    fn run_chain_version_upgrade() {
        // Define the desired stack size
        let stack_size = 4 * 1024 * 1024; //Lets set the stack size to be higher than the default 2MB

        let builder = std::thread::Builder::new()
            .stack_size(stack_size)
            .name("custom_stack_size_thread".into());

        let handler = builder
            .spawn(|| {
                let platform_version = PlatformVersion::first();
                let strategy = NetworkStrategy {
                    strategy: Strategy {
                        start_contracts: vec![],
                        operations: vec![],
                        start_identities: StartIdentities::default(),
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
                        validator_set_rotation_block_count: 125,
                        epoch_time_length_s: 1576800,
                        ..Default::default()
                    },
                    drive: DriveConfig {
                        epochs_per_era: 20,
                        ..Default::default()
                    },
                    block_spacing_ms: twenty_minutes_in_ms,
                    testing_configs: PlatformTestConfig::default_with_no_block_signing(),

                    ..Default::default()
                };
                let mut platform = TestPlatformBuilder::new()
                    .with_config(config.clone())
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
                );

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
                        (Some(&17), Some(&414))
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
                        start_time_ms: 1681094380000,
                        current_time_ms: end_time_ms,
                        instant_lock_quorums,
                    },
                    strategy.clone(),
                    config.clone(),
                    StrategyRandomness::SeedEntropy(7),
                );

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
                    assert_eq!(counter.get(&TEST_PROTOCOL_VERSION_2).unwrap(), Some(&154));
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
                        start_time_ms: 1681094380000,
                        current_time_ms: end_time_ms,
                        instant_lock_quorums,
                    },
                    strategy,
                    config,
                    StrategyRandomness::SeedEntropy(18),
                );

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
                    assert_eq!(counter.get(&TEST_PROTOCOL_VERSION_2).unwrap(), Some(&123));
                }
            })
            .expect("Failed to create thread with custom stack size");

        // Wait for the thread to finish and assert that it didn't panic.
        handler.join().expect("Thread has panicked");
    }

    #[test]
    fn run_chain_quick_version_upgrade() {
        // Define the desired stack size
        let stack_size = 4 * 1024 * 1024; //Lets set the stack size to be higher than the default 2MB

        let builder = std::thread::Builder::new()
            .stack_size(stack_size)
            .name("custom_stack_size_thread".into());

        let handler = builder
            .spawn(|| {
                let platform_version = PlatformVersion::first();
                let strategy = NetworkStrategy {
                    strategy: Strategy {
                        start_contracts: vec![],
                        operations: vec![],
                        start_identities: StartIdentities::default(),
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
                        validator_set_rotation_block_count: 30,
                        epoch_time_length_s: one_hour_in_s,
                        ..Default::default()
                    },
                    block_spacing_ms: thirty_seconds_in_ms,
                    testing_configs: PlatformTestConfig::default_with_no_block_signing(),

                    ..Default::default()
                };
                let mut platform = TestPlatformBuilder::new()
                    .with_config(config.clone())
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
                );

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
                        start_time_ms: 1681094380000,
                        current_time_ms: end_time_ms,
                        instant_lock_quorums,
                    },
                    strategy.clone(),
                    config.clone(),
                    StrategyRandomness::SeedEntropy(7),
                );

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
                        start_time_ms: 1681094380000,
                        current_time_ms: end_time_ms,
                        instant_lock_quorums,
                    },
                    strategy,
                    config,
                    StrategyRandomness::SeedEntropy(18),
                );
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
            })
            .expect("Failed to create thread with custom stack size");

        // Wait for the thread to finish and assert that it didn't panic.
        handler.join().expect("Thread has panicked");
    }

    #[test]
    fn run_chain_on_epoch_change_with_new_version_and_removing_votes() {
        // Add a new version to upgrade to new protocol version only with one vote
        const TEST_PROTOCOL_VERSION_4_WITH_1_HPMN_UPGRADE: u32 =
            (1 << TEST_PROTOCOL_VERSION_SHIFT_BYTES) + 4;

        let mut test_platform_v4 = PLATFORM_V1.clone();
        test_platform_v4
            .drive_abci
            .methods
            .protocol_upgrade
            .protocol_version_upgrade_percentage_needed = 1;

        PlatformVersion::replace_test_versions(vec![
            TEST_PLATFORM_V2,
            TEST_PLATFORM_V3,
            test_platform_v4,
        ]);

        let strategy = NetworkStrategy {
            total_hpmns: 50,
            upgrading_info: Some(UpgradingInfo {
                current_protocol_version: TEST_PROTOCOL_VERSION_4_WITH_1_HPMN_UPGRADE,
                proposed_protocol_versions_with_weight: vec![(TEST_PROTOCOL_VERSION_2, 1)],
                upgrade_three_quarters_life: 0.0,
            }),
            core_height_increase: CoreHeightIncrease::KnownCoreHeightIncreases(vec![1, 2, 3, 4, 5]),
            // Remove HPMNs to trigger remove_validators_proposed_app_versions
            proposer_strategy: MasternodeListChangesStrategy {
                removed_hpmns: Frequency {
                    times_per_block_range: 1..2,
                    chance_per_block: None,
                },
                ..Default::default()
            },
            ..Default::default()
        };

        // 1 block is 1 epoch
        let epoch_time_length_s = 60;

        let config = PlatformConfig {
            validator_set: ValidatorSetConfig {
                quorum_size: 30,
                ..Default::default()
            },
            chain_lock: ChainLockConfig::default_100_67(),
            instant_lock: InstantLockConfig::default_100_67(),
            execution: ExecutionConfig {
                epoch_time_length_s,
                ..Default::default()
            },
            initial_protocol_version: TEST_PROTOCOL_VERSION_4_WITH_1_HPMN_UPGRADE,
            block_spacing_ms: epoch_time_length_s * 1000,
            testing_configs: PlatformTestConfig {
                block_signing: false,
                block_commit_signature_verification: false,
                disable_instant_lock_signature_verification: true,
            },
            ..Default::default()
        };

        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let ChainExecutionOutcome {
            abci_app,
            proposers,
            validator_quorums: quorums,
            current_validator_quorum_hash: current_quorum_hash,
            end_time_ms,
            ..
        } = run_chain_for_strategy(&mut platform, 1, strategy.clone(), config.clone(), 13);

        let platform = abci_app.platform;

        let state = platform.state.load();
        let counter = platform.drive.cache.protocol_versions_counter.read();

        assert_eq!(state.last_committed_block_epoch().index, 0);
        assert_eq!(
            state.current_protocol_version_in_consensus(),
            TEST_PROTOCOL_VERSION_4_WITH_1_HPMN_UPGRADE
        );
        assert_eq!(
            state.next_epoch_protocol_version(),
            TEST_PROTOCOL_VERSION_4_WITH_1_HPMN_UPGRADE
        );
        assert_eq!(state.last_committed_core_height(), 2);
        assert_eq!(counter.get(&1).unwrap(), None);
        assert_eq!(counter.get(&TEST_PROTOCOL_VERSION_2).unwrap(), Some(&1));
        assert_eq!(counter.get(&TEST_PROTOCOL_VERSION_3).unwrap(), None);
        assert_eq!(
            counter
                .get(&TEST_PROTOCOL_VERSION_4_WITH_1_HPMN_UPGRADE)
                .unwrap(),
            None
        );

        drop(counter);

        // Next bock is epoch change. We want to test our protocol
        // upgrade logic. We will propose a new version and remove HPMN
        // to make sure all protocol version count functions are called during block execution.

        let last_committed_block_info = state
            .last_committed_block_info()
            .as_ref()
            .unwrap()
            .basic_info();

        let proposer_pro_tx_hash = proposers
            .first()
            .expect("we should have proposers")
            .masternode
            .pro_tx_hash;

        let current_quorum_with_test_info =
            quorums.get(&current_quorum_hash).expect("expected quorum");

        // We want to add proposal for a new version
        let proposed_version = TEST_PROTOCOL_VERSION_3;

        let block_info = BlockInfo {
            time_ms: end_time_ms + epoch_time_length_s + 1,
            height: last_committed_block_info.height + 1,
            core_height: last_committed_block_info.core_height,
            epoch: Default::default(),
        };

        abci_app
            .mimic_execute_block(
                proposer_pro_tx_hash.into(),
                current_quorum_with_test_info,
                proposed_version,
                block_info,
                0,
                &[],
                false,
                Vec::new(),
                MimicExecuteBlockOptions {
                    dont_finalize_block: strategy.dont_finalize_block(),
                    rounds_before_finalization: strategy
                        .failure_testing
                        .as_ref()
                        .and_then(|failure_testing| failure_testing.rounds_before_successful_block),
                    max_tx_bytes_per_block: strategy.max_tx_bytes_per_block,
                    independent_process_proposal_verification: strategy
                        .independent_process_proposal_verification,
                },
            )
            .expect("expected to execute a block");

        let state = platform.state.load();
        let counter = platform.drive.cache.protocol_versions_counter.read();

        assert_eq!(state.last_committed_block_epoch().index, 1);
        assert_eq!(
            state.current_protocol_version_in_consensus(),
            TEST_PROTOCOL_VERSION_4_WITH_1_HPMN_UPGRADE
        );
        assert_eq!(state.next_epoch_protocol_version(), TEST_PROTOCOL_VERSION_2);
        assert_eq!(counter.get(&1).unwrap(), None);
        assert_eq!(counter.get(&TEST_PROTOCOL_VERSION_2).unwrap(), None);
        assert_eq!(counter.get(&TEST_PROTOCOL_VERSION_3).unwrap(), Some(&1));
        assert_eq!(state.last_committed_core_height(), 3);
    }

    #[test]
    fn run_chain_version_upgrade_slow_upgrade() {
        // Define the desired stack size
        let stack_size = 4 * 1024 * 1024; //Lets set the stack size to be higher than the default 2MB

        let builder = std::thread::Builder::new()
            .stack_size(stack_size)
            .name("custom_stack_size_thread".into());

        let handler = builder
            .spawn(|| {
                let strategy = NetworkStrategy {
                    strategy: Strategy {
                        start_contracts: vec![],
                        operations: vec![],
                        start_identities: StartIdentities::default(),
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
                    validator_set: ValidatorSetConfig {
                        quorum_size: 40,
                        ..Default::default()
                    },
                    chain_lock: ChainLockConfig::default_100_67(),
                    instant_lock: InstantLockConfig::default_100_67(),
                    execution: ExecutionConfig {
                        verify_sum_trees: true,
                        validator_set_rotation_block_count: 80,
                        epoch_time_length_s: 1576800,
                        ..Default::default()
                    },
                    drive: DriveConfig {
                        epochs_per_era: 20,
                        ..Default::default()
                    },
                    block_spacing_ms: hour_in_ms,

                    testing_configs: PlatformTestConfig::default_with_no_block_signing(),
                    ..Default::default()
                };
                let mut platform = TestPlatformBuilder::new()
                    .with_config(config.clone())
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
                );
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
                        (Some(&35), Some(&64))
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
                        block_count: 2500,
                        proposers,
                        validator_quorums: quorums,
                        current_validator_quorum_hash: current_quorum_hash,
                        current_proposer_versions: Some(current_proposer_versions.clone()),
                        current_identity_nonce_counter: identity_nonce_counter,
                        current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                        start_time_ms: 1681094380000,
                        current_time_ms: end_time_ms,
                        instant_lock_quorums,
                    },
                    strategy.clone(),
                    config.clone(),
                    StrategyRandomness::SeedEntropy(7),
                );
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
                    // the counter is for the current voting during that window
                    assert_eq!(
                        (
                            counter.get(&1).unwrap(),
                            counter.get(&TEST_PROTOCOL_VERSION_2).unwrap()
                        ),
                        (Some(&8), Some(&79))
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
                        start_time_ms: 1681094380000,
                        current_time_ms: end_time_ms,
                        instant_lock_quorums,
                    },
                    strategy,
                    config,
                    StrategyRandomness::SeedEntropy(8),
                );

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
                        12
                    );
                    assert_eq!(
                        state.current_protocol_version_in_consensus(),
                        TEST_PROTOCOL_VERSION_2
                    );
                    assert_eq!(state.next_epoch_protocol_version(), TEST_PROTOCOL_VERSION_2);
                }
            })
            .expect("Failed to create thread with custom stack size");

        // Wait for the thread to finish and assert that it didn't panic.
        handler.join().expect("Thread has panicked");
    }

    #[test]
    fn run_chain_version_upgrade_slow_upgrade_quick_reversion_after_lock_in() {
        // Define the desired stack size
        let stack_size = 4 * 1024 * 1024; //Lets set the stack size to be higher than the default 2MB

        let builder = std::thread::Builder::new()
            .stack_size(stack_size)
            .name("custom_stack_size_thread".into());

        let handler = builder
            .spawn(|| {
                let strategy = NetworkStrategy {
                    strategy: Strategy {
                        start_contracts: vec![],
                        operations: vec![],
                        start_identities: StartIdentities::default(),
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
                    validator_set: ValidatorSetConfig {
                        quorum_size: 50,
                        ..Default::default()
                    },
                    chain_lock: ChainLockConfig::default_100_67(),
                    instant_lock: InstantLockConfig::default_100_67(),
                    execution: ExecutionConfig {
                        verify_sum_trees: true,
                        validator_set_rotation_block_count: 50,
                        epoch_time_length_s: 1576800,
                        ..Default::default()
                    },
                    drive: DriveConfig {
                        epochs_per_era: 20,
                        ..Default::default()
                    },
                    block_spacing_ms: hour_in_ms,

                    testing_configs: PlatformTestConfig::default_with_no_block_signing(),
                    ..Default::default()
                };
                let mut platform = TestPlatformBuilder::new()
                    .with_config(config.clone())
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
                );

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
                        start_time_ms: 1681094380000,
                        current_time_ms: end_time_ms,
                        instant_lock_quorums,
                    },
                    strategy,
                    config.clone(),
                    StrategyRandomness::SeedEntropy(99),
                );
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
                        (Some(&18), Some(&112))
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
                        identity_inserts: IdentityInsertInfo::default(),

                        identity_contract_nonce_gaps: None,
                        signer: None,
                    },
                    total_hpmns: 200,
                    extra_normal_mns: 0,
                    validator_quorum_count: 100,
                    upgrading_info: Some(UpgradingInfo {
                        current_protocol_version: 2,
                        proposed_protocol_versions_with_weight: vec![
                            (1, 9),
                            (TEST_PROTOCOL_VERSION_2, 1),
                        ],
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
                        start_time_ms: 1681094380000,
                        current_time_ms: end_time_ms,
                        instant_lock_quorums,
                    },
                    strategy.clone(),
                    config.clone(),
                    StrategyRandomness::SeedEntropy(40),
                );
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
                    //a lot nodes reverted to previous version, however this won't impact things
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
                        start_time_ms: 1681094380000,
                        current_time_ms: end_time_ms,
                        instant_lock_quorums,
                    },
                    strategy,
                    config,
                    StrategyRandomness::SeedEntropy(40),
                );
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
            })
            .expect("Failed to create thread with custom stack size");

        // Wait for the thread to finish and assert that it didn't panic.
        handler.join().expect("Thread has panicked");
    }

    #[test]
    fn run_chain_version_upgrade_multiple_versions() {
        // Define the desired stack size
        let stack_size = 4 * 1024 * 1024; //Lets set the stack size to be higher than the default 2MB

        let builder = std::thread::Builder::new()
            .stack_size(stack_size)
            .name("custom_stack_size_thread".into());

        let handler = builder
            .spawn(|| {
                let strategy = NetworkStrategy {
                    strategy: Strategy {
                        start_contracts: vec![],
                        operations: vec![],
                        start_identities: StartIdentities::default(),
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
                        verify_sum_trees: true,
                        validator_set_rotation_block_count: 30,
                        epoch_time_length_s: 1576800,
                        ..Default::default()
                    },
                    drive: DriveConfig {
                        epochs_per_era: 20,
                        ..Default::default()
                    },
                    block_spacing_ms: hour_in_ms,

                    testing_configs: PlatformTestConfig::default_with_no_block_signing(),
                    ..Default::default()
                };
                let mut platform = TestPlatformBuilder::new()
                    .with_config(config.clone())
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
                } = run_chain_for_strategy(&mut platform, 1400, strategy, config.clone(), 15);
                let state = abci_app.platform.state.load();
                {
                    let platform = abci_app.platform;
                    let counter = &platform.drive.cache.protocol_versions_counter.read();

                    assert_eq!(
                        state
                            .last_committed_block_info()
                            .as_ref()
                            .unwrap()
                            .basic_info()
                            .epoch
                            .index,
                        3
                    );
                    assert_eq!(state.current_protocol_version_in_consensus(), 1);
                    assert_eq!(state.next_epoch_protocol_version(), TEST_PROTOCOL_VERSION_2);
                    assert_eq!(
                        (
                            counter.get(&1).unwrap(),
                            counter.get(&TEST_PROTOCOL_VERSION_2).unwrap(),
                            counter.get(&TEST_PROTOCOL_VERSION_3).unwrap()
                        ),
                        (Some(&2), Some(&68), Some(&3))
                    ); //some nodes reverted to previous version

                    let epochs = platform
                        .drive
                        .get_epochs_infos(
                            2,
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
                        block_count: 1100,
                        proposers,
                        validator_quorums: quorums,
                        current_validator_quorum_hash: current_quorum_hash,
                        current_proposer_versions: None,
                        current_identity_nonce_counter: identity_nonce_counter,
                        current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                        start_time_ms: 1681094380000,
                        current_time_ms: end_time_ms,
                        instant_lock_quorums,
                    },
                    strategy,
                    config,
                    StrategyRandomness::SeedEntropy(7),
                );
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
                        5
                    );
                    assert_eq!(
                        state.current_protocol_version_in_consensus(),
                        TEST_PROTOCOL_VERSION_2
                    );
                    assert_eq!(state.next_epoch_protocol_version(), TEST_PROTOCOL_VERSION_3);
                    assert_eq!(
                        (
                            counter.get(&1).unwrap(),
                            counter.get(&TEST_PROTOCOL_VERSION_2).unwrap(),
                            counter.get(&TEST_PROTOCOL_VERSION_3).unwrap()
                        ),
                        (None, Some(&3), Some(&143))
                    );

                    let epochs = platform
                        .drive
                        .get_epochs_infos(
                            4,
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
            })
            .expect("Failed to create thread with custom stack size");

        // Wait for the thread to finish and assert that it didn't panic.
        handler.join().expect("Thread has panicked");
    }
}
