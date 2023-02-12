#[cfg(test)]
mod tests {

    use crate::{
        continue_chain_for_strategy, run_chain_for_strategy, ChainExecutionOutcome,
        ChainExecutionParameters, Frequency, Strategy, StrategyRandomness, UpgradingInfo,
    };

    use drive_abci::config::PlatformConfig;

    #[test]
    fn run_chain_version_upgrade() {
        let strategy = Strategy {
            contracts: vec![],
            operations: vec![],
            identities_inserts: Frequency {
                times_per_block_range: Default::default(),
                chance_per_block: None,
            },
            total_hpmns: 460,
            upgrading_info: Some(UpgradingInfo {
                current_protocol_version: 1,
                proposed_protocol_versions_with_weight: vec![(2, 1)],
                upgrade_three_quarters_life: 0.75,
            }),
        };
        let config = PlatformConfig {
            drive_config: Default::default(),
            verify_sum_trees: true,
            quorum_size: 100,
            quorum_rotation_block_count: 125,
        };
        let hour_in_ms = 1000 * 60 * 60;
        let ChainExecutionOutcome { platform, .. } =
            run_chain_for_strategy(2000, hour_in_ms, strategy, config, 15);
        let drive_cache = platform.drive.cache.borrow_mut();
        let counter = drive_cache
            .versions_counter
            .as_ref()
            .expect("expected a version counter");
        platform
            .drive
            .fetch_versions_with_counter(None)
            .expect("expected to get versions");
        assert_eq!(counter.get(&1), Some(&0)); //all nodes upgraded
        assert_eq!(counter.get(&2), Some(&451)); //most nodes were hit (12 were not)
        assert_eq!(platform.state.last_block_info.unwrap().epoch.index, 4);
        assert_eq!(platform.state.current_protocol_version_in_consensus, 2);
    }

