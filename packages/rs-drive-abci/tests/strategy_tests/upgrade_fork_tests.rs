#[cfg(test)]
mod tests {
    use dpp::block::extended_block_info::v0::ExtendedBlockInfoV0Getters;
    use dpp::dashcore::hashes::Hash;
    use dpp::dashcore::{BlockHash, ChainLock};
    use dpp::version::PlatformVersion;
    use drive::drive::config::DriveConfig;

    use crate::execution::{continue_chain_for_strategy, run_chain_for_strategy};
    use crate::strategy::{
        ChainExecutionOutcome, ChainExecutionParameters, NetworkStrategy, StrategyRandomness,
        UpgradingInfo,
    };
    use drive_abci::config::{ExecutionConfig, PlatformConfig, PlatformTestConfig};
    use drive_abci::platform_types::platform_state::v0::PlatformStateV0Methods;
    use drive_abci::test::helpers::setup::TestPlatformBuilder;
    use platform_version::version::mocks::v2_test::TEST_PROTOCOL_VERSION_2;
    use platform_version::version::mocks::v3_test::TEST_PROTOCOL_VERSION_3;
    use strategy_tests::frequency::Frequency;
    use strategy_tests::Strategy;

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
                        contracts_with_updates: vec![],
                        operations: vec![],
                        start_identities: (0, 0),
                        identities_inserts: Frequency {
                            times_per_block_range: Default::default(),
                            chance_per_block: None,
                        },
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
                    validator_set_quorum_size: 100,
                    validator_set_quorum_type: "llmq_100_67".to_string(),
                    chain_lock_quorum_type: "llmq_100_67".to_string(),
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
                    quorums,
                    current_quorum_hash,
                    current_proposer_versions,
                    end_time_ms,
                    identity_nonce_counter,
                    identity_contract_nonce_counter,
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
                        (counter.get(&1), counter.get(&TEST_PROTOCOL_VERSION_2)),
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
                    quorums,
                    current_quorum_hash,
                    end_time_ms,
                    identity_nonce_counter,
                    identity_contract_nonce_counter,
                    ..
                } = continue_chain_for_strategy(
                    abci_app,
                    ChainExecutionParameters {
                        block_start,
                        core_height_start: 1,
                        block_count: 200,
                        proposers,
                        quorums,
                        current_quorum_hash,
                        current_proposer_versions: Some(current_proposer_versions.clone()),
                        current_identity_nonce_counter: identity_nonce_counter,
                        current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                        start_time_ms: 1681094380000,
                        current_time_ms: end_time_ms,
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
                    assert_eq!(counter.get(&1), None); //no one has proposed 1 yet
                    assert_eq!(counter.get(&TEST_PROTOCOL_VERSION_2), Some(&154));
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
                        quorums,
                        current_quorum_hash,
                        current_proposer_versions: Some(current_proposer_versions),
                        current_identity_nonce_counter: identity_nonce_counter,
                        current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                        start_time_ms: 1681094380000,
                        current_time_ms: end_time_ms,
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
                    assert_eq!(counter.get(&1), None); //no one has proposed 1 yet
                    assert_eq!(counter.get(&TEST_PROTOCOL_VERSION_2), Some(&122));
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
                        contracts_with_updates: vec![],
                        operations: vec![],
                        start_identities: (0, 0),
                        identities_inserts: Frequency {
                            times_per_block_range: Default::default(),
                            chance_per_block: None,
                        },
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
                    validator_set_quorum_size: 30,
                    validator_set_quorum_type: "llmq_100_67".to_string(),
                    chain_lock_quorum_type: "llmq_100_67".to_string(),
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
                    quorums,
                    current_quorum_hash,
                    current_proposer_versions,
                    end_time_ms,
                    identity_nonce_counter,
                    identity_contract_nonce_counter,
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
                    assert_eq!(state.current_protocol_version_in_consensus(), 1);
                    assert_eq!(state.next_epoch_protocol_version(), 1);
                    assert_eq!(
                        (counter.get(&1), counter.get(&TEST_PROTOCOL_VERSION_2)),
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
                    quorums,
                    current_quorum_hash,
                    end_time_ms,
                    identity_nonce_counter,
                    identity_contract_nonce_counter,
                    ..
                } = continue_chain_for_strategy(
                    abci_app,
                    ChainExecutionParameters {
                        block_start,
                        core_height_start: 1,
                        block_count: 2,
                        proposers,
                        quorums,
                        current_quorum_hash,
                        current_proposer_versions: Some(current_proposer_versions.clone()),
                        current_identity_nonce_counter: identity_nonce_counter,
                        current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                        start_time_ms: 1681094380000,
                        current_time_ms: end_time_ms,
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
                    assert_eq!(counter.get(&1), None); //no one has proposed 1 yet
                    assert_eq!(counter.get(&TEST_PROTOCOL_VERSION_2), Some(&1));
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
                        quorums,
                        current_quorum_hash,
                        current_proposer_versions: Some(current_proposer_versions),
                        current_identity_nonce_counter: identity_nonce_counter,
                        current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                        start_time_ms: 1681094380000,
                        current_time_ms: end_time_ms,
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
                    assert_eq!(counter.get(&1), None); //no one has proposed 1 yet
                    assert_eq!(counter.get(&TEST_PROTOCOL_VERSION_2), Some(&1));
                }
            })
            .expect("Failed to create thread with custom stack size");

        // Wait for the thread to finish and assert that it didn't panic.
        handler.join().expect("Thread has panicked");
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
                        contracts_with_updates: vec![],
                        operations: vec![],
                        start_identities: (0, 0),
                        identities_inserts: Frequency {
                            times_per_block_range: Default::default(),
                            chance_per_block: None,
                        },
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
                    validator_set_quorum_size: 40,
                    validator_set_quorum_type: "llmq_100_67".to_string(),
                    chain_lock_quorum_type: "llmq_100_67".to_string(),
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
                    quorums,
                    current_quorum_hash,
                    current_proposer_versions,
                    end_time_ms,
                    identity_nonce_counter,
                    identity_contract_nonce_counter,
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
                        (counter.get(&1), counter.get(&TEST_PROTOCOL_VERSION_2)),
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
                    quorums,
                    current_quorum_hash,

                    end_time_ms,
                    identity_nonce_counter,
                    identity_contract_nonce_counter,
                    ..
                } = continue_chain_for_strategy(
                    abci_app,
                    ChainExecutionParameters {
                        block_start,
                        core_height_start: 1,
                        block_count: 2500,
                        proposers,
                        quorums,
                        current_quorum_hash,
                        current_proposer_versions: Some(current_proposer_versions.clone()),
                        current_identity_nonce_counter: identity_nonce_counter,
                        current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                        start_time_ms: 1681094380000,
                        current_time_ms: end_time_ms,
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
                        (counter.get(&1), counter.get(&TEST_PROTOCOL_VERSION_2)),
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
                        quorums,
                        current_quorum_hash,
                        current_proposer_versions: Some(current_proposer_versions),
                        current_identity_nonce_counter: identity_nonce_counter,
                        current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                        start_time_ms: 1681094380000,
                        current_time_ms: end_time_ms,
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
                        contracts_with_updates: vec![],
                        operations: vec![],
                        start_identities: (0, 0),
                        identities_inserts: Frequency {
                            times_per_block_range: Default::default(),
                            chance_per_block: None,
                        },
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
                    validator_set_quorum_size: 50,
                    validator_set_quorum_type: "llmq_100_67".to_string(),
                    chain_lock_quorum_type: "llmq_100_67".to_string(),
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
                    quorums,
                    current_quorum_hash,
                    current_proposer_versions,
                    end_time_ms,
                    identity_nonce_counter,
                    identity_contract_nonce_counter,
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
                    quorums,
                    current_quorum_hash,

                    end_time_ms,
                    identity_nonce_counter,
                    identity_contract_nonce_counter,
                    ..
                } = continue_chain_for_strategy(
                    abci_app,
                    ChainExecutionParameters {
                        block_start,
                        core_height_start: 1,
                        block_count: 3000,
                        proposers,
                        quorums,
                        current_quorum_hash,
                        current_proposer_versions: Some(current_proposer_versions),
                        current_identity_nonce_counter: identity_nonce_counter,
                        current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                        start_time_ms: 1681094380000,
                        current_time_ms: end_time_ms,
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
                        (counter.get(&1), counter.get(&TEST_PROTOCOL_VERSION_2)),
                        (Some(&18), Some(&111))
                    );
                    //not all nodes have upgraded
                }

                // we are now locked in, the current protocol version will change on next epoch
                // however most nodes now revert

                let strategy = NetworkStrategy {
                    strategy: Strategy {
                        contracts_with_updates: vec![],
                        operations: vec![],
                        start_identities: (0, 0),
                        identities_inserts: Frequency {
                            times_per_block_range: Default::default(),
                            chance_per_block: None,
                        },
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
                    quorums,
                    current_quorum_hash,
                    current_proposer_versions,

                    end_time_ms,
                    identity_nonce_counter,
                    identity_contract_nonce_counter,
                    ..
                } = continue_chain_for_strategy(
                    abci_app,
                    ChainExecutionParameters {
                        block_start,
                        core_height_start: 1,
                        block_count: 2000,
                        proposers,
                        quorums,
                        current_quorum_hash,
                        current_proposer_versions: None, //restart the proposer versions
                        current_identity_nonce_counter: identity_nonce_counter,
                        current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                        start_time_ms: 1681094380000,
                        current_time_ms: end_time_ms,
                    },
                    strategy.clone(),
                    config.clone(),
                    StrategyRandomness::SeedEntropy(40),
                );
                let state = platform.state.load();
                {
                    let counter = &platform.drive.cache.protocol_versions_counter.read();
                    assert_eq!(
                        (counter.get(&1), counter.get(&TEST_PROTOCOL_VERSION_2)),
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
                        quorums,
                        current_quorum_hash,
                        current_proposer_versions: Some(current_proposer_versions),
                        current_identity_nonce_counter: identity_nonce_counter,
                        current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                        start_time_ms: 1681094380000,
                        current_time_ms: end_time_ms,
                    },
                    strategy,
                    config,
                    StrategyRandomness::SeedEntropy(40),
                );
                let state = platform.state.load();
                {
                    let counter = &platform.drive.cache.protocol_versions_counter.read();
                    assert_eq!(
                        (counter.get(&1), counter.get(&TEST_PROTOCOL_VERSION_2)),
                        (Some(&23), Some(&2))
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
                        contracts_with_updates: vec![],
                        operations: vec![],
                        start_identities: (0, 0),
                        identities_inserts: Frequency {
                            times_per_block_range: Default::default(),
                            chance_per_block: None,
                        },
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
                    validator_set_quorum_size: 50,
                    validator_set_quorum_type: "llmq_100_67".to_string(),
                    chain_lock_quorum_type: "llmq_100_67".to_string(),
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
                    quorums,
                    current_quorum_hash,
                    end_time_ms,
                    identity_nonce_counter,
                    identity_contract_nonce_counter,
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
                            counter.get(&1),
                            counter.get(&TEST_PROTOCOL_VERSION_2),
                            counter.get(&TEST_PROTOCOL_VERSION_3)
                        ),
                        (Some(&2), Some(&68), Some(&3))
                    ); //some nodes reverted to previous version
                }

                let strategy = NetworkStrategy {
                    strategy: Strategy {
                        contracts_with_updates: vec![],
                        operations: vec![],
                        start_identities: (0, 0),
                        identities_inserts: Frequency {
                            times_per_block_range: Default::default(),
                            chance_per_block: None,
                        },
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
                        quorums,
                        current_quorum_hash,
                        current_proposer_versions: None,
                        current_identity_nonce_counter: identity_nonce_counter,
                        current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                        start_time_ms: 1681094380000,
                        current_time_ms: end_time_ms,
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
                            counter.get(&1),
                            counter.get(&TEST_PROTOCOL_VERSION_2),
                            counter.get(&TEST_PROTOCOL_VERSION_3)
                        ),
                        (None, Some(&3), Some(&143))
                    );
                }
            })
            .expect("Failed to create thread with custom stack size");

        // Wait for the thread to finish and assert that it didn't panic.
        handler.join().expect("Thread has panicked");
    }
}
