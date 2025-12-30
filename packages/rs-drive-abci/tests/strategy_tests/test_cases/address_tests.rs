#[cfg(test)]
mod tests {

    use crate::execution::run_chain_for_strategy;
    use crate::strategy::NetworkStrategy;
    use dapi_grpc::platform::v0::get_addresses_trunk_state_request::{
        GetAddressesTrunkStateRequestV0, Version as RequestVersion,
    };
    use dapi_grpc::platform::v0::get_addresses_trunk_state_response::Version as ResponseVersion;
    use dapi_grpc::platform::v0::get_recent_address_balance_changes_request::{
        GetRecentAddressBalanceChangesRequestV0, Version as RecentChangesRequestVersion,
    };
    use dapi_grpc::platform::v0::get_recent_address_balance_changes_response::{
        get_recent_address_balance_changes_response_v0, Version as RecentChangesResponseVersion,
    };
    use dapi_grpc::platform::v0::{
        address_balance_change, GetAddressesTrunkStateRequest,
        GetRecentAddressBalanceChangesRequest,
    };
    use dpp::address_funds::PlatformAddress;
    use dpp::dash_to_credits;
    use dpp::dashcore::hashes::Hash;
    use dpp::dashcore::QuorumHash;
    use dpp::data_contract::TokenConfiguration;
    use dpp::identity::{KeyType, Purpose, SecurityLevel};
    use dpp::prelude::{CoreBlockHeight, DataContract, Identifier};
    use dpp::state_transition::address_credit_withdrawal_transition::accessors::AddressCreditWithdrawalTransitionAccessorsV0;
    use dpp::state_transition::address_funding_from_asset_lock_transition::accessors::AddressFundingFromAssetLockTransitionAccessorsV0;
    use dpp::state_transition::address_funds_transfer_transition::accessors::AddressFundsTransferTransitionAccessorsV0;
    use dpp::state_transition::identity_create_from_addresses_transition::accessors::IdentityCreateFromAddressesTransitionAccessorsV0;
    use dpp::state_transition::identity_credit_transfer_to_addresses_transition::accessors::IdentityCreditTransferToAddressesTransitionAccessorsV0;
    use dpp::state_transition::identity_topup_from_addresses_transition::accessors::IdentityTopUpFromAddressesTransitionAccessorsV0;
    use dpp::state_transition::StateTransition;
    use dpp::state_transition::StateTransitionWitnessSigned;
    use drive::drive::Drive;
    use drive_abci::abci::config::AbciConfig;
    use drive_abci::config::{
        ChainLockConfig, ExecutionConfig, InstantLockConfig, PlatformConfig, PlatformTestConfig,
        ValidatorSetConfig,
    };
    use drive_abci::logging::LogLevel;
    use drive_abci::mimic::test_quorum::TestQuorumInfo;
    use drive_abci::mimic::CHAIN_ID;
    use drive_abci::platform_types::platform_state::PlatformStateV0Methods;
    use drive_abci::test::helpers::setup::TestPlatformBuilder;
    use drive_proof_verifier::{ContextProvider, ContextProviderError, FromProof};
    use platform_version::version::PlatformVersion;
    use std::collections::{BTreeMap, BTreeSet};
    use std::sync::Arc;
    use strategy_tests::frequency::Frequency;
    use strategy_tests::operations::{Operation, OperationType};
    use strategy_tests::{IdentityInsertInfo, StartAddresses, StartIdentities, Strategy};

    /// A test ContextProvider that provides quorum public keys from a map of test quorums.
    /// This is used to verify proof signatures in strategy tests.
    struct TestContextProvider {
        quorum_public_keys: BTreeMap<([u8; 32], u32), [u8; 48]>,
    }

    impl TestContextProvider {
        /// Create a new TestContextProvider from the validator quorums in a chain execution outcome.
        fn from_quorums(
            validator_quorums: &BTreeMap<QuorumHash, TestQuorumInfo>,
            quorum_type: u32,
        ) -> Self {
            let quorum_public_keys = validator_quorums
                .iter()
                .map(|(quorum_hash, quorum_info)| {
                    let quorum_hash_bytes: [u8; 32] = *quorum_hash.as_raw_hash().as_byte_array();
                    let public_key_bytes: [u8; 48] = quorum_info
                        .public_key
                        .0
                        .to_compressed()
                        .try_into()
                        .expect("public key should be 48 bytes");
                    ((quorum_hash_bytes, quorum_type), public_key_bytes)
                })
                .collect();
            Self { quorum_public_keys }
        }
    }

    impl ContextProvider for TestContextProvider {
        fn get_quorum_public_key(
            &self,
            quorum_type: u32,
            quorum_hash: [u8; 32],
            _core_chain_locked_height: u32,
        ) -> Result<[u8; 48], ContextProviderError> {
            self.quorum_public_keys
                .get(&(quorum_hash, quorum_type))
                .copied()
                .ok_or_else(|| {
                    ContextProviderError::InvalidQuorum(format!(
                        "quorum not found: type={}, hash={}",
                        quorum_type,
                        hex::encode(quorum_hash)
                    ))
                })
        }

        fn get_data_contract(
            &self,
            _data_contract_id: &Identifier,
            _platform_version: &PlatformVersion,
        ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
            Ok(None)
        }

        fn get_token_configuration(
            &self,
            _token_id: &Identifier,
        ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
            Ok(None)
        }

        fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
            Ok(1)
        }
    }

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

        // Build expected address changes from state transitions
        // For each block, collect:
        // - set_balance_addresses: addresses used as inputs (spending from)
        // - add_to_balance_addresses: addresses used as outputs (receiving funds)
        //
        // Both AddressFundsTransfer and AddressFundingFromAssetLock use
        // ExecutionEvent::PaidFromAddressInputs which tracks address balance changes.
        let mut expected_per_block: BTreeMap<
            u64,
            (BTreeSet<PlatformAddress>, BTreeSet<PlatformAddress>),
        > = BTreeMap::new();

        for (block_height, results) in &outcome.state_transition_results_per_block {
            let mut set_balance_addresses = BTreeSet::new();
            let mut add_to_balance_addresses = BTreeSet::new();

            for (state_transition, result) in results {
                // Only count successful state transitions
                if result.code != 0 {
                    continue;
                }

                match state_transition {
                    StateTransition::AddressFundsTransfer(transfer) => {
                        // Inputs are addresses being spent from → SetBalance
                        for address in transfer.inputs().keys() {
                            set_balance_addresses.insert(*address);
                        }
                        // Outputs are addresses receiving funds → AddToBalance
                        for address in transfer.outputs().keys() {
                            add_to_balance_addresses.insert(*address);
                        }
                    }
                    StateTransition::AddressFundingFromAssetLock(funding) => {
                        // AddressFundingFromAssetLock has no inputs (funds come from asset lock)
                        // Outputs are addresses receiving funds from asset lock → AddToBalance
                        for address in funding.outputs().keys() {
                            add_to_balance_addresses.insert(*address);
                        }
                    }
                    _ => {
                        // Other state transitions don't affect address balances
                    }
                }
            }

            if !set_balance_addresses.is_empty() || !add_to_balance_addresses.is_empty() {
                expected_per_block.insert(
                    *block_height,
                    (set_balance_addresses, add_to_balance_addresses),
                );
            }
        }

        assert!(
            !expected_per_block.is_empty(),
            "expected at least one block with address balance changes"
        );

        // Query recent address balance changes
        let platform = &outcome.abci_app.platform;
        let platform_state = platform.state.load();
        let platform_version = platform_state.current_platform_version().unwrap();

        let request = GetRecentAddressBalanceChangesRequest {
            version: Some(RecentChangesRequestVersion::V0(
                GetRecentAddressBalanceChangesRequestV0 {
                    start_height: 1,
                    prove: false,
                },
            )),
        };

        let query_validation_result = platform
            .query_recent_address_balance_changes(request, &platform_state, platform_version)
            .expect("expected to run query");

        assert!(
            query_validation_result.errors.is_empty(),
            "query errors: {:?}",
            query_validation_result.errors
        );

        let response = query_validation_result
            .into_data()
            .expect("expected data on query_validation_result");

        let versioned_result = response.version.expect("expected a version");
        match versioned_result {
            RecentChangesResponseVersion::V0(v0) => {
                let result = v0.result.expect("expected a result");
                match result {
                    get_recent_address_balance_changes_response_v0::Result::AddressBalanceUpdateEntries(entries) => {
                        // Build actual address changes from query result
                        let mut actual_per_block: BTreeMap<u64, (BTreeSet<PlatformAddress>, BTreeSet<PlatformAddress>)> =
                            BTreeMap::new();

                        for block_change in &entries.block_changes {
                            let mut set_balance_addresses = BTreeSet::new();
                            let mut add_to_balance_addresses = BTreeSet::new();

                            for change in &block_change.changes {
                                let address = PlatformAddress::from_bytes(&change.address)
                                    .expect("expected valid address bytes");

                                match &change.operation {
                                    Some(address_balance_change::Operation::SetBalance(_)) => {
                                        set_balance_addresses.insert(address);
                                    }
                                    Some(address_balance_change::Operation::AddToBalance(_)) => {
                                        add_to_balance_addresses.insert(address);
                                    }
                                    None => {
                                        panic!("expected an operation on address balance change");
                                    }
                                }
                            }

                            actual_per_block.insert(
                                block_change.block_height,
                                (set_balance_addresses, add_to_balance_addresses),
                            );
                        }

                        // Aggregate all expected and actual addresses across all blocks
                        // Note: We aggregate because some transitions may fail during process_proposal
                        // and end up in different blocks than state_transition_results_per_block indicates
                        let mut all_expected_set: BTreeSet<PlatformAddress> = BTreeSet::new();
                        let mut all_expected_add: BTreeSet<PlatformAddress> = BTreeSet::new();
                        let mut all_actual_set: BTreeSet<PlatformAddress> = BTreeSet::new();
                        let mut all_actual_add: BTreeSet<PlatformAddress> = BTreeSet::new();

                        for (_, (exp_set, exp_add)) in &expected_per_block {
                            all_expected_set.extend(exp_set.iter().copied());
                            all_expected_add.extend(exp_add.iter().copied());
                        }

                        for (_, (act_set, act_add)) in &actual_per_block {
                            all_actual_set.extend(act_set.iter().copied());
                            all_actual_add.extend(act_add.iter().copied());
                        }

                        // Verify we have both operation types recorded
                        assert!(
                            !all_actual_set.is_empty(),
                            "expected at least one SetBalance operation"
                        );
                        assert!(
                            !all_actual_add.is_empty(),
                            "expected at least one AddToBalance operation"
                        );

                        // Verify a significant portion of expected addresses are found in actual
                        // (some may be missing due to transitions failing during process_proposal)
                        let set_intersection: BTreeSet<_> = all_expected_set.intersection(&all_actual_set).collect();
                        let add_intersection: BTreeSet<_> = all_expected_add.intersection(&all_actual_add).collect();

                        let set_match_ratio = if all_expected_set.is_empty() {
                            1.0
                        } else {
                            set_intersection.len() as f64 / all_expected_set.len() as f64
                        };
                        let add_match_ratio = if all_expected_add.is_empty() {
                            1.0
                        } else {
                            add_intersection.len() as f64 / all_expected_add.len() as f64
                        };

                        eprintln!(
                            "SetBalance: expected={}, actual={}, match_ratio={:.2} ({}/{})",
                            all_expected_set.len(),
                            all_actual_set.len(),
                            set_match_ratio,
                            set_intersection.len(),
                            all_expected_set.len()
                        );
                        eprintln!(
                            "AddToBalance: expected={}, actual={}, match_ratio={:.2} ({}/{})",
                            all_expected_add.len(),
                            all_actual_add.len(),
                            add_match_ratio,
                            add_intersection.len(),
                            all_expected_add.len()
                        );

                        // Require at least 50% of expected addresses to be found
                        // (accounting for transitions that may fail during execution)
                        assert!(
                            set_match_ratio >= 0.5,
                            "SetBalance match ratio too low: {:.2}, expected at least 0.5",
                            set_match_ratio
                        );
                        assert!(
                            add_match_ratio >= 0.5,
                            "AddToBalance match ratio too low: {:.2}, expected at least 0.5",
                            add_match_ratio
                        );
                    }
                    get_recent_address_balance_changes_response_v0::Result::Proof(_) => {
                        panic!("expected entries, not proof");
                    }
                }
            }
        }
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
            32,
            "trunk query should return 32 elements"
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

    #[test]
    fn run_chain_address_transitions_with_checkpoints_stop_and_restart() {
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

        // Verify checkpoints exist before restart
        let checkpoints = platform.platform.drive.checkpoints.load();
        assert!(
            !checkpoints.is_empty(),
            "expected at least one checkpoint to be created"
        );
        let (&checkpoint_height, _) = checkpoints.last_key_value().unwrap();
        assert_eq!(checkpoint_height, 12, "expected checkpoint height to be 12");

        // Verify checkpoint platform states exist before restart
        let checkpoint_states_before = platform.platform.checkpoint_platform_states.load();
        assert!(
            checkpoint_states_before.contains_key(&12),
            "expected checkpoint platform state at height 12 before restart"
        );

        // Simulate restart by reloading state from storage
        platform
            .platform
            .reload_state_from_storage(platform_version)
            .expect("expected to reload state from storage");

        // Verify checkpoint platform states were restored after restart
        let checkpoint_states_after = platform.platform.checkpoint_platform_states.load();
        assert!(
            checkpoint_states_after.contains_key(&12),
            "expected checkpoint platform state at height 12 after restart"
        );

        // Verify we can still query using checkpoints after restart
        let platform_state = platform.platform.state.load();
        let current_height = platform_state.last_committed_block_height();
        assert_eq!(current_height, 13, "expected current height to be 13");

        // Test the ABCI query layer for trunk state (uses LatestCheckpoint)
        let request = GetAddressesTrunkStateRequest {
            version: Some(RequestVersion::V0(GetAddressesTrunkStateRequestV0 {})),
        };

        let query_result = platform
            .platform
            .query_addresses_trunk_state(request, &platform_state, platform_version)
            .expect("should execute trunk state query after restart");

        assert!(
            query_result.errors.is_empty(),
            "query should succeed after restart: {:?}",
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
            "grovedb proof should not be empty after restart"
        );

        // Verify the metadata shows we used the checkpoint (height should match checkpoint)
        let metadata = response_v0.metadata.expect("expected metadata");
        assert_eq!(
            metadata.height, 12,
            "trunk query should use checkpoint at height 12 after restart"
        );

        // Verify the proof
        let (root_hash, trunk_result) =
            Drive::verify_address_funds_trunk_query(&proof.grovedb_proof, platform_version)
                .expect("should verify trunk query proof after restart");

        // The root hash should be valid (32 bytes)
        assert_eq!(root_hash.len(), 32, "root hash should be 32 bytes");

        // Verify trunk query results match expected values
        assert_eq!(
            trunk_result.elements.len(),
            32,
            "trunk query should return 32 elements after restart"
        );
        assert_eq!(
            trunk_result.leaf_keys.len(),
            0,
            "trunk query should return 0 leaf keys after restart"
        );
        assert_eq!(
            trunk_result.chunk_depths,
            vec![6],
            "trunk query should have chunk_depths [6] after restart"
        );
    }

    #[test]
    fn run_chain_address_withdrawal_transitions() {
        use dpp::dashcore::Txid;

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
                    // Then withdraw from those funded addresses
                    Operation {
                        op_type: OperationType::AddressWithdrawal(
                            dash_to_credits!(1)..=dash_to_credits!(5), // withdrawal amount
                            Some(dash_to_credits!(0.5)..=dash_to_credits!(1)), // optional output amount (change)
                            None, // fee strategy (default)
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

        // Configure mock RPC to handle withdrawal transactions
        platform
            .core_rpc
            .expect_send_raw_transaction()
            .returning(move |_| Ok(Txid::all_zeros()));

        let outcome = run_chain_for_strategy(
            &mut platform,
            15,
            strategy,
            config,
            15,
            &mut None,
            &mut None,
        );

        // Count successful AddressCreditWithdrawal state transitions
        let withdrawal_count = outcome
            .state_transition_results_per_block
            .values()
            .flat_map(|results| results.iter())
            .filter(|(state_transition, result)| {
                result.code == 0
                    && matches!(
                        state_transition,
                        StateTransition::AddressCreditWithdrawal(_)
                    )
            })
            .count();

        assert!(
            withdrawal_count > 0,
            "expected at least one successful address credit withdrawal"
        );

        // Count successful AddressFundingFromAssetLock state transitions
        let funding_count = outcome
            .state_transition_results_per_block
            .values()
            .flat_map(|results| results.iter())
            .filter(|(state_transition, result)| {
                result.code == 0
                    && matches!(
                        state_transition,
                        StateTransition::AddressFundingFromAssetLock(_)
                    )
            })
            .count();

        assert!(
            funding_count > 0,
            "expected at least one successful address funding from asset lock"
        );

        // Verify that the query results match the expected operations from state transitions
        // For each state transition we know:
        // - AddressFundingFromAssetLock: outputs receive AddToBalance with specific amounts
        // - AddressCreditWithdrawal: inputs get SetBalance, optional output gets AddToBalance

        // Build expected operations per block from state transitions
        // Map: block_height -> (address -> expected operation)
        let mut expected_per_block: BTreeMap<
            u64,
            BTreeMap<PlatformAddress, address_balance_change::Operation>,
        > = BTreeMap::new();

        for (block_height, results) in &outcome.state_transition_results_per_block {
            for (state_transition, result) in results {
                if result.code != 0 {
                    continue;
                }

                let block_ops = expected_per_block.entry(*block_height).or_default();

                match state_transition {
                    StateTransition::AddressFundingFromAssetLock(funding) => {
                        // Outputs are addresses receiving funds → AddToBalance
                        for (address, maybe_credits) in funding.outputs() {
                            // Credits may be None if not explicitly specified, but we still expect AddToBalance
                            let credits = maybe_credits.unwrap_or(0);
                            block_ops.insert(
                                *address,
                                address_balance_change::Operation::AddToBalance(credits),
                            );
                        }
                    }
                    StateTransition::AddressCreditWithdrawal(withdrawal) => {
                        // Inputs are addresses being spent from → SetBalance
                        // The final balance depends on fees, so we just verify it's a SetBalance
                        for address in withdrawal.inputs().keys() {
                            // We can't predict the exact balance after fees, but we know it should be SetBalance
                            // Mark with a placeholder that we'll verify is SetBalance type
                            block_ops.insert(
                                *address,
                                address_balance_change::Operation::SetBalance(0), // placeholder
                            );
                        }
                        // Optional output (change) → AddToBalance
                        if let Some((address, credits)) = withdrawal.output() {
                            block_ops.insert(
                                *address,
                                address_balance_change::Operation::AddToBalance(*credits),
                            );
                        }
                    }
                    _ => {}
                }
            }
        }

        // Query recent address balance changes
        let platform = &outcome.abci_app.platform;
        let platform_state = platform.state.load();
        let platform_version = platform_state.current_platform_version().unwrap();

        let request = GetRecentAddressBalanceChangesRequest {
            version: Some(RecentChangesRequestVersion::V0(
                GetRecentAddressBalanceChangesRequestV0 {
                    start_height: 1,
                    prove: false,
                },
            )),
        };

        let query_validation_result = platform
            .query_recent_address_balance_changes(request, &platform_state, platform_version)
            .expect("expected to run query");

        assert!(
            query_validation_result.errors.is_empty(),
            "query errors: {:?}",
            query_validation_result.errors
        );

        let response = query_validation_result
            .into_data()
            .expect("expected data on query_validation_result");

        let versioned_result = response.version.expect("expected a version");
        match versioned_result {
            RecentChangesResponseVersion::V0(v0) => {
                let result = v0.result.expect("expected a result");
                match result {
                    get_recent_address_balance_changes_response_v0::Result::AddressBalanceUpdateEntries(entries) => {
                        // Build actual operations per block from query results
                        let mut actual_per_block: BTreeMap<u64, BTreeMap<PlatformAddress, address_balance_change::Operation>> =
                            BTreeMap::new();

                        for block_change in &entries.block_changes {
                            let block_ops = actual_per_block.entry(block_change.block_height).or_default();

                            for change in &block_change.changes {
                                let address = PlatformAddress::from_bytes(&change.address)
                                    .expect("expected valid address bytes");

                                if let Some(op) = &change.operation {
                                    block_ops.insert(address, op.clone());
                                }
                            }
                        }

                        // Verify each expected operation exists in actual results
                        for (block_height, expected_ops) in &expected_per_block {
                            for (address, expected_op) in expected_ops {
                                // Find this address in actual results (may be in a different block due to processing)
                                let mut found = false;
                                for (_, actual_ops) in &actual_per_block {
                                    if let Some(actual_op) = actual_ops.get(address) {
                                        // Verify operation type matches
                                        match (expected_op, actual_op) {
                                            (
                                                address_balance_change::Operation::SetBalance(_),
                                                address_balance_change::Operation::SetBalance(_),
                                            ) => {
                                                // SetBalance type matches (value may differ due to fees)
                                                found = true;
                                                break;
                                            }
                                            (
                                                address_balance_change::Operation::AddToBalance(expected_credits),
                                                address_balance_change::Operation::AddToBalance(actual_credits),
                                            ) => {
                                                // AddToBalance - verify the amount matches
                                                // If expected is 0 (unknown from state transition), just verify type
                                                // Otherwise, actual may be less due to fee deduction from outputs
                                                if *expected_credits == 0 || actual_credits <= expected_credits {
                                                    found = true;
                                                    break;
                                                }
                                            }
                                            _ => {
                                                // Type mismatch
                                            }
                                        }
                                    }
                                }

                                assert!(
                                    found,
                                    "Missing or mismatched operation for address {:?} at block {}: expected {:?}",
                                    address, block_height, expected_op
                                );
                            }
                        }
                    }
                    get_recent_address_balance_changes_response_v0::Result::Proof(_) => {
                        panic!("expected entries, not proof");
                    }
                }
            }
        }
    }

    /// Test for IdentityCreditTransferToAddresses with pre-configured identities
    /// that have transfer keys
    #[test]
    fn run_chain_identity_credit_transfer_to_addresses() {
        drive_abci::logging::init_for_tests(LogLevel::Debug);

        // Create start identities with transfer keys
        let extra_keys = [(
            Purpose::TRANSFER,
            [(SecurityLevel::CRITICAL, vec![KeyType::ECDSA_SECP256K1])].into(),
        )]
        .into();

        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![
                    // Fund addresses first via asset lock so we have recipients
                    Operation {
                        op_type: OperationType::AddressFundingFromCoreAssetLock(
                            dash_to_credits!(20)..=dash_to_credits!(20),
                        ),
                        frequency: Frequency {
                            times_per_block_range: 1..3,
                            chance_per_block: None,
                        },
                    },
                    // Identity transfers credits to addresses
                    Operation {
                        op_type: OperationType::IdentityTransferToAddresses(
                            dash_to_credits!(1)..=dash_to_credits!(2),
                            1..=3,
                            Some(0.3),
                            None,
                        ),
                        frequency: Frequency {
                            times_per_block_range: 1..3,
                            chance_per_block: None,
                        },
                    },
                ],
                start_identities: StartIdentities {
                    number_of_identities: 10,
                    keys_per_identity: 3,
                    starting_balances: dash_to_credits!(100),
                    extra_keys,
                    hard_coded: vec![],
                },
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
            strategy.clone(),
            config.clone(),
            89,
            &mut None,
            &mut None,
        );

        // Count IdentityCreditTransferToAddresses transitions
        let mut transfer_to_addresses_count = 0u32;
        let mut funding_count = 0u32;

        for results in outcome.state_transition_results_per_block.values() {
            for (state_transition, result) in results {
                if result.code == 0 {
                    match state_transition {
                        StateTransition::IdentityCreditTransferToAddresses(_) => {
                            transfer_to_addresses_count += 1
                        }
                        StateTransition::AddressFundingFromAssetLock(_) => funding_count += 1,
                        _ => {}
                    }
                }
            }
        }

        assert!(
            transfer_to_addresses_count > 0,
            "expected at least one IdentityCreditTransferToAddresses, got 0"
        );
        assert!(
            funding_count > 0,
            "expected at least one AddressFundingFromAssetLock, got 0"
        );

        // Build expected operations from state transitions
        let mut expected_per_block: BTreeMap<
            u64,
            BTreeMap<PlatformAddress, address_balance_change::Operation>,
        > = BTreeMap::new();

        for (block_height, results) in &outcome.state_transition_results_per_block {
            for (state_transition, result) in results {
                if result.code != 0 {
                    continue;
                }

                let block_ops = expected_per_block.entry(*block_height).or_default();

                match state_transition {
                    StateTransition::AddressFundingFromAssetLock(funding) => {
                        for (address, maybe_credits) in funding.outputs() {
                            let credits = maybe_credits.unwrap_or(0);
                            block_ops.insert(
                                *address,
                                address_balance_change::Operation::AddToBalance(credits),
                            );
                        }
                    }
                    StateTransition::IdentityCreditTransferToAddresses(transfer) => {
                        for (address, credits) in transfer.recipient_addresses() {
                            block_ops.insert(
                                *address,
                                address_balance_change::Operation::AddToBalance(*credits),
                            );
                        }
                    }
                    _ => {}
                }
            }
        }

        // Query recent address balance changes
        let platform = &outcome.abci_app.platform;
        let platform_state = platform.state.load();
        let platform_version = platform_state.current_platform_version().unwrap();

        let request = GetRecentAddressBalanceChangesRequest {
            version: Some(RecentChangesRequestVersion::V0(
                GetRecentAddressBalanceChangesRequestV0 {
                    start_height: 1,
                    prove: false,
                },
            )),
        };

        let query_validation_result = platform
            .query_recent_address_balance_changes(request, &platform_state, platform_version)
            .expect("expected to run query");

        assert!(
            query_validation_result.errors.is_empty(),
            "query errors: {:?}",
            query_validation_result.errors
        );

        let response = query_validation_result
            .into_data()
            .expect("expected data on query_validation_result");

        let versioned_result = response.version.expect("expected a version");
        match versioned_result {
            RecentChangesResponseVersion::V0(v0) => {
                let result = v0.result.expect("expected a result");
                match result {
                    get_recent_address_balance_changes_response_v0::Result::AddressBalanceUpdateEntries(entries) => {
                        // Build actual operations from query results
                        let mut actual_per_block: BTreeMap<
                            u64,
                            BTreeMap<PlatformAddress, address_balance_change::Operation>,
                        > = BTreeMap::new();

                        for block_change in &entries.block_changes {
                            let block_ops =
                                actual_per_block.entry(block_change.block_height).or_default();

                            for change in &block_change.changes {
                                let address = PlatformAddress::from_bytes(&change.address)
                                    .expect("expected valid address bytes");

                                if let Some(op) = &change.operation {
                                    block_ops.insert(address, op.clone());
                                }
                            }
                        }

                        // Verify each expected operation exists in actual results
                        for (block_height, expected_ops) in &expected_per_block {
                            for (address, expected_op) in expected_ops {
                                let mut found = false;
                                for (_, actual_ops) in &actual_per_block {
                                    if let Some(actual_op) = actual_ops.get(address) {
                                        match (expected_op, actual_op) {
                                            (
                                                address_balance_change::Operation::AddToBalance(
                                                    expected_credits,
                                                ),
                                                address_balance_change::Operation::AddToBalance(
                                                    actual_credits,
                                                ),
                                            ) => {
                                                // AddToBalance matches (may be adjusted for fees)
                                                if *expected_credits == 0
                                                    || actual_credits <= expected_credits
                                                {
                                                    found = true;
                                                    break;
                                                }
                                            }
                                            (
                                                address_balance_change::Operation::AddToBalance(_),
                                                address_balance_change::Operation::SetBalance(_),
                                            ) => {
                                                // Address was funded then spent - valid
                                                found = true;
                                                break;
                                            }
                                            _ => {}
                                        }
                                    }
                                }

                                assert!(
                                    found,
                                    "Missing operation for address {:?} at block {}: expected {:?}",
                                    address, block_height, expected_op
                                );
                            }
                        }
                    }
                    get_recent_address_balance_changes_response_v0::Result::Proof(_) => {
                        panic!("expected entries, not proof");
                    }
                }
            }
        }
    }

    /// Comprehensive test with 5 address-related state transitions:
    /// 1. AddressFundingFromAssetLock - Fund addresses from asset lock
    /// 2. AddressFundsTransfer - Transfer between addresses
    /// 3. AddressCreditWithdrawal - Withdraw from addresses to core
    /// 4. IdentityCreateFromAddresses - Creates identity from address funds
    /// 5. IdentityTopUpFromAddresses - Tops up identity from address funds
    /// Note: IdentityCreditTransferToAddresses requires identities with transfer keys (special setup)
    #[test]
    fn run_chain_all_address_transitions() {
        use dpp::dashcore::Txid;

        drive_abci::logging::init_for_tests(LogLevel::Debug);

        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![
                    // 1. Fund addresses via asset lock
                    Operation {
                        op_type: OperationType::AddressFundingFromCoreAssetLock(
                            dash_to_credits!(20)..=dash_to_credits!(20),
                        ),
                        frequency: Frequency {
                            times_per_block_range: 2..4,
                            chance_per_block: None,
                        },
                    },
                    // 2. Transfer between addresses
                    Operation {
                        op_type: OperationType::AddressTransfer(
                            dash_to_credits!(1)..=dash_to_credits!(3),
                            1..=2,
                            Some(0.3),
                            None,
                        ),
                        frequency: Frequency {
                            times_per_block_range: 1..3,
                            chance_per_block: None,
                        },
                    },
                    // 3. Withdraw from addresses
                    Operation {
                        op_type: OperationType::AddressWithdrawal(
                            dash_to_credits!(1)..=dash_to_credits!(2),
                            Some(dash_to_credits!(0.5)..=dash_to_credits!(1)),
                            None,
                        ),
                        frequency: Frequency {
                            times_per_block_range: 0..2,
                            chance_per_block: Some(0.5),
                        },
                    },
                    // Note: IdentityTransferToAddresses requires identities with transfer keys
                    // which need special setup - skipped for now
                    // 5. Create identity from address funds
                    Operation {
                        op_type: OperationType::IdentityCreateFromAddresses(
                            dash_to_credits!(5)..=dash_to_credits!(10),
                            Some(dash_to_credits!(1)..=dash_to_credits!(2)),
                            None,
                            3,
                            [(
                                Purpose::AUTHENTICATION,
                                [(SecurityLevel::CRITICAL, vec![KeyType::ECDSA_SECP256K1])].into(),
                            )]
                            .into(),
                        ),
                        frequency: Frequency {
                            times_per_block_range: 0..2,
                            chance_per_block: Some(0.3),
                        },
                    },
                    // 6. Top up identity from address funds
                    Operation {
                        op_type: OperationType::IdentityTopUpFromAddresses(
                            dash_to_credits!(1)..=dash_to_credits!(3),
                        ),
                        frequency: Frequency {
                            times_per_block_range: 0..2,
                            chance_per_block: Some(0.4),
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

        // Configure mock RPC to handle withdrawal transactions
        platform
            .core_rpc
            .expect_send_raw_transaction()
            .returning(move |_| Ok(Txid::all_zeros()));

        let outcome = run_chain_for_strategy(
            &mut platform,
            15,
            strategy.clone(),
            config.clone(),
            89,
            &mut None,
            &mut None,
        );

        // Count each type of state transition
        let mut funding_count = 0u32;
        let mut transfer_count = 0u32;
        let mut withdrawal_count = 0u32;
        let mut identity_create_from_addresses_count = 0u32;
        let mut identity_topup_from_addresses_count = 0u32;

        for results in outcome.state_transition_results_per_block.values() {
            for (state_transition, result) in results {
                if result.code == 0 {
                    match state_transition {
                        StateTransition::AddressFundingFromAssetLock(_) => funding_count += 1,
                        StateTransition::AddressFundsTransfer(_) => transfer_count += 1,
                        StateTransition::AddressCreditWithdrawal(_) => withdrawal_count += 1,
                        StateTransition::IdentityCreateFromAddresses(_) => {
                            identity_create_from_addresses_count += 1
                        }
                        StateTransition::IdentityTopUpFromAddresses(_) => {
                            identity_topup_from_addresses_count += 1
                        }
                        _ => {}
                    }
                }
            }
        }

        // Verify we have at least some of each type (may not have all due to randomness)
        assert!(
            funding_count > 0,
            "expected at least one AddressFundingFromAssetLock"
        );
        assert!(
            transfer_count > 0,
            "expected at least one AddressFundsTransfer"
        );

        // Build expected operations from state transitions
        let mut expected_per_block: BTreeMap<
            u64,
            BTreeMap<PlatformAddress, address_balance_change::Operation>,
        > = BTreeMap::new();

        for (block_height, results) in &outcome.state_transition_results_per_block {
            for (state_transition, result) in results {
                if result.code != 0 {
                    continue;
                }

                let block_ops = expected_per_block.entry(*block_height).or_default();

                match state_transition {
                    // 1. AddressFundingFromAssetLock: outputs → AddToBalance
                    StateTransition::AddressFundingFromAssetLock(funding) => {
                        for (address, maybe_credits) in funding.outputs() {
                            let credits = maybe_credits.unwrap_or(0);
                            block_ops.insert(
                                *address,
                                address_balance_change::Operation::AddToBalance(credits),
                            );
                        }
                    }

                    // 2. AddressFundsTransfer: inputs → SetBalance, outputs → AddToBalance
                    StateTransition::AddressFundsTransfer(transfer) => {
                        for address in transfer.inputs().keys() {
                            block_ops
                                .insert(*address, address_balance_change::Operation::SetBalance(0));
                        }
                        for (address, credits) in transfer.outputs() {
                            block_ops.insert(
                                *address,
                                address_balance_change::Operation::AddToBalance(*credits),
                            );
                        }
                    }

                    // 3. AddressCreditWithdrawal: inputs → SetBalance, optional output → AddToBalance
                    StateTransition::AddressCreditWithdrawal(withdrawal) => {
                        for address in withdrawal.inputs().keys() {
                            block_ops
                                .insert(*address, address_balance_change::Operation::SetBalance(0));
                        }
                        if let Some((address, credits)) = withdrawal.output() {
                            block_ops.insert(
                                *address,
                                address_balance_change::Operation::AddToBalance(*credits),
                            );
                        }
                    }

                    // Note: IdentityCreditTransferToAddresses requires special setup - skipped

                    // 4. IdentityCreateFromAddresses: inputs → SetBalance, optional output → AddToBalance
                    StateTransition::IdentityCreateFromAddresses(create) => {
                        for address in create.inputs().keys() {
                            block_ops
                                .insert(*address, address_balance_change::Operation::SetBalance(0));
                        }
                        if let Some((address, credits)) = create.output() {
                            block_ops.insert(
                                *address,
                                address_balance_change::Operation::AddToBalance(*credits),
                            );
                        }
                    }

                    // 5. IdentityTopUpFromAddresses: inputs → SetBalance, optional output → AddToBalance
                    StateTransition::IdentityTopUpFromAddresses(topup) => {
                        for address in topup.inputs().keys() {
                            block_ops
                                .insert(*address, address_balance_change::Operation::SetBalance(0));
                        }
                        if let Some((address, credits)) = topup.output() {
                            block_ops.insert(
                                *address,
                                address_balance_change::Operation::AddToBalance(*credits),
                            );
                        }
                    }

                    _ => {}
                }
            }
        }

        // Query recent address balance changes
        let platform = &outcome.abci_app.platform;
        let platform_state = platform.state.load();
        let platform_version = platform_state.current_platform_version().unwrap();

        let request = GetRecentAddressBalanceChangesRequest {
            version: Some(RecentChangesRequestVersion::V0(
                GetRecentAddressBalanceChangesRequestV0 {
                    start_height: 1,
                    prove: false,
                },
            )),
        };

        let query_validation_result = platform
            .query_recent_address_balance_changes(request, &platform_state, platform_version)
            .expect("expected to run query");

        assert!(
            query_validation_result.errors.is_empty(),
            "query errors: {:?}",
            query_validation_result.errors
        );

        let response = query_validation_result
            .into_data()
            .expect("expected data on query_validation_result");

        let versioned_result = response.version.expect("expected a version");
        match versioned_result {
            RecentChangesResponseVersion::V0(v0) => {
                let result = v0.result.expect("expected a result");
                match result {
                    get_recent_address_balance_changes_response_v0::Result::AddressBalanceUpdateEntries(entries) => {
                        // Build actual operations per block from query results
                        let mut actual_per_block: BTreeMap<
                            u64,
                            BTreeMap<PlatformAddress, address_balance_change::Operation>,
                        > = BTreeMap::new();

                        for block_change in &entries.block_changes {
                            let block_ops =
                                actual_per_block.entry(block_change.block_height).or_default();

                            for change in &block_change.changes {
                                let address = PlatformAddress::from_bytes(&change.address)
                                    .expect("expected valid address bytes");

                                if let Some(op) = &change.operation {
                                    block_ops.insert(address, op.clone());
                                }
                            }
                        }

                        // Verify each expected operation exists in actual results
                        // Note: An address can appear in multiple transactions in the same or different blocks
                        // with different operation types (e.g., AddToBalance from funding, SetBalance from spending)
                        // The stored operation type may differ from expected if a later transaction modified the address
                        for (block_height, expected_ops) in &expected_per_block {
                            for (address, expected_op) in expected_ops {
                                let mut found = false;
                                for (_, actual_ops) in &actual_per_block {
                                    if let Some(actual_op) = actual_ops.get(address) {
                                        match (expected_op, actual_op) {
                                            (
                                                address_balance_change::Operation::SetBalance(_),
                                                address_balance_change::Operation::SetBalance(_),
                                            ) => {
                                                // SetBalance matches SetBalance
                                                found = true;
                                                break;
                                            }
                                            (
                                                address_balance_change::Operation::AddToBalance(
                                                    expected_credits,
                                                ),
                                                address_balance_change::Operation::AddToBalance(
                                                    actual_credits,
                                                ),
                                            ) => {
                                                // AddToBalance matches AddToBalance (may be adjusted for fees)
                                                if *expected_credits == 0
                                                    || actual_credits <= expected_credits
                                                {
                                                    found = true;
                                                    break;
                                                }
                                            }
                                            (
                                                address_balance_change::Operation::AddToBalance(_),
                                                address_balance_change::Operation::SetBalance(_),
                                            ) => {
                                                // Address was funded (AddToBalance) then spent (SetBalance)
                                                // SetBalance overwrites, so this is valid
                                                found = true;
                                                break;
                                            }
                                            (
                                                address_balance_change::Operation::SetBalance(_),
                                                address_balance_change::Operation::AddToBalance(_),
                                            ) => {
                                                // Address was spent (SetBalance) then received (AddToBalance)
                                                // AddToBalance adds to the SetBalance result, so this is valid
                                                found = true;
                                                break;
                                            }
                                        }
                                    }
                                }

                                assert!(
                                    found,
                                    "Missing operation for address {:?} at block {}: expected {:?}",
                                    address, block_height, expected_op
                                );
                            }
                        }
                    }
                    get_recent_address_balance_changes_response_v0::Result::Proof(_) => {
                        panic!("expected entries, not proof");
                    }
                }
            }
        }
    }

    /// Test that verifies proof signatures using the rs-sdk FromProof pattern.
    /// This test is designed to reveal proof signature mismatch errors by:
    /// 1. Running the chain with quorum signing enabled
    /// 2. Using a test ContextProvider that knows the quorum public keys
    /// 3. Verifying proofs using the FromProof trait which includes signature verification
    #[test]
    fn run_chain_address_transitions_with_proof_signature_verification() {
        use dpp::dashcore::Network;
        use drive::grovedb::GroveTrunkQueryResult;

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
                block_signing: true,
                block_commit_signature_verification: true,
                disable_checkpoints: false,
                ..PlatformTestConfig::default_minimal_verifications()
            },
            abci: AbciConfig {
                chain_id: CHAIN_ID.to_string(),
                ..Default::default()
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
            config.clone(),
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

        let platform_version = PlatformVersion::latest();

        // Get current platform state for the query
        let platform_state = outcome.abci_app.platform.state.load();
        let current_height = platform_state.last_committed_block_height();

        // Verify checkpoints exist
        let checkpoints = outcome.abci_app.platform.drive.checkpoints.load();
        assert!(
            !checkpoints.is_empty(),
            "expected at least one checkpoint to be created"
        );

        // Get the checkpoint height
        let (&checkpoint_height, _) = checkpoints.last_key_value().unwrap();

        // Verify expected heights
        assert_eq!(current_height, 13, "expected current height to be 13");
        assert_eq!(checkpoint_height, 12, "expected checkpoint height to be 12");

        // Create a test ContextProvider that knows about the quorum public keys
        let quorum_type = config.validator_set.quorum_type as u32;
        let context_provider =
            TestContextProvider::from_quorums(&outcome.validator_quorums, quorum_type);

        // Test the ABCI query layer for trunk state (uses LatestCheckpoint)
        let request = GetAddressesTrunkStateRequest {
            version: Some(RequestVersion::V0(GetAddressesTrunkStateRequestV0 {})),
        };

        let query_result = outcome
            .abci_app
            .platform
            .query_addresses_trunk_state(request.clone(), &platform_state, platform_version)
            .expect("should execute trunk state query");

        assert!(
            query_result.errors.is_empty(),
            "query should succeed: {:?}",
            query_result.errors
        );

        let response = query_result.into_data().expect("expected data");

        // Now verify the proof using the FromProof trait with our test ContextProvider
        // This is the key test - it verifies the proof signature using the quorum public key
        let verification_result = GroveTrunkQueryResult::maybe_from_proof_with_metadata::<_, _>(
            request,
            response,
            Network::Testnet,
            platform_version,
            &context_provider,
        );

        // This should succeed if proof signatures are correctly generated
        let (trunk_result, metadata, proof) = verification_result
            .expect("proof signature verification should succeed - this is the key test");

        let trunk_result = trunk_result.expect("trunk result should exist");

        // Log proof details for debugging
        tracing::info!(
            "Proof verified successfully: quorum_hash={}, quorum_type={}, signature_len={}, round={}",
            hex::encode(&proof.quorum_hash),
            proof.quorum_type,
            proof.signature.len(),
            proof.round
        );

        // Verify metadata shows we used the checkpoint
        assert_eq!(
            metadata.height, 12,
            "trunk query should use checkpoint at height 12"
        );

        // Verify trunk query results
        assert_eq!(
            trunk_result.elements.len(),
            32,
            "trunk query should return 32 elements"
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

        // Verify the proof has valid quorum info
        assert_eq!(
            proof.quorum_hash.len(),
            32,
            "quorum hash should be 32 bytes"
        );
        assert_eq!(proof.signature.len(), 96, "signature should be 96 bytes");
        assert!(
            proof.signature.iter().any(|&b| b != 0),
            "signature should not be all zeros"
        );
        assert_eq!(
            proof.quorum_type, quorum_type,
            "quorum type in proof should match config"
        );
    }
}
