#[cfg(test)]
mod tests {

    use crate::execution::run_chain_for_strategy;
    use crate::strategy::NetworkStrategy;
    use dpp::dash_to_duffs;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::{KeyType, Purpose, SecurityLevel};
    use drive_abci::config::{
        ChainLockConfig, ExecutionConfig, InstantLockConfig, PlatformConfig, PlatformTestConfig,
        ValidatorSetConfig,
    };
    use drive_abci::test::helpers::setup::TestPlatformBuilder;
    use platform_version::version::PlatformVersion;
    use strategy_tests::frequency::Frequency;
    use strategy_tests::operations::{Operation, OperationType};
    use strategy_tests::{IdentityInsertInfo, StartIdentities, Strategy};

    #[test]
    fn run_chain_transfer_between_identities() {
        let platform_version = PlatformVersion::latest();
        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![Operation {
                    op_type: OperationType::IdentityTransfer(None),
                    frequency: Frequency {
                        times_per_block_range: 1..3,
                        chance_per_block: None,
                    },
                }],
                start_identities: StartIdentities::default(),
                identity_inserts: IdentityInsertInfo {
                    //we do this to create some paying transactions
                    frequency: Frequency {
                        times_per_block_range: 6..10,
                        chance_per_block: None,
                    },
                    start_keys: 3,
                    extra_keys: [(
                        Purpose::TRANSFER,
                        [(SecurityLevel::CRITICAL, vec![KeyType::ECDSA_SECP256K1])].into(),
                    )]
                    .into(),
                    start_balance_range: dash_to_duffs!(1)..=dash_to_duffs!(1),
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

        let outcome = run_chain_for_strategy(
            &mut platform,
            15,
            strategy,
            config,
            15,
            &mut None,
            &mut None,
        );

        let _balances = &outcome
            .abci_app
            .platform
            .drive
            .fetch_identities_balances(
                &outcome
                    .identities
                    .iter()
                    .map(|identity| identity.id().to_buffer())
                    .collect(),
                None,
                platform_version,
            )
            .expect("expected to fetch balances");

        assert_eq!(outcome.identities.len(), 106);
    }
}
