#[cfg(test)]
mod tests {
    use crate::execution::run_chain_for_strategy;
    use crate::strategy::CoreHeightIncrease::RandomCoreHeightIncrease;
    use crate::strategy::{ChainExecutionOutcome, MasternodeListChangesStrategy, NetworkStrategy};
    use dash_platform_macros::stack_size;
    use dpp::block::extended_block_info::v0::ExtendedBlockInfoV0Getters;
    use dpp::dash_to_duffs;
    use dpp::dashcore::hashes::Hash;
    use dpp::dashcore::{BlockHash, ChainLock};
    use dpp::dashcore_rpc::dashcore_rpc_json::QuorumType;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dpp::util::hash::hash_to_hex_string;
    use drive_abci::config::{
        ChainLockConfig, ExecutionConfig, InstantLockConfig, PlatformConfig, PlatformTestConfig,
        ValidatorSetConfig,
    };
    use drive_abci::platform_types::platform_state::PlatformStateV0Methods;
    use drive_abci::test::helpers::setup::TestPlatformBuilder;
    use itertools::Itertools;
    use platform_version::version::PlatformVersion;
    use strategy_tests::frequency::Frequency;
    use strategy_tests::{IdentityInsertInfo, StartIdentities, Strategy};
    #[test]
    fn run_chain_core_height_randomly_increasing() {
        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
                identity_inserts: IdentityInsertInfo::default(),

                identity_contract_nonce_gaps: None,
                signer: None,
            },
            total_hpmns: 100,
            extra_normal_mns: 0,
            validator_quorum_count: 24,
            chain_lock_quorum_count: 24,
            upgrading_info: None,
            core_height_increase: RandomCoreHeightIncrease(Frequency {
                times_per_block_range: 1..3,
                chance_per_block: Some(0.01),
            }),
            proposer_strategy: Default::default(),
            rotate_quorums: false,
            failure_testing: None,
            query_testing: None,
            verify_state_transition_results: true,
            ..Default::default()
        };
        let config = PlatformConfig {
            validator_set: ValidatorSetConfig::default_100_67(),
            chain_lock: ChainLockConfig::default_100_67(),
            instant_lock: InstantLockConfig::default_100_67(),
            execution: ExecutionConfig {
                verify_sum_trees: true,

                ..Default::default()
            },
            block_spacing_ms: 3000,
            testing_configs: PlatformTestConfig::default_minimal_verifications(),
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        run_chain_for_strategy(
            &mut platform,
            1000,
            strategy,
            config,
            15,
            &mut None,
            &mut None,
        );
    }

    #[test]
    fn run_chain_core_height_randomly_increasing_with_epoch_change() {
        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
                identity_inserts: IdentityInsertInfo::default(),

                identity_contract_nonce_gaps: None,
                signer: None,
            },
            total_hpmns: 100,
            extra_normal_mns: 0,
            validator_quorum_count: 24,
            chain_lock_quorum_count: 24,
            upgrading_info: None,
            core_height_increase: RandomCoreHeightIncrease(Frequency {
                times_per_block_range: 1..3,
                chance_per_block: Some(0.5),
            }),
            proposer_strategy: Default::default(),
            rotate_quorums: false,
            failure_testing: None,
            query_testing: None,
            verify_state_transition_results: true,
            ..Default::default()
        };
        let hour_in_ms = 1000 * 60 * 60;
        let config = PlatformConfig {
            validator_set: ValidatorSetConfig::default_100_67(),
            chain_lock: ChainLockConfig::default_100_67(),
            instant_lock: InstantLockConfig::default_100_67(),
            execution: ExecutionConfig {
                verify_sum_trees: true,

                ..Default::default()
            },
            block_spacing_ms: hour_in_ms,
            testing_configs: PlatformTestConfig::default_minimal_verifications(),
            ..Default::default()
        };

        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let outcome = run_chain_for_strategy(
            &mut platform,
            1000,
            strategy,
            config,
            15,
            &mut None,
            &mut None,
        );
        assert_eq!(outcome.masternode_identity_balances.len(), 100);
        let all_have_balances = outcome
            .masternode_identity_balances
            .iter()
            .all(|(_, balance)| *balance != 0);
        assert!(all_have_balances, "all masternodes should have a balance");
    }

    #[test]
    #[stack_size(4 * 1024 * 1024)]
    fn run_chain_core_height_randomly_increasing_with_quick_epoch_change() {
        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
                identity_inserts: IdentityInsertInfo::default(),

                identity_contract_nonce_gaps: None,
                signer: None,
            },
            total_hpmns: 100,
            extra_normal_mns: 0,
            validator_quorum_count: 24,
            chain_lock_quorum_count: 24,
            upgrading_info: None,
            core_height_increase: RandomCoreHeightIncrease(Frequency {
                times_per_block_range: 1..3,
                chance_per_block: Some(0.5),
            }),
            proposer_strategy: Default::default(),
            rotate_quorums: false,
            failure_testing: None,
            query_testing: None,
            verify_state_transition_results: true,
            ..Default::default()
        };
        let hour_in_s = 60 * 60;
        let three_mins_in_ms = 1000 * 60 * 3;
        let config = PlatformConfig {
            validator_set: ValidatorSetConfig::default_100_67(),
            chain_lock: ChainLockConfig::default_100_67(),
            instant_lock: InstantLockConfig::default_100_67(),
            execution: ExecutionConfig {
                verify_sum_trees: true,

                epoch_time_length_s: hour_in_s,
                ..Default::default()
            },
            block_spacing_ms: three_mins_in_ms,
            testing_configs: PlatformTestConfig::default_minimal_verifications(),
            ..Default::default()
        };

        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let outcome = run_chain_for_strategy(
            &mut platform,
            1000,
            strategy,
            config,
            15,
            &mut None,
            &mut None,
        );
        assert_eq!(outcome.masternode_identity_balances.len(), 100);
        let all_have_balances = outcome
            .masternode_identity_balances
            .iter()
            .all(|(_, balance)| *balance != 0);
        assert!(all_have_balances, "all masternodes should have a balance");
        // 49 makes sense because we have about 20 blocks per epoch, and 1000/20 = 50 (but we didn't go over so we should be at 49)
        assert_eq!(outcome.end_epoch_index, 49);
    }

    #[test]
    fn run_chain_core_height_randomly_increasing_with_quorum_updates() {
        let platform_version = PlatformVersion::latest();
        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
                identity_inserts: IdentityInsertInfo::default(),
                identity_contract_nonce_gaps: None,
                signer: None,
            },
            total_hpmns: 500,
            extra_normal_mns: 0,
            validator_quorum_count: 24,
            chain_lock_quorum_count: 24,
            upgrading_info: None,
            core_height_increase: RandomCoreHeightIncrease(Frequency {
                times_per_block_range: 5..6,
                chance_per_block: Some(0.5),
            }),
            proposer_strategy: Default::default(),
            rotate_quorums: true,
            failure_testing: None,
            query_testing: None,
            verify_state_transition_results: true,
            ..Default::default()
        };
        let config = PlatformConfig {
            validator_set: ValidatorSetConfig {
                quorum_type: QuorumType::Llmq100_67,
                quorum_size: 10,
                ..Default::default()
            },
            chain_lock: ChainLockConfig {
                quorum_type: QuorumType::Llmq100_67,
                quorum_size: 10,
                quorum_window: 24,
                quorum_active_signers: 24,
                quorum_rotation: false,
            },
            instant_lock: InstantLockConfig {
                quorum_type: QuorumType::Llmq100_67,
                quorum_size: 10,
                quorum_window: 24,
                quorum_active_signers: 24,
                quorum_rotation: false,
            },
            execution: ExecutionConfig {
                verify_sum_trees: true,

                ..Default::default()
            },
            block_spacing_ms: 300,
            testing_configs: PlatformTestConfig::default_minimal_verifications(),
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let ChainExecutionOutcome { abci_app, .. } = run_chain_for_strategy(
            &mut platform,
            2000,
            strategy,
            config,
            40,
            &mut None,
            &mut None,
        );

        // With these params if we didn't rotate we would have at most 240
        // of the 500 hpmns that could get paid, however we are expecting that most
        // will be able to propose a block (and then get paid later on).

        let platform = abci_app.platform;
        let counter = &platform.drive.cache.protocol_versions_counter.read();
        platform
            .drive
            .fetch_versions_with_counter(None, &platform_version.drive)
            .expect("expected to get versions");

        let state = abci_app.platform.state.load();

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
        assert!(
            counter
                .get(&platform_version.protocol_version)
                .unwrap()
                .unwrap()
                > &240
        );
    }

    #[test]
    fn run_chain_core_height_randomly_increasing_with_quorum_updates_new_proposers() {
        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
                identity_inserts: IdentityInsertInfo::default(),

                identity_contract_nonce_gaps: None,
                signer: None,
            },
            total_hpmns: 100,
            extra_normal_mns: 0,
            validator_quorum_count: 24,
            chain_lock_quorum_count: 24,
            upgrading_info: None,
            core_height_increase: RandomCoreHeightIncrease(Frequency {
                times_per_block_range: 1..2,
                chance_per_block: Some(0.2),
            }),
            proposer_strategy: MasternodeListChangesStrategy {
                new_hpmns: Frequency {
                    times_per_block_range: 1..3,
                    chance_per_block: Some(0.5),
                },
                ..Default::default()
            },
            rotate_quorums: true,
            failure_testing: None,
            query_testing: None,
            verify_state_transition_results: true,
            ..Default::default()
        };
        let config = PlatformConfig {
            validator_set: ValidatorSetConfig {
                quorum_type: QuorumType::Llmq100_67,
                quorum_size: 10,
                ..Default::default()
            },
            chain_lock: ChainLockConfig {
                quorum_type: QuorumType::Llmq100_67,
                quorum_size: 10,
                quorum_window: 24,
                quorum_active_signers: 24,
                quorum_rotation: false,
            },
            instant_lock: InstantLockConfig {
                quorum_type: QuorumType::Llmq100_67,
                quorum_size: 10,
                quorum_window: 24,
                quorum_active_signers: 24,
                quorum_rotation: false,
            },
            execution: ExecutionConfig {
                verify_sum_trees: true,

                ..Default::default()
            },
            block_spacing_ms: 300,
            testing_configs: PlatformTestConfig::default_minimal_verifications(),
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let ChainExecutionOutcome { abci_app, .. } = run_chain_for_strategy(
            &mut platform,
            300,
            strategy,
            config,
            43,
            &mut None,
            &mut None,
        );

        // With these params if we add new mns the hpmn masternode list would be 100, but we
        // can expect it to be much higher.

        let platform = abci_app.platform;
        let platform_state = platform.state.load();

        assert!(platform_state.hpmn_masternode_list().len() > 100);
    }

    #[test]
    fn run_chain_core_height_randomly_increasing_with_quorum_updates_changing_proposers() {
        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
                identity_inserts: IdentityInsertInfo::default(),

                identity_contract_nonce_gaps: None,
                signer: None,
            },
            total_hpmns: 100,
            extra_normal_mns: 0,
            validator_quorum_count: 24,
            chain_lock_quorum_count: 24,
            upgrading_info: None,
            core_height_increase: RandomCoreHeightIncrease(Frequency {
                times_per_block_range: 1..2,
                chance_per_block: Some(0.2),
            }),
            proposer_strategy: MasternodeListChangesStrategy {
                new_hpmns: Frequency {
                    times_per_block_range: 1..3,
                    chance_per_block: Some(0.5),
                },
                removed_hpmns: Frequency {
                    times_per_block_range: 1..3,
                    chance_per_block: Some(0.5),
                },
                ..Default::default()
            },
            rotate_quorums: true,
            failure_testing: None,
            query_testing: None,
            verify_state_transition_results: true,
            ..Default::default()
        };
        let config = PlatformConfig {
            validator_set: ValidatorSetConfig {
                quorum_type: QuorumType::Llmq100_67,
                quorum_size: 10,
                ..Default::default()
            },
            chain_lock: ChainLockConfig {
                quorum_type: QuorumType::Llmq100_67,
                quorum_size: 10,
                quorum_window: 24,
                quorum_active_signers: 24,
                quorum_rotation: false,
            },
            instant_lock: InstantLockConfig {
                quorum_type: QuorumType::Llmq100_67,
                quorum_size: 10,
                quorum_window: 24,
                quorum_active_signers: 24,
                quorum_rotation: false,
            },
            execution: ExecutionConfig {
                verify_sum_trees: true,

                ..Default::default()
            },
            block_spacing_ms: 300,
            testing_configs: PlatformTestConfig::default_minimal_verifications(),
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let ChainExecutionOutcome { abci_app, .. } = run_chain_for_strategy(
            &mut platform,
            300,
            strategy,
            config,
            43,
            &mut None,
            &mut None,
        );

        // With these params if we add new mns the hpmn masternode list would be randomly different from 100.

        let platform = abci_app.platform;
        let platform_state = platform.state.load();

        assert_ne!(platform_state.hpmn_masternode_list().len(), 100);
    }

    #[test]
    fn run_chain_core_height_randomly_increasing_with_quorum_updates_updating_proposers() {
        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
                identity_inserts: IdentityInsertInfo::default(),

                identity_contract_nonce_gaps: None,
                signer: None,
            },
            total_hpmns: 100,
            extra_normal_mns: 0,
            validator_quorum_count: 24,
            chain_lock_quorum_count: 24,
            upgrading_info: None,
            core_height_increase: RandomCoreHeightIncrease(Frequency {
                times_per_block_range: 1..2,
                chance_per_block: Some(0.2),
            }),
            proposer_strategy: MasternodeListChangesStrategy {
                updated_hpmns: Frequency {
                    times_per_block_range: 1..3,
                    chance_per_block: Some(0.5),
                },
                ..Default::default()
            },
            rotate_quorums: true,
            failure_testing: None,
            query_testing: None,
            verify_state_transition_results: true,
            ..Default::default()
        };
        let config = PlatformConfig {
            validator_set: ValidatorSetConfig {
                quorum_type: QuorumType::Llmq100_67,
                quorum_size: 10,
                ..Default::default()
            },
            chain_lock: ChainLockConfig {
                quorum_type: QuorumType::Llmq100_67,
                quorum_size: 10,
                quorum_window: 24,
                quorum_active_signers: 24,
                quorum_rotation: false,
            },
            instant_lock: InstantLockConfig {
                quorum_type: QuorumType::Llmq100_67,
                quorum_size: 10,
                quorum_window: 24,
                quorum_active_signers: 24,
                quorum_rotation: false,
            },
            execution: ExecutionConfig {
                verify_sum_trees: true,

                ..Default::default()
            },
            block_spacing_ms: 300,
            testing_configs: PlatformTestConfig::default_minimal_verifications(),
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let ChainExecutionOutcome {
            abci_app,
            proposers,
            ..
        } = run_chain_for_strategy(
            &mut platform,
            300,
            strategy,
            config,
            43,
            &mut None,
            &mut None,
        );

        // With these params if we add new mns the hpmn masternode list would be randomly different from 100.

        let platform_version = PlatformVersion::latest();
        let platform = abci_app.platform;
        let _platform_state = platform.state.load();

        // We need to find if any masternode has ever had their keys disabled.

        let hpmns = platform
            .drive
            .fetch_full_identities(
                proposers
                    .into_iter()
                    .map(|proposer| proposer.masternode.pro_tx_hash.to_byte_array())
                    .collect::<Vec<_>>()
                    .as_slice(),
                None,
                platform_version,
            )
            .expect("expected to fetch identities");

        let has_disabled_keys = hpmns.values().any(|identity| {
            identity
                .as_ref()
                .map(|identity| {
                    identity
                        .public_keys()
                        .values()
                        .any(|key| key.disabled_at().is_some())
                })
                .unwrap_or_default()
        });
        assert!(has_disabled_keys);
    }

    #[test]
    fn run_chain_rotation_is_deterministic_1_block() {
        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
                identity_inserts: IdentityInsertInfo {
                    //we do this to create some paying transactions
                    frequency: Frequency {
                        times_per_block_range: 1..2,
                        chance_per_block: None,
                    },
                    start_keys: 5,
                    extra_keys: Default::default(),
                    start_balance_range: dash_to_duffs!(1)..=dash_to_duffs!(1),
                },

                identity_contract_nonce_gaps: None,
                signer: None,
            },
            total_hpmns: 50,
            extra_normal_mns: 0,
            validator_quorum_count: 10,
            upgrading_info: None,
            core_height_increase: RandomCoreHeightIncrease(Frequency {
                times_per_block_range: 1..2,
                chance_per_block: None,
            }),
            proposer_strategy: Default::default(),
            rotate_quorums: true,
            failure_testing: None,
            query_testing: None,
            verify_state_transition_results: false,
            ..Default::default()
        };
        let day_in_ms = 1000 * 60 * 60 * 24;
        let config = PlatformConfig {
            validator_set: ValidatorSetConfig {
                quorum_type: QuorumType::Llmq100_67,
                quorum_size: 3,
                ..Default::default()
            },
            chain_lock: ChainLockConfig {
                quorum_type: QuorumType::Llmq100_67,
                quorum_size: 3,
                quorum_window: 24,
                quorum_active_signers: 24,
                quorum_rotation: false,
            },
            instant_lock: InstantLockConfig {
                quorum_type: QuorumType::Llmq100_67,
                quorum_size: 3,
                quorum_window: 24,
                quorum_active_signers: 24,
                quorum_rotation: false,
            },
            execution: ExecutionConfig {
                verify_sum_trees: true,
                ..Default::default()
            },
            block_spacing_ms: day_in_ms,
            testing_configs: PlatformTestConfig::default_minimal_verifications(),
            ..Default::default()
        };

        let mut platforms = Vec::new();
        let mut outcomes = Vec::new();

        for _ in 0..2 {
            let platform = TestPlatformBuilder::new()
                .with_config(config.clone())
                .build_with_mock_rpc();
            platforms.push(platform);
        }

        for platform in &mut platforms {
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

            let outcome = run_chain_for_strategy(
                platform,
                1,
                strategy.clone(),
                config.clone(),
                7,
                &mut None,
                &mut None,
            );
            outcomes.push(outcome);
        }

        let first_proposers_fingerprint = hash_to_hex_string(
            outcomes[0]
                .proposers
                .iter()
                .map(|masternode_list_item_with_updates| {
                    hex::encode(masternode_list_item_with_updates.masternode.pro_tx_hash)
                })
                .join("|"),
        );

        assert!(outcomes.iter().all(|outcome| {
            let last_proposers_fingerprint = hash_to_hex_string(
                outcome
                    .proposers
                    .iter()
                    .map(|masternode_list_item_with_updates| {
                        hex::encode(masternode_list_item_with_updates.masternode.pro_tx_hash)
                    })
                    .join("|"),
            );

            first_proposers_fingerprint == last_proposers_fingerprint
        }));

        let first_masternodes_fingerprint = hash_to_hex_string(
            outcomes[0]
                .masternode_identity_balances
                .keys()
                .map(hex::encode)
                .join("|"),
        );

        assert!(outcomes.iter().all(|outcome| {
            let last_masternodes_fingerprint = hash_to_hex_string(
                outcome
                    .masternode_identity_balances
                    .keys()
                    .map(hex::encode)
                    .join("|"),
            );

            first_masternodes_fingerprint == last_masternodes_fingerprint
        }));

        let first_validator_set_fingerprint = hash_to_hex_string(
            outcomes[0]
                .current_quorum()
                .validator_set
                .iter()
                .map(|validator| hex::encode(validator.pro_tx_hash))
                .join("|"),
        );

        assert!(outcomes.iter().all(|outcome| {
            let last_validator_set_fingerprint = hash_to_hex_string(
                outcome
                    .current_quorum()
                    .validator_set
                    .iter()
                    .map(|validator| hex::encode(validator.pro_tx_hash))
                    .join("|"),
            );

            first_validator_set_fingerprint == last_validator_set_fingerprint
        }));

        let state = outcomes[0].abci_app.platform.state.load();
        let protocol_version = state.current_protocol_version_in_consensus();
        let platform_version =
            PlatformVersion::get(protocol_version).expect("expected platform version");

        let first_last_app_hash = outcomes[0]
            .abci_app
            .platform
            .drive
            .grove
            .root_hash(None, &platform_version.drive.grove_version)
            .unwrap()
            .expect("should return app hash");

        assert!(outcomes.iter().all(|outcome| {
            let last_app_hash = outcome
                .abci_app
                .platform
                .drive
                .grove
                .root_hash(None, &platform_version.drive.grove_version)
                .unwrap()
                .expect("should return app hash");

            last_app_hash == first_last_app_hash
        }));
    }

    #[test]
    fn run_chain_heavy_rotation_deterministic_before_payout() {
        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
                identity_inserts: IdentityInsertInfo {
                    //we do this to create some paying transactions
                    frequency: Frequency {
                        times_per_block_range: 1..2,
                        chance_per_block: None,
                    },
                    start_keys: 5,
                    extra_keys: Default::default(),
                    start_balance_range: dash_to_duffs!(1)..=dash_to_duffs!(1),
                },

                identity_contract_nonce_gaps: None,
                signer: None,
            },
            total_hpmns: 500,
            extra_normal_mns: 0,
            validator_quorum_count: 100,
            upgrading_info: None,
            core_height_increase: RandomCoreHeightIncrease(Frequency {
                times_per_block_range: 1..2,
                chance_per_block: None,
            }),
            proposer_strategy: Default::default(),
            rotate_quorums: true,
            failure_testing: None,
            query_testing: None,
            verify_state_transition_results: false,
            ..Default::default()
        };
        let day_in_ms = 1000 * 60 * 60 * 24;
        let config = PlatformConfig {
            validator_set: ValidatorSetConfig {
                quorum_type: QuorumType::Llmq100_67,
                quorum_size: 3,
                ..Default::default()
            },
            chain_lock: ChainLockConfig {
                quorum_type: QuorumType::Llmq100_67,
                quorum_size: 3,
                quorum_window: 24,
                quorum_active_signers: 24,
                quorum_rotation: false,
            },
            instant_lock: InstantLockConfig {
                quorum_type: QuorumType::Llmq100_67,
                quorum_size: 3,
                quorum_window: 24,
                quorum_active_signers: 24,
                quorum_rotation: false,
            },
            execution: ExecutionConfig {
                verify_sum_trees: true,
                epoch_time_length_s: 1576800,
                ..Default::default()
            },
            block_spacing_ms: day_in_ms,
            testing_configs: PlatformTestConfig::default_minimal_verifications(),
            ..Default::default()
        };
        let mut platform_a = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();
        let mut platform_b = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();
        platform_a
            .core_rpc
            .expect_get_best_chain_lock()
            .returning(move || {
                Ok(ChainLock {
                    block_height: 10,
                    block_hash: BlockHash::from_byte_array([1; 32]),
                    signature: [2; 96].into(),
                })
            });
        platform_b
            .core_rpc
            .expect_get_best_chain_lock()
            .returning(move || {
                Ok(ChainLock {
                    block_height: 10,
                    block_hash: BlockHash::from_byte_array([1; 32]),
                    signature: [2; 96].into(),
                })
            });

        let outcome_a = run_chain_for_strategy(
            &mut platform_a,
            18,
            strategy.clone(),
            config.clone(),
            7,
            &mut None,
            &mut None,
        );
        let outcome_b = run_chain_for_strategy(
            &mut platform_b,
            18,
            strategy,
            config,
            7,
            &mut None,
            &mut None,
        );
        assert_eq!(outcome_a.end_epoch_index, outcome_b.end_epoch_index); // 100/18
        assert_eq!(outcome_a.masternode_identity_balances.len(), 500); // 500 nodes
        assert_eq!(outcome_b.masternode_identity_balances.len(), 500); // 500 nodes
        assert_eq!(outcome_a.end_epoch_index, 0); // 100/18
        let masternodes_fingerprint_a = hash_to_hex_string(
            outcome_a
                .masternode_identity_balances
                .keys()
                .map(hex::encode)
                .join("|"),
        );
        assert_eq!(
            masternodes_fingerprint_a,
            "0154fd29f0062819ee6b8063ea02c9f3296ed9af33a4538ae98087edb1a75029".to_string()
        );
        let masternodes_fingerprint_b = hash_to_hex_string(
            outcome_b
                .masternode_identity_balances
                .keys()
                .map(hex::encode)
                .join("|"),
        );
        assert_eq!(
            masternodes_fingerprint_b,
            "0154fd29f0062819ee6b8063ea02c9f3296ed9af33a4538ae98087edb1a75029".to_string()
        );

        let state = outcome_a.abci_app.platform.state.load();
        let protocol_version = state.current_protocol_version_in_consensus();
        let platform_version =
            PlatformVersion::get(protocol_version).expect("expected platform version");

        let last_app_hash_a = outcome_a
            .abci_app
            .platform
            .drive
            .grove
            .root_hash(None, &platform_version.drive.grove_version)
            .unwrap()
            .expect("should return app hash");

        let last_app_hash_b = outcome_b
            .abci_app
            .platform
            .drive
            .grove
            .root_hash(None, &platform_version.drive.grove_version)
            .unwrap()
            .expect("should return app hash");

        assert_eq!(last_app_hash_a, last_app_hash_b);

        let balance_count = outcome_a
            .masternode_identity_balances
            .into_iter()
            .filter(|(_, balance)| *balance != 0)
            .count();
        assert_eq!(balance_count, 0);
    }

    #[test]
    fn run_chain_proposer_proposes_a_chainlock_that_would_remove_themselves_from_the_list_deterministic(
    ) {
        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
                identity_inserts: IdentityInsertInfo {
                    //we do this to create some paying transactions
                    frequency: Frequency {
                        times_per_block_range: 1..2,
                        chance_per_block: None,
                    },
                    start_keys: 5,
                    extra_keys: Default::default(),
                    start_balance_range: dash_to_duffs!(1)..=dash_to_duffs!(1),
                },

                identity_contract_nonce_gaps: None,
                signer: None,
            },
            total_hpmns: 500,
            extra_normal_mns: 0,
            validator_quorum_count: 100,
            upgrading_info: None,
            core_height_increase: RandomCoreHeightIncrease(Frequency {
                times_per_block_range: 1..2,
                chance_per_block: None,
            }),
            proposer_strategy: Default::default(),
            rotate_quorums: true,
            failure_testing: None,
            query_testing: None,
            verify_state_transition_results: false,
            ..Default::default()
        };
        let day_in_ms = 1000 * 60 * 60 * 24;
        let config = PlatformConfig {
            validator_set: ValidatorSetConfig {
                quorum_type: QuorumType::Llmq100_67,
                quorum_size: 3,
                ..Default::default()
            },
            chain_lock: ChainLockConfig {
                quorum_type: QuorumType::Llmq100_67,
                quorum_size: 3,
                quorum_window: 24,
                quorum_active_signers: 24,
                quorum_rotation: false,
            },
            instant_lock: InstantLockConfig {
                quorum_type: QuorumType::Llmq100_67,
                quorum_size: 3,
                quorum_window: 24,
                quorum_active_signers: 24,
                quorum_rotation: false,
            },
            execution: ExecutionConfig {
                verify_sum_trees: true,
                ..Default::default()
            },
            block_spacing_ms: day_in_ms,
            testing_configs: PlatformTestConfig::default_minimal_verifications(),
            ..Default::default()
        };
        let mut platform_a = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();
        let mut platform_b = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();
        platform_a
            .core_rpc
            .expect_get_best_chain_lock()
            .returning(move || {
                Ok(ChainLock {
                    block_height: 10,
                    block_hash: BlockHash::from_byte_array([1; 32]),
                    signature: [2; 96].into(),
                })
            });
        platform_b
            .core_rpc
            .expect_get_best_chain_lock()
            .returning(move || {
                Ok(ChainLock {
                    block_height: 10,
                    block_hash: BlockHash::from_byte_array([1; 32]),
                    signature: [2; 96].into(),
                })
            });

        let outcome_a = run_chain_for_strategy(
            &mut platform_a,
            100,
            strategy.clone(),
            config.clone(),
            7,
            &mut None,
            &mut None,
        );
        let outcome_b = run_chain_for_strategy(
            &mut platform_b,
            100,
            strategy,
            config,
            7,
            &mut None,
            &mut None,
        );
        assert_eq!(outcome_a.end_epoch_index, outcome_b.end_epoch_index); // 100/18
        assert_eq!(outcome_a.masternode_identity_balances.len(), 500); // 500 nodes
        assert_eq!(outcome_b.masternode_identity_balances.len(), 500); // 500 nodes
                                                                       //assert_eq!(outcome_a.end_epoch_index, 1); // 100/18
        let masternodes_fingerprint_a = hash_to_hex_string(
            outcome_a
                .masternode_identity_balances
                .keys()
                .map(hex::encode)
                .join("|"),
        );
        assert_eq!(
            masternodes_fingerprint_a,
            "0154fd29f0062819ee6b8063ea02c9f3296ed9af33a4538ae98087edb1a75029".to_string()
        );
        let masternodes_fingerprint_b = hash_to_hex_string(
            outcome_b
                .masternode_identity_balances
                .keys()
                .map(hex::encode)
                .join("|"),
        );
        assert_eq!(
            masternodes_fingerprint_b,
            "0154fd29f0062819ee6b8063ea02c9f3296ed9af33a4538ae98087edb1a75029".to_string()
        );

        let state = outcome_a.abci_app.platform.state.load();
        let protocol_version = state.current_protocol_version_in_consensus();
        let platform_version =
            PlatformVersion::get(protocol_version).expect("expected platform version");

        let last_app_hash_a = outcome_a
            .abci_app
            .platform
            .drive
            .grove
            .root_hash(None, &platform_version.drive.grove_version)
            .unwrap()
            .expect("should return app hash");

        let last_app_hash_b = outcome_b
            .abci_app
            .platform
            .drive
            .grove
            .root_hash(None, &platform_version.drive.grove_version)
            .unwrap()
            .expect("should return app hash");

        assert_eq!(last_app_hash_a, last_app_hash_b);

        let balance_count = outcome_a
            .masternode_identity_balances
            .into_iter()
            .filter(|(_, balance)| *balance != 0)
            .count();
        // we have a maximum 90 quorums, that could have been used, 7 were used twice
        assert_eq!(balance_count, 83);
    }
}
