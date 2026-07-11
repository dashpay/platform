//! Regression tests for data contract history (keeps_history) proof verification.
//!
//! These tests verify that contracts with `keeps_history=true` can be created and updated
//! successfully, with proper proof verification.
//!
//! Bug fixed: When creating/updating a data contract with `keeps_history=true`, the proof
//! verification would fail with "proof did not contain contract with id ... expected to
//! exist because of state transition (create)".
//!
//! Root cause: In `prove_state_transition_v0`, the code always used non-historical path
//! queries regardless of whether the contract had `keeps_history=true`.

#[cfg(test)]
mod tests {
    use crate::execution::run_chain_for_strategy;
    use crate::strategy::NetworkStrategy;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::config::v0::{
        DataContractConfigGettersV0, DataContractConfigSettersV0,
    };
    use dpp::tests::json_document::json_document_to_created_contract;
    use drive_abci::config::{
        ChainLockConfig, ExecutionConfig, InstantLockConfig, PlatformConfig, PlatformTestConfig,
        ValidatorSetConfig,
    };
    use drive_abci::test::helpers::setup::TestPlatformBuilder;
    use platform_version::version::PlatformVersion;
    use strategy_tests::frequency::Frequency;
    use strategy_tests::{IdentityInsertInfo, StartAddresses, StartIdentities, Strategy};

    /// Regression test for historical contract proof verification bug.
    ///
    /// Bug: When creating a data contract with `keeps_history=true`, the proof
    /// verification would fail with "proof did not contain contract with id ...
    /// expected to exist because of state transition (create)".
    ///
    /// Root cause: In `prove_state_transition_v0`, the code always used
    /// `contract_ids_to_non_historical_path_query` regardless of whether
    /// the contract had `keeps_history=true`.
    #[tokio::test]
    async fn run_chain_create_contract_with_keeps_history_true() {
        let platform_version = PlatformVersion::latest();

        // Load a base contract and modify it to enable keeps_history
        let mut contract = json_document_to_created_contract(
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
            1,
            true,
            platform_version,
        )
        .expect("expected to get contract from a json document");

        // Enable keeps_history on the contract configuration
        contract
            .data_contract_mut()
            .config_mut()
            .set_keeps_history(true);

        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![(contract.clone(), None)],
                operations: vec![],
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
            // CRITICAL: This flag enables proof verification for all state transitions
            // This is what catches the bug - proof verification fails without the fix
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

        // Run the chain - this will create identities and the contract with keeps_history=true
        // The verify_state_transition_results flag ensures proofs are verified
        let outcome =
            run_chain_for_strategy(&mut platform, 2, strategy, config, 15, &mut None, &mut None)
                .await;

        // Verify all state transitions succeeded (including contract creation)
        for (block_height, tx_results) in outcome.state_transition_results_per_block.iter() {
            println!(
                "Block {}: {} state transitions",
                block_height,
                tx_results.len()
            );
            for (state_transition, result) in tx_results {
                println!(
                    "  ST type: {:?}, code: {}",
                    state_transition.state_transition_type(),
                    result.code
                );
                assert_eq!(
                    result.code, 0,
                    "state transition got code {} : {:?}",
                    result.code, state_transition
                );
            }
        }

        // Verify we can fetch the contract and it has keeps_history enabled
        // The contract ID is regenerated when the state transition is created,
        // so we need to get the actual ID from the strategy's contracts
        let contract_id = outcome
            .strategy
            .strategy
            .start_contracts
            .first()
            .expect("expected at least one contract")
            .0
            .data_contract()
            .id();
        println!("Looking for contract with ID: {:?}", contract_id);

        let fetched_contract = outcome
            .abci_app
            .platform
            .drive
            .fetch_contract(contract_id.to_buffer(), None, None, None, platform_version)
            .value
            .expect("expected to execute the fetch of a contract")
            .expect("expected to get a contract fetch info");

        // Verify the contract has keeps_history enabled
        assert!(
            fetched_contract.contract.config().keeps_history(),
            "Contract should have keeps_history=true"
        );
    }

    /// Test both historical and non-historical contract creation in the same chain
    /// to ensure both paths work correctly.
    #[tokio::test]
    async fn run_chain_create_contracts_with_and_without_history() {
        let platform_version = PlatformVersion::latest();

        // Contract 1: keeps_history = true
        let mut historical_contract = json_document_to_created_contract(
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
            1,
            true,
            platform_version,
        )
        .expect("expected to get contract from a json document");
        historical_contract
            .data_contract_mut()
            .config_mut()
            .set_keeps_history(true);

        // Contract 2: keeps_history = false (default)
        let non_historical_contract = json_document_to_created_contract(
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
            2,
            true,
            platform_version,
        )
        .expect("expected to get contract from a json document");
        // Note: keeps_history defaults to false, so we don't need to set it

        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![
                    (historical_contract.clone(), None),
                    (non_historical_contract.clone(), None),
                ],
                operations: vec![],
                start_identities: StartIdentities::default(),
                start_addresses: StartAddresses::default(),
                identity_inserts: IdentityInsertInfo {
                    frequency: Frequency {
                        times_per_block_range: 2..3, // Create 2 identities per block
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
            run_chain_for_strategy(&mut platform, 3, strategy, config, 15, &mut None, &mut None)
                .await;

        // Verify all state transitions succeeded
        for tx_results_per_block in outcome.state_transition_results_per_block.values() {
            for (state_transition, result) in tx_results_per_block {
                assert_eq!(
                    result.code, 0,
                    "state transition got code {} : {:?}",
                    result.code, state_transition
                );
            }
        }

        // Get the actual contract IDs from the strategy (they get regenerated)
        let contracts = &outcome.strategy.strategy.start_contracts;
        assert_eq!(contracts.len(), 2, "Expected 2 contracts");

        // Verify historical contract exists and has correct keeps_history setting
        let historical_id = contracts[0].0.data_contract().id();
        let fetched_historical = outcome
            .abci_app
            .platform
            .drive
            .fetch_contract(
                historical_id.to_buffer(),
                None,
                None,
                None,
                platform_version,
            )
            .value
            .expect("expected to fetch contract")
            .expect("expected contract fetch info");

        assert!(
            fetched_historical.contract.config().keeps_history(),
            "Historical contract should have keeps_history=true"
        );

        // Verify non-historical contract exists and has correct keeps_history setting
        let non_historical_id = contracts[1].0.data_contract().id();
        let fetched_non_historical = outcome
            .abci_app
            .platform
            .drive
            .fetch_contract(
                non_historical_id.to_buffer(),
                None,
                None,
                None,
                platform_version,
            )
            .value
            .expect("expected to fetch contract")
            .expect("expected contract fetch info");

        assert!(
            !fetched_non_historical.contract.config().keeps_history(),
            "Non-historical contract should have keeps_history=false"
        );
    }
}
