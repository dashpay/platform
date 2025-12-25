#[cfg(test)]
mod tests {

    use crate::execution::run_chain_for_strategy;
    use crate::strategy::NetworkStrategy;
    use dapi_grpc::platform::v0::get_addresses_trunk_state_request::{
        GetAddressesTrunkStateRequestV0, Version as RequestVersion,
    };
    use dapi_grpc::platform::v0::get_addresses_trunk_state_response::Version as ResponseVersion;
    use dapi_grpc::platform::v0::GetAddressesTrunkStateRequest;
    use dpp::dash_to_credits;
    use dpp::identity::{KeyType, Purpose, SecurityLevel};
    use dpp::state_transition::StateTransition;
    use drive::drive::Drive;
    use drive_abci::config::{
        ChainLockConfig, ExecutionConfig, InstantLockConfig, PlatformConfig, PlatformTestConfig,
        ValidatorSetConfig,
    };
    use drive_abci::logging::LogLevel;
    use drive_abci::platform_types::platform_state::PlatformStateV0Methods;
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

    #[test]
    fn run_chain_address_transitions_with_checkpoints() {
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
            block_spacing_ms: 180000, // 3 mins
            testing_configs: PlatformTestConfig {
                disable_checkpoints: false,
                ..PlatformTestConfig::default_minimal_verifications()
            },
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let outcome = run_chain_for_strategy(
            &mut platform,
            13,
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

        // Drop outcome to release the mutable borrow of platform
        drop(outcome);

        let platform_version = PlatformVersion::latest();

        // Get current platform state for the query
        let platform_state = platform.platform.state.load();
        let current_height = platform_state.last_committed_block_height();

        // Verify checkpoints exist
        let checkpoints = platform.platform.drive.checkpoints.load();
        assert!(
            !checkpoints.is_empty(),
            "expected at least one checkpoint to be created"
        );

        // Get the checkpoint height
        let (&checkpoint_height, _) = checkpoints.last_key_value().unwrap();

        // Verify expected heights
        assert_eq!(current_height, 13, "expected current height to be 13");
        assert_eq!(checkpoint_height, 12, "expected checkpoint height to be 12");

        // Test the ABCI query layer for trunk state (uses LatestCheckpoint)
        let request = GetAddressesTrunkStateRequest {
            version: Some(RequestVersion::V0(GetAddressesTrunkStateRequestV0 {})),
        };

        let query_result = platform
            .platform
            .query_addresses_trunk_state(request, &platform_state, platform_version)
            .expect("should execute trunk state query");

        assert!(
            query_result.errors.is_empty(),
            "query should succeed: {:?}",
            query_result.errors
        );

        let response = query_result.into_data().expect("expected data");
        let response_v0 = match response.version.expect("expected version") {
            ResponseVersion::V0(v0) => v0,
        };

        // Verify we got a proof
        let proof = response_v0.proof.expect("expected proof");
        assert!(
            !proof.grovedb_proof.is_empty(),
            "grovedb proof should not be empty"
        );

        // Verify the metadata shows we used the checkpoint (height should match checkpoint)
        let metadata = response_v0.metadata.expect("expected metadata");
        assert_eq!(
            metadata.height, 12,
            "trunk query should use checkpoint at height 12"
        );

        // Verify the proof
        let (root_hash, trunk_result) =
            Drive::verify_address_funds_trunk_query(&proof.grovedb_proof, platform_version)
                .expect("should verify trunk query proof");

        // The root hash should be valid (32 bytes)
        assert_eq!(root_hash.len(), 32, "root hash should be 32 bytes");

        // Verify trunk query results
        assert_eq!(
            trunk_result.elements.len(),
            29,
            "trunk query should return 29 elements"
        );
        assert_eq!(
            trunk_result.leaf_keys.len(),
            0,
            "trunk query should return 0 leaf keys"
        );
        assert_eq!(
            trunk_result.chunk_depths,
            vec![6],
            "trunk query should have chunk_depths [6]"
        );
    }
}
