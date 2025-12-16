#[cfg(test)]
mod tests {
    use crate::execution::run_chain_for_strategy;
    use crate::strategy::NetworkStrategy;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use drive_abci::config::{
        ChainLockConfig, ExecutionConfig, InstantLockConfig, PlatformConfig, PlatformTestConfig,
        ValidatorSetConfig,
    };
    use drive_abci::platform_types::platform_state::PlatformStateV0Methods;
    use drive_abci::test::helpers::setup::TestPlatformBuilder;
    use platform_version::version::PlatformVersion;
    use strategy_tests::frequency::Frequency;
    use strategy_tests::operations::{IdentityUpdateOp, Operation, OperationType};
    use strategy_tests::{IdentityInsertInfo, StartIdentities, Strategy};
    #[test]
    fn run_chain_update_identities_add_keys() {
        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![Operation {
                    op_type: OperationType::IdentityUpdate(
                        IdentityUpdateOp::IdentityUpdateAddKeys(3),
                    ),
                    frequency: Frequency {
                        times_per_block_range: 1..2,
                        chance_per_block: None,
                    },
                }],
                start_identities: StartIdentities::default(),
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
            // because we can add an identity and add keys to it in the same block
            // the result would be different from expected
            verify_state_transition_results: false,
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
        let state = outcome.abci_app.platform.state.load();
        let protocol_version = state.current_protocol_version_in_consensus();
        let platform_version = PlatformVersion::get(protocol_version).unwrap();

        let identities = outcome
            .abci_app
            .platform
            .drive
            .fetch_full_identities(
                outcome
                    .identities
                    .into_iter()
                    .map(|identity| identity.id().to_buffer())
                    .collect::<Vec<_>>()
                    .as_slice(),
                None,
                platform_version,
            )
            .expect("expected to fetch balances");

        assert!(identities
            .into_iter()
            .any(|(_, identity)| { identity.expect("expected identity").public_keys().len() > 7 }));
    }

    #[test]
    fn run_chain_update_identities_remove_keys() {
        let platform_version = PlatformVersion::latest();
        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![Operation {
                    op_type: OperationType::IdentityUpdate(
                        IdentityUpdateOp::IdentityUpdateDisableKey(3),
                    ),
                    frequency: Frequency {
                        times_per_block_range: 1..2,
                        chance_per_block: None,
                    },
                }],
                start_identities: StartIdentities::default(),
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
            // because we can add an identity and remove keys to it in the same block
            // the result would be different from expected
            verify_state_transition_results: false,
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

        let identities = outcome
            .abci_app
            .platform
            .drive
            .fetch_full_identities(
                outcome
                    .identities
                    .into_iter()
                    .map(|identity| identity.id().to_buffer())
                    .collect::<Vec<_>>()
                    .as_slice(),
                None,
                platform_version,
            )
            .expect("expected to fetch balances");

        assert!(identities.into_iter().any(|(_, identity)| {
            identity
                .expect("expected identity")
                .public_keys()
                .iter()
                .any(|(_, public_key)| public_key.is_disabled())
        }));
    }
}
