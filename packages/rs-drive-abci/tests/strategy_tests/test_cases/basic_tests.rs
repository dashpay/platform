#[cfg(test)]
mod tests {
    use crate::execution::{continue_chain_for_strategy, run_chain_for_strategy};
    use crate::strategy::{
        ChainExecutionOutcome, ChainExecutionParameters, FailureStrategy, NetworkStrategy,
        StrategyRandomness,
    };
    use dpp::block::extended_block_info::v0::ExtendedBlockInfoV0Getters;
    use dpp::dashcore_rpc::json::QuorumType;
    use std::collections::BTreeMap;
    use strategy_tests::{IdentityInsertInfo, StartIdentities, Strategy};

    use drive_abci::config::{
        ChainLockConfig, ExecutionConfig, InstantLockConfig, PlatformConfig, PlatformTestConfig,
        ValidatorSetConfig,
    };

    use drive_abci::logging::LogLevel;
    use drive_abci::platform_types::platform_state::PlatformStateV0Methods;
    use itertools::Itertools;
    use tenderdash_abci::proto::abci::{RequestInfo, ResponseInfo};

    use crate::addresses_with_balance::AddressesWithBalance;
    use dpp::dash_to_duffs;
    use drive_abci::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
    use platform_version::version::PlatformVersion;
    use strategy_tests::frequency::Frequency;
    use tenderdash_abci::Application;

    #[test]
    fn run_chain_nothing_happening() {
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
            proposer_strategy: Default::default(),
            rotate_quorums: false,
            failure_testing: None,
            query_testing: None,
            verify_state_transition_results: false,
            ..Default::default()
        };
        let config = PlatformConfig {
            validator_set: ValidatorSetConfig::default_100_67(),
            chain_lock: ChainLockConfig::default_100_67(),
            instant_lock: InstantLockConfig::default_100_67(),
            execution: ExecutionConfig {
                verify_sum_trees: true,

                ..ExecutionConfig::default()
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
            100,
            strategy,
            config,
            15,
            &mut None,
            &mut None,
        );
    }

    #[test]
    fn run_chain_block_signing() {
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

            proposer_strategy: Default::default(),
            rotate_quorums: false,
            failure_testing: None,
            query_testing: None,
            verify_state_transition_results: false,
            ..Default::default()
        };
        let config = PlatformConfig {
            validator_set: ValidatorSetConfig::default_100_67(),
            chain_lock: ChainLockConfig::default_100_67(),
            instant_lock: InstantLockConfig::default_100_67(),
            execution: ExecutionConfig {
                verify_sum_trees: true,

                ..ExecutionConfig::default()
            },
            block_spacing_ms: 3000,
            testing_configs: PlatformTestConfig::default(),
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        run_chain_for_strategy(
            &mut platform,
            50,
            strategy,
            config,
            13,
            &mut None,
            &mut None,
        );
    }