    #[test]
    fn run_chain_version_upgrade_slow_upgrade() {
        let strategy = Strategy {
            contracts: vec![],
            operations: vec![],
            identities_inserts: Frequency {
                times_per_block_range: Default::default(),
                chance_per_block: None,
            },
            total_hpmns: 460,
            upgrading_info: Some(UpgradingInfo {
                current_protocol_version: 1,
                proposed_protocol_versions_with_weight: vec![(2, 1)],
                upgrade_three_quarters_life: 5.0,
            }),
        };
        let config = PlatformConfig {
            drive_config: Default::default(),
            verify_sum_trees: true,
            quorum_size: 100,
            quorum_rotation_block_count: 125,
        };
        let hour_in_ms = 1000 * 60 * 60;
        let ChainExecutionOutcome {
            platform,
            proposers,
            current_proposers,
            current_proposer_versions,
            end_time_ms,
            ..
        } = run_chain_for_strategy(2000, hour_in_ms, strategy.clone(), config.clone(), 15);
        {
            let drive_cache = platform.drive.cache.borrow_mut();
            let counter = drive_cache
                .versions_counter
                .as_ref()
                .expect("expected a version counter");
            assert_eq!(counter.get(&1), Some(&263)); //not all nodes have upgraded
            assert_eq!(counter.get(&2), Some(&183)); //most nodes were hit (12 were not)
            assert_eq!(
                platform.state.last_block_info.as_ref().unwrap().epoch.index,
                4
            );
            assert_eq!(platform.state.current_protocol_version_in_consensus, 1);
        }

        // we did not yet hit the required threshold to upgrade
        // let's go a little longer

        let block_start = platform.state.last_block_info.as_ref().unwrap().height + 1;
        let ChainExecutionOutcome {
            platform,
            proposers,
            current_proposers,
            end_time_ms,
            ..
        } = continue_chain_for_strategy(
            platform,
            ChainExecutionParameters {
                block_start,
                block_count: 2500,
                block_spacing_ms: hour_in_ms,
                proposers,
                current_proposers,
                current_proposer_versions: Some(current_proposer_versions.clone()),
                current_time_ms: end_time_ms,
            },
            strategy.clone(),
            config.clone(),
            StrategyRandomness::SeedEntropy(7),
        );
        {
            let drive_cache = platform.drive.cache.borrow_mut();
            let counter = drive_cache
                .versions_counter
                .as_ref()
                .expect("expected a version counter");
            assert_eq!(counter.get(&1), Some(&117)); //not all nodes have upgraded
            assert_eq!(counter.get(&2), Some(&343)); //all nodes were hit (we need 345 to upgrade)
            assert_eq!(
                platform.state.last_block_info.as_ref().unwrap().epoch.index,
                10
            );
            assert_eq!(platform.state.current_protocol_version_in_consensus, 1);
            assert_eq!(platform.state.next_epoch_protocol_version, 1);
        }

        // we still did not yet hit the required threshold to upgrade
        // let's go a just a little longer

        let block_start = platform.state.last_block_info.as_ref().unwrap().height + 1;
        let ChainExecutionOutcome {
            platform,
            proposers,
            current_proposers,
            end_time_ms,
            ..
        } = continue_chain_for_strategy(
            platform,
            ChainExecutionParameters {
                block_start,
                block_count: 400,
                block_spacing_ms: hour_in_ms,
                proposers,
                current_proposers,
                current_proposer_versions: Some(current_proposer_versions.clone()),
                current_time_ms: end_time_ms,
            },
            strategy.clone(),
            config.clone(),
            StrategyRandomness::SeedEntropy(7),
        );
        {
            let drive_cache = platform.drive.cache.borrow_mut();
            let counter = drive_cache
                .versions_counter
                .as_ref()
                .expect("expected a version counter");
            assert_eq!((counter.get(&1), counter.get(&2)), (Some(&103), Some(&357))); //not all nodes have upgraded
            assert_eq!(
                platform.state.last_block_info.as_ref().unwrap().epoch.index,
                11
            );
            assert_eq!(platform.state.current_protocol_version_in_consensus, 1);
            assert_eq!(platform.state.next_epoch_protocol_version, 2);
        }

        // we are now locked in, the current protocol version will change on next epoch

        let block_start = platform.state.last_block_info.as_ref().unwrap().height + 1;
        let ChainExecutionOutcome { platform, .. } = continue_chain_for_strategy(
            platform,
            ChainExecutionParameters {
                block_start,
                block_count: 400,
                block_spacing_ms: hour_in_ms,
                proposers,
                current_proposers,
                current_proposer_versions: Some(current_proposer_versions.clone()),
                current_time_ms: end_time_ms,
            },
            strategy,
            config,
            StrategyRandomness::SeedEntropy(8),
        );
        {
            let drive_cache = platform.drive.cache.borrow_mut();
            let counter = drive_cache
                .versions_counter
                .as_ref()
                .expect("expected a version counter");
            assert_eq!((counter.get(&1), counter.get(&2)), (Some(&85), Some(&375))); //some nodes reverted to previous version
            assert_eq!(platform.state.last_block_info.unwrap().epoch.index, 12);
            assert_eq!(platform.state.current_protocol_version_in_consensus, 2);
            assert_eq!(platform.state.next_epoch_protocol_version, 2);
        }
    }

