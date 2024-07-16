#[cfg(test)]
mod tests {

    use crate::execution::run_chain_for_strategy;
    use crate::strategy::CoreHeightIncrease::RandomCoreHeightIncrease;
    use crate::strategy::{MasternodeListChangesStrategy, NetworkStrategy};
    use drive_abci::config::{
        ExecutionConfig, InstantLockConfig, PlatformConfig, PlatformTestConfig,
    };

    use drive_abci::test::helpers::setup::TestPlatformBuilder;
    use strategy_tests::frequency::Frequency;
    use strategy_tests::{IdentityInsertInfo, StartIdentities, Strategy};

    #[test]
    fn run_chain_lock_update_quorums_not_changing() {
        // The point of this test is to check that chain locks can be validated in the
        // simple case where quorums do not change
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
            extra_normal_mns: 400,
            validator_quorum_count: 24,
            chain_lock_quorum_count: 4,
            upgrading_info: None,
            core_height_increase: RandomCoreHeightIncrease(Frequency {
                times_per_block_range: 1..2,
                chance_per_block: None,
            }),
            proposer_strategy: MasternodeListChangesStrategy {
                new_hpmns: Default::default(),
                removed_hpmns: Default::default(),
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
            independent_process_proposal_verification: true,
            sign_chain_locks: true,
            ..Default::default()
        };

        let config = PlatformConfig {
            instant_lock: InstantLockConfig::default_100_67(),
            execution: ExecutionConfig {
                verify_sum_trees: true,

                ..Default::default()
            },
            block_spacing_ms: 3000,
            testing_configs: PlatformTestConfig {
                block_signing: false,
                store_platform_state: false,
                block_commit_signature_verification: false,
                disable_instant_lock_signature_verification: true,
            },
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        run_chain_for_strategy(&mut platform, 50, strategy, config, 13, &mut None);
    }
}
