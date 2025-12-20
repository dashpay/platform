#[cfg(test)]
mod tests {

    use crate::execution::run_chain_for_strategy;
    use crate::strategy::NetworkStrategy;
    use dpp::dash_to_credits;
    use dpp::identity::{KeyType, Purpose, SecurityLevel};
    use dpp::state_transition::StateTransition;
    use drive_abci::config::{
        ChainLockConfig, ExecutionConfig, InstantLockConfig, PlatformConfig, PlatformTestConfig,
        ValidatorSetConfig,
    };
    use drive_abci::logging::LogLevel;
    use drive_abci::test::helpers::setup::TestPlatformBuilder;
    use platform_version::version::PlatformVersion;
    use strategy_tests::frequency::Frequency;
    use strategy_tests::operations::{Operation, OperationType};
    use strategy_tests::{IdentityInsertInfo, StartAddresses, StartIdentities, Strategy};

    #[test]
    fn run_chain_address_transitions() {
        drive_abci::logging::init_for_tests(LogLevel::Debug);

        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![
                    Operation {
                        op_type: OperationType::AddressTransfer(
                            dash_to_credits!(5)..=dash_to_credits!(5),
                            1..=4,
                            Some(0.2),
                            None,
                        ),
                        frequency: Frequency {
                            times_per_block_range: 1..3,
                            chance_per_block: None,
                        },
                    },
                    Operation {
                        op_type: OperationType::AddressFundingFromCoreAssetLock(
                            dash_to_credits!(20)..=dash_to_credits!(20),
                        ),
                        frequency: Frequency {
                            times_per_block_range: 1..3,
                            chance_per_block: None,
                        },
                    },
                ],
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
            total_hpmns: 100,
            extra_normal_mns: 0,
            validator_quorum_count: 24,
            chain_lock_quorum_count: 24,
            upgrading_info: None,

            proposer_strategy: Default::default(),
            rotate_quorums: false,
            failure_testing: None,
            query_testing: None,
            verify_state_transition_results: true,
            sign_instant_locks: true,
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

        let outcome = run_chain_for_strategy(
            &mut platform,
            10,
            strategy,
            config,
            15,
            &mut None,
            &mut None,
        );

        let executed = outcome
            .state_transition_results_per_block
            .values()
            .flat_map(|results| results.iter())
            .filter(|(state_transition, result)| {
                result.code == 0
                    && matches!(state_transition, StateTransition::AddressFundsTransfer(_))
            })
            .count();
        assert!(executed > 0, "expected at least one address transfer");
    }

    #[test]
    fn run_chain_identity_to_addresses_transitions() {
        drive_abci::logging::init_for_tests(LogLevel::Debug);

        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![Operation {
                    op_type: OperationType::IdentityTransferToAddresses(
                        dash_to_credits!(0.05)..=dash_to_credits!(0.05),
                        1..=4,
                        Some(0.2),
                        None,
                    ),
                    frequency: Frequency {
                        times_per_block_range: 1..3,
                        chance_per_block: None,
                    },
                }],
                start_identities: StartIdentities::default(),
                start_addresses: StartAddresses::default(),
                identity_inserts: IdentityInsertInfo {
                    frequency: Frequency {
                        times_per_block_range: 1..2,
                        chance_per_block: None,
                    },
                    start_keys: 3,
                    extra_keys: [(
                        Purpose::TRANSFER,
                        [(SecurityLevel::CRITICAL, vec![KeyType::ECDSA_SECP256K1])].into(),
                    )]
                    .into(),
                    ..Default::default()
                },
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
            verify_state_transition_results: true,
            sign_instant_locks: true,
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

        let outcome = run_chain_for_strategy(
            &mut platform,
            10,
            strategy,
            config,
            15,
            &mut None,
            &mut None,
        );

        let executed = outcome
            .state_transition_results_per_block
            .values()
            .flat_map(|results| results.iter())
            .filter(|(state_transition, result)| {
                result.code == 0
                    && matches!(
                        state_transition,
                        StateTransition::IdentityCreditTransferToAddresses(_)
                    )
            })
            .count();
        assert!(
            executed > 0,
            "expected at least one identity credit transfer to addresses"
        );

        let addresses = outcome.addresses_with_balance;

        // Verify that addresses were created with balances
        assert!(
            !addresses.addresses_with_balance.is_empty(),
            "expected at least one address with balance"
        );

        // Check that each address has a positive balance
        for (address, (_nonce, balance)) in &addresses.addresses_with_balance {
            assert!(
                *balance > 0,
                "Address {:?} should have a positive balance",
                address
            );
        }
    }

    #[test]
    fn run_chain_identity_create_from_addresses_transitions() {
        let _platform_version = PlatformVersion::latest();
        drive_abci::logging::init_for_tests(LogLevel::Debug);

        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![
                    // First fund addresses via asset lock
                    Operation {
                        op_type: OperationType::AddressFundingFromCoreAssetLock(
                            dash_to_credits!(20)..=dash_to_credits!(20),
                        ),
                        frequency: Frequency {
                            times_per_block_range: 2..4,
                            chance_per_block: None,
                        },
                    },
                    // Then create identities from those funded addresses
                    Operation {
                        op_type: OperationType::IdentityCreateFromAddresses(
                            dash_to_credits!(5)..=dash_to_credits!(10),
                            Some(dash_to_credits!(1)..=dash_to_credits!(2)), // output amount
                            None, // fee strategy (default)
                            3,    // key_count
                            [(
                                Purpose::TRANSFER,
                                [(SecurityLevel::CRITICAL, vec![KeyType::ECDSA_SECP256K1])].into(),
                            )]
                            .into(), // extra_keys
                        ),
                        frequency: Frequency {
                            times_per_block_range: 1..2,
                            chance_per_block: None,
                        },
                    },
                ],
                start_identities: StartIdentities::default(),
                start_addresses: StartAddresses::default(),
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
            verify_state_transition_results: true,
            sign_instant_locks: true,
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

        let outcome = run_chain_for_strategy(
            &mut platform,
            15,
            strategy,
            config,
            15,
            &mut None,
            &mut None,
        );

        let executed = outcome
            .state_transition_results_per_block
            .values()
            .flat_map(|results| results.iter())
            .filter(|(state_transition, result)| {
                result.code == 0
                    && matches!(
                        state_transition,
                        StateTransition::IdentityCreateFromAddresses(_)
                    )
            })
            .count();
        assert!(
            executed > 0,
            "expected at least one identity create from addresses"
        );

        // Verify that output addresses were created with balances (from the output param)
        let addresses = outcome.addresses_with_balance;
        assert!(
            !addresses.addresses_with_balance.is_empty(),
            "expected at least one address with balance from outputs"
        );

        // Check that identities were created
        assert!(
            !outcome.identities.is_empty(),
            "expected at least one identity to be created"
        );
    }
}