    #[test]
    fn run_chain_version_upgrade_slow_upgrade_quick_reversion_after_lock_in() {
        let strategy = Strategy {
            contracts: vec![],
            operations: vec![],
            identities_inserts: Frequency {
                times_per_block_range: Default::default(),
                chance_per_block: None,
            },
            total_hpmns: 460,
            upgrading_info: Some(UpgradingInfo {
                current_protocol_version: 1,
                proposed_protocol_versions_with_weight: vec![(2, 1)],
                upgrade_three_quarters_life: 5.0,
            }),
        };
        let config = PlatformConfig {
            drive_config: Default::default(),
            verify_sum_trees: true,
            quorum_size: 100,
            quorum_rotation_block_count: 125,
        };
        let hour_in_ms = 1000 * 60 * 60;
        let ChainExecutionOutcome {
            platform,
            proposers,
            current_proposers,
            current_proposer_versions,
            end_time_ms,
            ..
        } = run_chain_for_strategy(2000, hour_in_ms, strategy.clone(), config.clone(), 15);
        {
            let drive_cache = platform.drive.cache.borrow_mut();
            let counter = drive_cache
                .versions_counter
                .as_ref()
                .expect("expected a version counter");
            assert_eq!(counter.get(&1), Some(&263)); //not all nodes have upgraded
            assert_eq!(counter.get(&2), Some(&183)); //most nodes were hit (12 were not)
            assert_eq!(
                platform.state.last_block_info.as_ref().unwrap().epoch.index,
                4
            );
            assert_eq!(platform.state.current_protocol_version_in_consensus, 1);
        }

        // we did not yet hit the required threshold to upgrade
        // let's go a little longer

        let block_start = platform.state.last_block_info.as_ref().unwrap().height + 1;
        let ChainExecutionOutcome {
            platform,
            proposers,
            current_proposers,
            end_time_ms,
            ..
        } = continue_chain_for_strategy(
            platform,
            ChainExecutionParameters {
                block_start,
                block_count: 2000,
                block_spacing_ms: hour_in_ms,
                proposers,
                current_proposers,
                current_proposer_versions: Some(current_proposer_versions.clone()),
                current_time_ms: end_time_ms,
            },
            strategy.clone(),
            config.clone(),
            StrategyRandomness::SeedEntropy(7),
        );
        {
            let drive_cache = platform.drive.cache.borrow_mut();
            let counter = drive_cache
                .versions_counter
                .as_ref()
                .expect("expected a version counter");
            assert_eq!((counter.get(&1), counter.get(&2)), (Some(&404), Some(&56)));
            assert_eq!(
                platform.state.last_block_info.as_ref().unwrap().epoch.index,
                9
            );
            assert_eq!(platform.state.current_protocol_version_in_consensus, 1);
        }

        // we still did not yet hit the required threshold to upgrade
        // let's go a just a little longer

        let block_start = platform.state.last_block_info.as_ref().unwrap().height + 1;
        let ChainExecutionOutcome {
            platform,
            proposers,
            current_proposers,
            end_time_ms,
            ..
        } = continue_chain_for_strategy(
            platform,
            ChainExecutionParameters {
                block_start,
                block_count: 800,
                block_spacing_ms: hour_in_ms,
                proposers,
                current_proposers,
                current_proposer_versions: Some(current_proposer_versions.clone()),
                current_time_ms: end_time_ms,
            },
            strategy.clone(),
            config.clone(),
            StrategyRandomness::SeedEntropy(7),
        );
        {
            let drive_cache = platform.drive.cache.borrow_mut();
            let counter = drive_cache
                .versions_counter
                .as_ref()
                .expect("expected a version counter");
            assert_eq!((counter.get(&1), counter.get(&2)), (Some(&106), Some(&354))); //not all nodes have upgraded
            assert_eq!(
                platform.state.last_block_info.as_ref().unwrap().epoch.index,
                10
            );
            assert_eq!(platform.state.current_protocol_version_in_consensus, 1);
            assert_eq!(platform.state.next_epoch_protocol_version, 2);
        }

        // we are now locked in, the current protocol version will change on next epoch

        let strategy = Strategy {
            contracts: vec![],
            operations: vec![],
            identities_inserts: Frequency {
                times_per_block_range: Default::default(),
                chance_per_block: None,
            },
            total_hpmns: 460,
            upgrading_info: Some(UpgradingInfo {
                current_protocol_version: 2,
                proposed_protocol_versions_with_weight: vec![(1, 9), (2, 1)],
                upgrade_three_quarters_life: 0.1,
            }),
        };

        let block_start = platform.state.last_block_info.as_ref().unwrap().height + 1;
        let ChainExecutionOutcome {
            platform,
            proposers,
            current_proposers,
            end_time_ms,
            ..
        } = continue_chain_for_strategy(
            platform,
            ChainExecutionParameters {
                block_start,
                block_count: 2000,
                block_spacing_ms: hour_in_ms / 10, //speed things up
                proposers,
                current_proposers,
                current_proposer_versions: Some(current_proposer_versions.clone()),
                current_time_ms: end_time_ms,
            },
            strategy.clone(),
            config.clone(),
            StrategyRandomness::SeedEntropy(40),
        );
        {
            let drive_cache = platform.drive.cache.borrow_mut();
            let counter = drive_cache
                .versions_counter
                .as_ref()
                .expect("expected a version counter");
            assert_eq!((counter.get(&1), counter.get(&2)), (Some(&403), Some(&57)));
            //a lot nodes reverted to previous version, however this won't impact things
            assert_eq!(
                platform.state.last_block_info.as_ref().unwrap().epoch.index,
                11
            );
            assert_eq!(platform.state.current_protocol_version_in_consensus, 2);
            assert_eq!(platform.state.next_epoch_protocol_version, 2);
        }

        let block_start = platform.state.last_block_info.as_ref().unwrap().height + 1;
        let ChainExecutionOutcome { platform, .. } = continue_chain_for_strategy(
            platform,
            ChainExecutionParameters {
                block_start,
                block_count: 100,
                block_spacing_ms: hour_in_ms * 4, //let's try to move to next epoch
                proposers,
                current_proposers,
                current_proposer_versions: Some(current_proposer_versions.clone()),
                current_time_ms: end_time_ms,
            },
            strategy,
            config,
            StrategyRandomness::SeedEntropy(40),
        );
        {
            let drive_cache = platform.drive.cache.borrow_mut();
            let counter = drive_cache
                .versions_counter
                .as_ref()
                .expect("expected a version counter");
            assert_eq!((counter.get(&1), counter.get(&2)), (Some(&404), Some(&56)));
            //a lot nodes reverted to previous version, however this won't impact things
            assert_eq!(platform.state.last_block_info.unwrap().epoch.index, 12);
            assert_eq!(platform.state.current_protocol_version_in_consensus, 2);
            assert_eq!(platform.state.next_epoch_protocol_version, 1);
        }
    }

