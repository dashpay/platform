#[cfg(test)]
mod tests {
    use tenderdash_abci::proto::types::CoreChainLock;

    use crate::execution::{continue_chain_for_strategy, run_chain_for_strategy};
    use crate::frequency::Frequency;
    use crate::strategy::{ChainExecutionOutcome, ChainExecutionParameters, MasternodeListChangesStrategy, Strategy, StrategyRandomness, UpgradingInfo};
    use drive_abci::config::{PlatformConfig, PlatformTestConfig};
    use drive_abci::test::helpers::setup::TestPlatformBuilder;

    #[test]
    fn run_chain_random_bans() {
        let strategy = Strategy {
            contracts_with_updates: vec![],
            operations: vec![],
            identities_inserts: Frequency {
                times_per_block_range: Default::default(),
                chance_per_block: None,
            },
            total_hpmns: 100,
            extra_normal_mns: 0,
            quorum_count: 24,
            upgrading_info: None,
            core_height_increase: Frequency {
                times_per_block_range: 1..2,
                chance_per_block: Some(0.2),
            },
            proposer_strategy: MasternodeListChangesStrategy {
                new_hpmns: Default::default(),
                removed_hpmns: Default::default(),
                updated_hpmns: Default::default(),
                banned_hpmns: Frequency { times_per_block_range: 1..2, chance_per_block: Some(0.01)} ,
                unbanned_hpmns: Default::default(),
                new_masternodes: Default::default(),
                removed_masternodes: Default::default(),
                updated_mastenodes: Default::default(),
                banned_masternodes: Default::default(),
                unbanned_masternodes: Default::default(),
            },
            rotate_quorums: true,
            failure_testing: None,
            query_testing: None,
            verify_state_transition_results: false,
        };
        let config = PlatformConfig {
            verify_sum_trees: true,
            quorum_size: 100,
            validator_set_quorum_rotation_block_count: 25,
            block_spacing_ms: 3000,
            testing_configs: PlatformTestConfig::default_with_no_block_signing(),
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        platform
            .core_rpc
            .expect_get_best_chain_lock()
            .returning(move || {
                Ok(CoreChainLock {
                    core_block_height: 10,
                    core_block_hash: [1; 32].to_vec(),
                    signature: [2; 96].to_vec(),
                })
            });
        run_chain_for_strategy(&mut platform, 50, strategy, config, 13);
    }
}