    #[test]
    fn run_chain_stop_and_restart() {
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

            proposer_strategy: Default::default(),
            rotate_quorums: false,
            failure_testing: None,
            query_testing: None,
            verify_state_transition_results: false,
            ..Default::default()
        };
        let config = PlatformConfig {
            validator_set: ValidatorSetConfig::default_100_67(),
            chain_lock: ChainLockConfig::default_100_67(),
            instant_lock: InstantLockConfig::default_100_67(),
            execution: ExecutionConfig {
                verify_sum_trees: true,

                ..ExecutionConfig::default()
            },
            block_spacing_ms: 3000,
            testing_configs: PlatformTestConfig::default(),
            ..Default::default()
        };
        let TempPlatform {
            mut platform,
            tempdir: _,
        } = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

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
            15,
            strategy.clone(),
            config.clone(),
            40,
            &mut None,
            &mut None,
        );

        let state = abci_app.platform.state.load();

        let protocol_version = state.current_protocol_version_in_consensus();

        let platform_version =
            PlatformVersion::get(protocol_version).expect("expected platform version");

        let known_root_hash = abci_app
            .platform
            .drive
            .grove
            .root_hash(None, &platform_version.drive.grove_version)
            .unwrap()
            .expect("expected root hash");

        abci_app
            .platform
            .reload_state_from_storage(platform_version)
            .expect("expected to recreate state");

        let ResponseInfo {
            data: _,
            version: _,
            app_version: _,
            last_block_height,
            last_block_app_hash,
        } = abci_app
            .info(RequestInfo {
                version: tenderdash_abci::proto::meta::TENDERDASH_VERSION.to_string(),
                block_version: 0,
                p2p_version: 0,
                abci_version: tenderdash_abci::proto::meta::ABCI_VERSION.to_string(),
            })
            .expect("expected to call info");

        assert_eq!(last_block_height, 15);
        assert_eq!(last_block_app_hash, known_root_hash);

        let state = abci_app.platform.state.load();

        let block_start = state
            .last_committed_block_info()
            .as_ref()
            .unwrap()
            .basic_info()
            .height
            + 1;

        continue_chain_for_strategy(
            abci_app,
            ChainExecutionParameters {
                block_start,
                core_height_start: 1,
                block_count: 30,
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
            StrategyRandomness::SeedEntropy(7),
        );
    }

    #[test]
    fn run_chain_stop_and_restart_multiround() {
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

            proposer_strategy: Default::default(),
            rotate_quorums: false,
            failure_testing: Some(FailureStrategy {
                deterministic_start_seed: None,
                dont_finalize_block: false,
                expect_every_block_errors_with_codes: vec![],
                expect_specific_block_errors_with_codes: Default::default(),
                rounds_before_successful_block: Some(5),
            }),
            query_testing: None,
            verify_state_transition_results: false,
            ..Default::default()
        };
        let config = PlatformConfig {
            validator_set: ValidatorSetConfig::default_100_67(),
            chain_lock: ChainLockConfig::default_100_67(),
            instant_lock: InstantLockConfig::default_100_67(),
            execution: ExecutionConfig {
                verify_sum_trees: true,

                ..ExecutionConfig::default()
            },
            block_spacing_ms: 3000,
            testing_configs: PlatformTestConfig::default(),
            ..Default::default()
        };
        let TempPlatform {
            mut platform,
            tempdir: _,
        } = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

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
            15,
            strategy.clone(),
            config.clone(),
            40,
            &mut None,
            &mut None,
        );

        let state = abci_app.platform.state.load();

        let protocol_version = state.current_protocol_version_in_consensus();

        let platform_version =
            PlatformVersion::get(protocol_version).expect("expected platform version");

        let known_root_hash = abci_app
            .platform
            .drive
            .grove
            .root_hash(None, &platform_version.drive.grove_version)
            .unwrap()
            .expect("expected root hash");

        abci_app
            .platform
            .reload_state_from_storage(platform_version)
            .expect("expected to recreate state");

        let ResponseInfo {
            data: _,
            version: _,
            app_version: _,
            last_block_height,
            last_block_app_hash,
        } = abci_app
            .info(RequestInfo {
                version: tenderdash_abci::proto::meta::TENDERDASH_VERSION.to_string(),
                block_version: 0,
                p2p_version: 0,
                abci_version: tenderdash_abci::proto::meta::ABCI_VERSION.to_string(),
            })
            .expect("expected to call info");

        assert_eq!(last_block_height, 15);
        assert_eq!(last_block_app_hash, known_root_hash);

        let state = abci_app.platform.state.load();

        let block_start = state
            .last_committed_block_info()
            .as_ref()
            .unwrap()
            .basic_info()
            .height
            + 1;

        continue_chain_for_strategy(
            abci_app,
            ChainExecutionParameters {
                block_start,
                core_height_start: 1,
                block_count: 30,
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
            StrategyRandomness::SeedEntropy(7),
        );
    }

    #[test]
    fn run_chain_stop_and_restart_with_rotation() {
        drive_abci::logging::init_for_tests(LogLevel::Silent);

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
            validator_quorum_count: 100,
            upgrading_info: None,

            proposer_strategy: Default::default(),
            rotate_quorums: false,
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
        let TempPlatform {
            mut platform,
            tempdir: _,
        } = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let ChainExecutionOutcome {
            abci_app,
            proposers,
            validator_quorums: quorums,
            current_validator_quorum_hash: current_quorum_hash,
            current_proposer_versions,
            end_time_ms,
            identity_nonce_counter,
            identity_contract_nonce_counter,
            ..
        } = run_chain_for_strategy(
            &mut platform,
            100,
            strategy.clone(),
            config.clone(),
            89,
            &mut None,
            &mut None,
        );

        let state = abci_app.platform.state.load();
        let protocol_version = state.current_protocol_version_in_consensus();

        let platform_version = PlatformVersion::get(protocol_version).unwrap();

        let known_root_hash = abci_app
            .platform
            .drive
            .grove
            .root_hash(None, &platform_version.drive.grove_version)
            .unwrap()
            .expect("expected root hash");

        abci_app
            .platform
            .reload_state_from_storage(platform_version)
            .expect("expected to recreate state");

        let ResponseInfo {
            data: _,
            version: _,
            app_version: _,
            last_block_height,
            last_block_app_hash,
        } = abci_app
            .info(RequestInfo {
                version: tenderdash_abci::proto::meta::TENDERDASH_VERSION.to_string(),
                block_version: 0,
                p2p_version: 0,
                abci_version: tenderdash_abci::proto::meta::ABCI_VERSION.to_string(),
            })
            .expect("expected to call info");

        assert_eq!(last_block_height, 100);
        assert_eq!(last_block_app_hash, known_root_hash);

        let state = abci_app.platform.state.load();

        let block_start = state
            .last_committed_block_info()
            .as_ref()
            .unwrap()
            .basic_info()
            .height
            + 1;

        continue_chain_for_strategy(
            abci_app,
            ChainExecutionParameters {
                block_start,
                core_height_start: 10,
                block_count: 30,
                proposers,
                validator_quorums: quorums,
                current_validator_quorum_hash: current_quorum_hash,
                instant_lock_quorums: Default::default(),
                current_proposer_versions: Some(current_proposer_versions),
                current_identity_nonce_counter: identity_nonce_counter,
                current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                current_votes: BTreeMap::default(),
                start_time_ms: 1681094380000,
                current_time_ms: end_time_ms,
                current_identities: Vec::new(),
                current_addresses_with_balance: AddressesWithBalance::default(),
            },
            strategy,
            config,
            StrategyRandomness::SeedEntropy(block_start),
        );
    }

    // Test should filter out transactions exceeding max tx bytes per block
    #[test]
    fn run_transactions_exceeding_max_block_size() {
        let strategy = NetworkStrategy {
            strategy: Strategy {
                identity_inserts: IdentityInsertInfo {
                    frequency: Frequency {
                        times_per_block_range: 5..6,
                        chance_per_block: None,
                    },
                    start_keys: 5,
                    extra_keys: Default::default(),
                    start_balance_range: dash_to_duffs!(1)..=dash_to_duffs!(1),
                },

                ..Default::default()
            },
            max_tx_bytes_per_block: 3500,
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

        let outcome =
            run_chain_for_strategy(&mut platform, 1, strategy, config, 15, &mut None, &mut None);
        let state_transitions = outcome
            .state_transition_results_per_block
            .get(&1)
            .expect("expected state transition results");

        // Only three out of five transitions should've made to the block
        assert_eq!(state_transitions.len(), 3);
    }
}