    #[test]
    fn run_chain_version_upgrade_twice_back_to_back() {
        let strategy = Strategy {
            contracts: vec![],
            operations: vec![],
            identities_inserts: Frequency {
                times_per_block_range: Default::default(),
                chance_per_block: None,
            },
            total_hpmns: 460,
            upgrading_info: Some(UpgradingInfo {
                current_protocol_version: 1,
                proposed_protocol_versions_with_weight: vec![(2, 8), (2, 8), (3, 1)],
                upgrade_three_quarters_life: 0.75,
            }),
        };
        let config = PlatformConfig {
            drive_config: Default::default(),
            verify_sum_trees: true,
            quorum_size: 100,
            quorum_rotation_block_count: 125,
        };
        let hour_in_ms = 1000 * 60 * 60;
        let ChainExecutionOutcome { platform, .. } =
            run_chain_for_strategy(2000, hour_in_ms, strategy, config, 15);
        let drive_cache = platform.drive.cache.borrow_mut();
        let counter = drive_cache
            .versions_counter
            .as_ref()
            .expect("expected a version counter");
        platform
            .drive
            .fetch_versions_with_counter(None)
            .expect("expected to get versions");
        assert_eq!(counter.get(&1), Some(&0)); //all nodes upgraded
        assert_eq!(counter.get(&2), Some(&419)); //most nodes were hit (12 were not)
        assert_eq!(platform.state.last_block_info.unwrap().epoch.index, 4);
        assert_eq!(platform.state.current_protocol_version_in_consensus, 2);
    }
}
