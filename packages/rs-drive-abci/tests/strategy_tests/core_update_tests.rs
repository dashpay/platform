#[cfg(test)]
mod tests {
    use dpp::dashcore::hashes::Hash;
    use dpp::dashcore::{BlockHash, ChainLock};
    use tenderdash_abci::proto::types::CoreChainLock;

    use crate::execution::run_chain_for_strategy;
    use crate::strategy::CoreHeightIncrease::RandomCoreHeightIncrease;
    use crate::strategy::{MasternodeListChangesStrategy, NetworkStrategy};
    use drive_abci::config::{ExecutionConfig, PlatformConfig, PlatformTestConfig};
    use drive_abci::platform_types::platform_state::v0::PlatformStateV0Methods;
    use drive_abci::platform_types::validator_set::v0::ValidatorSetV0Getters;
    use drive_abci::test::helpers::setup::TestPlatformBuilder;
    use strategy_tests::frequency::Frequency;
    use strategy_tests::Strategy;

    #[test]
    fn run_chain_random_bans() {
        let strategy = NetworkStrategy {
            strategy: Strategy {
                contracts_with_updates: vec![],
                operations: vec![],
                start_identities: vec![],
                identities_inserts: Frequency {
                    times_per_block_range: Default::default(),
                    chance_per_block: None,
                },
                signer: None,
            },
            total_hpmns: 100,
            extra_normal_mns: 0,
            validator_quorum_count: 24,
            chain_lock_quorum_count: 24,
            upgrading_info: None,
            core_height_increase: RandomCoreHeightIncrease(Frequency {
                times_per_block_range: 1..2,
                chance_per_block: None,
            }),
            proposer_strategy: MasternodeListChangesStrategy {
                new_hpmns: Default::default(),
                removed_hpmns: Default::default(),
                updated_hpmns: Default::default(),
                banned_hpmns: Frequency {
                    times_per_block_range: 1..2,
                    chance_per_block: Some(0.1),
                },
                unbanned_hpmns: Default::default(),
                changed_ip_hpmns: Default::default(),
                changed_p2p_port_hpmns: Default::default(),
                changed_http_port_hpmns: Default::default(),
                new_masternodes: Default::default(),
                removed_masternodes: Default::default(),
                updated_masternodes: Default::default(),
                banned_masternodes: Default::default(),
                unbanned_masternodes: Default::default(),
                changed_ip_masternodes: Default::default(),
            },
            rotate_quorums: true,
            failure_testing: None,
            query_testing: None,
            verify_state_transition_results: false,
            ..Default::default()
        };

        let quorum_size = 100;

        let config = PlatformConfig {
            validator_set_quorum_size: quorum_size,
            validator_set_quorum_type: "llmq_100_67".to_string(),
            chain_lock_quorum_type: "llmq_100_67".to_string(),
            execution: ExecutionConfig {
                verify_sum_trees: true,
                validator_set_rotation_block_count: 25,
                ..Default::default()
            },
            block_spacing_ms: 3000,
            testing_configs: PlatformTestConfig::default_with_no_block_signing(),
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let outcome = run_chain_for_strategy(&mut platform, 50, strategy, config, 13);

        // we expect to see quorums with banned members

        let state = outcome.abci_app.platform.state.read().unwrap();

        let banned_count = state
            .validator_sets()
            .values()
            .map(|validator_set| {
                validator_set
                    .members()
                    .values()
                    .map(|validator| validator.is_banned as usize)
                    .sum::<usize>()
            })
            .sum::<usize>();

        assert!(banned_count > 1);

        // We should also see validator sets with less than the quorum size

        let has_smaller_validator_sets =
            outcome
                .validator_set_updates
                .into_iter()
                .any(|(_, validator_set_update)| {
                    (validator_set_update.validator_updates.len() as u16) < quorum_size
                });
        assert!(has_smaller_validator_sets);
    }

    #[test]
    fn run_chain_random_removals() {
        let strategy = NetworkStrategy {
            strategy: Strategy {
                contracts_with_updates: vec![],
                operations: vec![],
                start_identities: vec![],
                identities_inserts: Frequency {
                    times_per_block_range: Default::default(),
                    chance_per_block: None,
                },
                signer: None,
            },
            total_hpmns: 100,
            extra_normal_mns: 0,
            validator_quorum_count: 24,
            chain_lock_quorum_count: 24,
            upgrading_info: None,
            core_height_increase: RandomCoreHeightIncrease(Frequency {
                times_per_block_range: 1..2,
                chance_per_block: None,
            }),
            proposer_strategy: MasternodeListChangesStrategy {
                new_hpmns: Default::default(),
                removed_hpmns: Frequency {
                    times_per_block_range: 1..2,
                    chance_per_block: Some(0.1),
                },
                updated_hpmns: Default::default(),
                banned_hpmns: Default::default(),
                unbanned_hpmns: Default::default(),
                changed_ip_hpmns: Default::default(),
                changed_p2p_port_hpmns: Default::default(),
                changed_http_port_hpmns: Default::default(),
                new_masternodes: Default::default(),
                removed_masternodes: Default::default(),
                updated_masternodes: Default::default(),
                banned_masternodes: Default::default(),
                unbanned_masternodes: Default::default(),
                changed_ip_masternodes: Default::default(),
            },
            rotate_quorums: true,
            failure_testing: None,
            query_testing: None,
            verify_state_transition_results: false,
            ..Default::default()
        };

        let quorum_size = 100;

        let config = PlatformConfig {
            validator_set_quorum_size: quorum_size,
            validator_set_quorum_type: "llmq_100_67".to_string(),
            chain_lock_quorum_type: "llmq_100_67".to_string(),
            execution: ExecutionConfig {
                verify_sum_trees: true,
                validator_set_rotation_block_count: 25,
                ..Default::default()
            },
            block_spacing_ms: 3000,
            testing_configs: PlatformTestConfig::default_with_no_block_signing(),
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let outcome = run_chain_for_strategy(&mut platform, 50, strategy, config, 13);

        // we expect to see quorums with banned members

        let _state = outcome.abci_app.platform.state.read().unwrap();

        // We should also see validator sets with less than the quorum size

        let has_smaller_validator_sets =
            outcome
                .validator_set_updates
                .into_iter()
                .any(|(_, validator_set_update)| {
                    (validator_set_update.validator_updates.len() as u16) < quorum_size
                });
        assert!(has_smaller_validator_sets);
    }

    #[test]
    fn run_chain_random_bans_and_unbans() {
        let strategy = NetworkStrategy {
            strategy: Strategy {
                contracts_with_updates: vec![],
                operations: vec![],
                start_identities: vec![],
                identities_inserts: Frequency {
                    times_per_block_range: Default::default(),
                    chance_per_block: None,
                },
                signer: None,
            },
            total_hpmns: 100,
            extra_normal_mns: 0,
            validator_quorum_count: 24,
            chain_lock_quorum_count: 24,
            upgrading_info: None,
            core_height_increase: RandomCoreHeightIncrease(Frequency {
                times_per_block_range: 1..2,
                chance_per_block: None,
            }),
            proposer_strategy: MasternodeListChangesStrategy {
                new_hpmns: Default::default(),
                removed_hpmns: Default::default(),
                updated_hpmns: Default::default(),
                banned_hpmns: Frequency {
                    times_per_block_range: 1..2,
                    chance_per_block: Some(0.1), //lower chance of banning
                },
                unbanned_hpmns: Frequency {
                    times_per_block_range: 1..2,
                    chance_per_block: Some(0.3), //higher chance of unbanning
                },
                changed_ip_hpmns: Default::default(),
                changed_p2p_port_hpmns: Default::default(),
                changed_http_port_hpmns: Default::default(),
                new_masternodes: Default::default(),
                removed_masternodes: Default::default(),
                updated_masternodes: Default::default(),
                banned_masternodes: Default::default(),
                unbanned_masternodes: Default::default(),
                changed_ip_masternodes: Default::default(),
            },
            rotate_quorums: true,
            failure_testing: None,
            query_testing: None,
            verify_state_transition_results: false,
            ..Default::default()
        };

        let quorum_size = 100;

        let config = PlatformConfig {
            validator_set_quorum_size: quorum_size,
            validator_set_quorum_type: "llmq_100_67".to_string(),
            chain_lock_quorum_type: "llmq_100_67".to_string(),
            execution: ExecutionConfig {
                verify_sum_trees: true,
                validator_set_rotation_block_count: 25,
                ..Default::default()
            },
            block_spacing_ms: 3000,
            testing_configs: PlatformTestConfig::default_with_no_block_signing(),
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let outcome = run_chain_for_strategy(&mut platform, 26, strategy, config, 13);

        // We should also see validator sets with less than the quorum size

        let validator_set_sizes: Vec<(u64, u16)> = outcome
            .validator_set_updates
            .into_iter()
            .map(|(height, validator_set_update)| {
                (height, validator_set_update.validator_updates.len() as u16)
            })
            .collect();

        let mut found_smaller_then_bigger = false;
        for i in 0..(validator_set_sizes.len() - 1) {
            let (_height_i, size_i) = validator_set_sizes[i];
            if validator_set_sizes
                .iter()
                .skip(i + 1)
                .any(|&(_height_j, size_j)| size_j > size_i)
            {
                found_smaller_then_bigger = true;
                break;
            }
        }

        assert!(
            found_smaller_then_bigger,
            "No instances found where validator set size got smaller and then bigger again"
        );
    }
}
