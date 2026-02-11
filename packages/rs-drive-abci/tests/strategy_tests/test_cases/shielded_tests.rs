#[cfg(test)]
mod tests {

    use crate::execution::run_chain_for_strategy;
    use crate::strategy::NetworkStrategy;
    use dpp::dash_to_credits;
    use dpp::state_transition::StateTransition;
    use drive_abci::config::{
        ChainLockConfig, ExecutionConfig, InstantLockConfig, PlatformConfig, PlatformTestConfig,
        ValidatorSetConfig,
    };
    use drive_abci::logging::LogLevel;
    use drive_abci::test::helpers::setup::TestPlatformBuilder;
    use strategy_tests::frequency::Frequency;
    use strategy_tests::operations::{Operation, OperationType};
    use strategy_tests::{IdentityInsertInfo, StartAddresses, StartIdentities, Strategy};

    /// Strategy test that funds addresses via asset locks and then shields funds
    /// into the shielded credit pool through the multi-block execution pipeline.
    ///
    /// This exercises the full Shield transition lifecycle:
    /// 1. Orchard bundle building (output-only, no spends)
    /// 2. Halo 2 ZK proof generation (via cached ProvingKey)
    /// 3. Address input witness signing
    /// 4. Platform validation (structure + state + ZK proof verification)
    /// 5. Storage operations (commitment tree, encrypted notes, pool balance)
    ///
    /// Note: The first run takes ~30s to build the ProvingKey (cached via OnceLock).
    #[test]
    fn run_chain_shield_transitions() {
        drive_abci::logging::init_for_tests(LogLevel::Debug);

        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![
                    // Fund addresses first (every block, 2-3 asset locks of 20 DASH each)
                    Operation {
                        op_type: OperationType::AddressFundingFromCoreAssetLock(
                            dash_to_credits!(20)..=dash_to_credits!(20),
                        ),
                        frequency: Frequency {
                            times_per_block_range: 2..4,
                            chance_per_block: None,
                        },
                    },
                    // Shield funds from funded addresses (1 per block, 1-5 DASH)
                    Operation {
                        op_type: OperationType::Shield(dash_to_credits!(1)..=dash_to_credits!(5)),
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
            // Shield proof verification is implemented but we keep this simple
            verify_state_transition_results: false,
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

        let outcome =
            run_chain_for_strategy(&mut platform, 5, strategy, config, 15, &mut None, &mut None);

        // Count successful shield transitions across all blocks
        let shield_count = outcome
            .state_transition_results_per_block
            .values()
            .flat_map(|results| results.iter())
            .filter(|(st, result)| matches!(st, StateTransition::Shield(_)) && result.code == 0)
            .count();

        assert!(
            shield_count > 0,
            "expected at least one successful shield transition across 5 blocks"
        );

        tracing::info!(shield_count, "Shield strategy test completed successfully");
    }
}
