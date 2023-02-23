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
                upgrade_three_quarters_life: 0.1,
            }),
        };
        let config = PlatformConfig {
            verify_sum_trees: true,
            quorum_size: 100,
            validator_set_quorum_rotation_block_count: 125,
            ..Default::default()
        };
        let twenty_minutes_in_ms = 1000 * 60 * 20;
        let ChainExecutionOutcome {
            platform,
            proposers,
            current_proposers,
            current_proposer_versions,
            end_time_ms,
            ..
        } = run_chain_for_strategy(
            1300,
            twenty_minutes_in_ms,
            strategy.clone(),
            config.clone(),
            15,
        );
        {
            let drive_cache = platform.drive.cache.borrow_mut();
            let counter = drive_cache
                .protocol_versions_counter
                .as_ref()
                .expect("expected a version counter");
            platform
                .drive
                .fetch_versions_with_counter(None)
                .expect("expected to get versions");

            assert_eq!(
                platform
                    .state
                    .borrow()
                    .last_block_info
                    .as_ref()
                    .unwrap()
                    .epoch
                    .index,
                0
            );
            assert_eq!(
                platform
                    .state
                    .borrow()
                    .current_protocol_version_in_consensus,
                1
            );
            assert_eq!(counter.get(&1), Some(&13)); //most nodes were hit (60 were not)
            assert_eq!(counter.get(&2), Some(&400)); //most nodes were hit (60 were not)
        }

        // we did not yet hit the epoch change
        // let's go a little longer

        let hour_in_ms = 1000 * 60 * 60;
        let block_start = platform
            .state
            .borrow()
            .last_block_info
            .as_ref()
            .unwrap()
            .height
            + 1;
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
                block_count: 200,
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
                .protocol_versions_counter
                .as_ref()
                .expect("expected a version counter");
            assert_eq!(
                platform
                    .state
                    .borrow()
                    .last_block_info
                    .as_ref()
                    .unwrap()
                    .epoch
                    .index,
                1
            );
            assert_eq!(
                platform
                    .state
                    .borrow()
                    .current_protocol_version_in_consensus,
                1
            );
            assert_eq!(platform.state.borrow().next_epoch_protocol_version, 2);
            assert_eq!(counter.get(&1), None); //no one has proposed 1 yet
            assert_eq!(counter.get(&2), Some(&154));
        }

        // we locked in
        // let's go a little longer to see activation

        let block_start = platform
            .state
            .borrow()
            .last_block_info
            .as_ref()
            .unwrap()
            .height
            + 1;
        let ChainExecutionOutcome {
            platform,
            
            
            
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
            StrategyRandomness::SeedEntropy(18),
        );
        {
            let drive_cache = platform.drive.cache.borrow_mut();
            let counter = drive_cache
                .protocol_versions_counter
                .as_ref()
                .expect("expected a version counter");
            assert_eq!(
                platform
                    .state
                    .borrow()
                    .last_block_info
                    .as_ref()
                    .unwrap()
                    .epoch
                    .index,
                2
            );
            assert_eq!(
                platform
                    .state
                    .borrow()
                    .current_protocol_version_in_consensus,
                2
            );
            assert_eq!(platform.state.borrow().next_epoch_protocol_version, 2);
            assert_eq!(counter.get(&1), None); //no one has proposed 1 yet
            assert_eq!(counter.get(&2), Some(&124));
        }
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
            total_hpmns: 120,
            upgrading_info: Some(UpgradingInfo {
                current_protocol_version: 1,
                proposed_protocol_versions_with_weight: vec![(2, 1)],
                upgrade_three_quarters_life: 5.0, //it will take an epoch before we get enough nodes
            }),
        };
        let config = PlatformConfig {
            verify_sum_trees: true,
            quorum_size: 40,
            validator_set_quorum_rotation_block_count: 50,
            ..Default::default()
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
            let _counter = drive_cache
                .protocol_versions_counter
                .as_ref()
                .expect("expected a version counter");
            assert_eq!(
                platform
                    .state
                    .borrow()
                    .last_block_info
                    .as_ref()
                    .unwrap()
                    .epoch
                    .index,
                4
            );
            assert_eq!(
                platform
                    .state
                    .borrow()
                    .current_protocol_version_in_consensus,
                1
            );
            assert_eq!(platform.state.borrow().next_epoch_protocol_version, 1);
        }

        // we did not yet hit the required threshold to upgrade
        // let's go a little longer

        let block_start = platform
            .state
            .borrow()
            .last_block_info
            .as_ref()
            .unwrap()
            .height
            + 1;
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
                block_count: 1600,
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
                .protocol_versions_counter
                .as_ref()
                .expect("expected a version counter");
            assert_eq!(
                platform
                    .state
                    .borrow()
                    .last_block_info
                    .as_ref()
                    .unwrap()
                    .epoch
                    .index,
                8
            );
            assert_eq!(
                platform
                    .state
                    .borrow()
                    .current_protocol_version_in_consensus,
                1
            );
            assert_eq!(platform.state.borrow().next_epoch_protocol_version, 2);
            // the counter is for the current voting during that window
            assert_eq!((counter.get(&1), counter.get(&2)), (Some(&13), Some(&54)));
        }

        // we are now locked in, the current protocol version will change on next epoch

        let block_start = platform
            .state
            .borrow()
            .last_block_info
            .as_ref()
            .unwrap()
            .height
            + 1;
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
            let _counter = drive_cache
                .protocol_versions_counter
                .as_ref()
                .expect("expected a version counter");
            assert_eq!(
                platform
                    .state
                    .borrow()
                    .last_block_info
                    .as_ref()
                    .unwrap()
                    .epoch
                    .index,
                9
            );
            assert_eq!(
                platform
                    .state
                    .borrow()
                    .current_protocol_version_in_consensus,
                2
            );
            assert_eq!(platform.state.borrow().next_epoch_protocol_version, 2);
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
            total_hpmns: 200,
            upgrading_info: Some(UpgradingInfo {
                current_protocol_version: 1,
                proposed_protocol_versions_with_weight: vec![(2, 1)],
                upgrade_three_quarters_life: 5.0,
            }),
        };
        let config = PlatformConfig {
            verify_sum_trees: true,
            quorum_size: 50,
            validator_set_quorum_rotation_block_count: 60,
            ..Default::default()
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
            let _counter = drive_cache
                .protocol_versions_counter
                .as_ref()
                .expect("expected a version counter");
            assert_eq!(
                platform
                    .state
                    .borrow()
                    .last_block_info
                    .as_ref()
                    .unwrap()
                    .epoch
                    .index,
                4
            );
            assert_eq!(
                platform
                    .state
                    .borrow()
                    .current_protocol_version_in_consensus,
                1
            );
        }

        // we still did not yet hit the required threshold to upgrade
        // let's go a just a little longer

        let block_start = platform
            .state
            .borrow()
            .last_block_info
            .as_ref()
            .unwrap()
            .height
            + 1;
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
                block_count: 3000,
                block_spacing_ms: hour_in_ms,
                proposers,
                current_proposers,
                current_proposer_versions: Some(current_proposer_versions.clone()),
                current_time_ms: end_time_ms,
            },
            strategy.clone(),
            config.clone(),
            StrategyRandomness::SeedEntropy(99),
        );
        {
            let drive_cache = platform.drive.cache.borrow_mut();
            let counter = drive_cache
                .protocol_versions_counter
                .as_ref()
                .expect("expected a version counter");
            assert_eq!(
                platform
                    .state
                    .borrow()
                    .last_block_info
                    .as_ref()
                    .unwrap()
                    .epoch
                    .index,
                11
            );
            assert_eq!(
                platform
                    .state
                    .borrow()
                    .current_protocol_version_in_consensus,
                1
            );
            assert_eq!(platform.state.borrow().next_epoch_protocol_version, 2);
            assert_eq!((counter.get(&1), counter.get(&2)), (Some(&11), Some(&105)));
            //not all nodes have upgraded
        }

        // we are now locked in, the current protocol version will change on next epoch
        // however most nodes now rever

        let strategy = Strategy {
            contracts: vec![],
            operations: vec![],
            identities_inserts: Frequency {
                times_per_block_range: Default::default(),
                chance_per_block: None,
            },
            total_hpmns: 200,
            upgrading_info: Some(UpgradingInfo {
                current_protocol_version: 2,
                proposed_protocol_versions_with_weight: vec![(1, 9), (2, 1)],
                upgrade_three_quarters_life: 0.1,
            }),
        };

        let block_start = platform
            .state
            .borrow()
            .last_block_info
            .as_ref()
            .unwrap()
            .height
            + 1;
        let ChainExecutionOutcome {
            platform,
            proposers,
            current_proposers,
            current_proposer_versions,
            end_time_ms,
            ..
        } = continue_chain_for_strategy(
            platform,
            ChainExecutionParameters {
                block_start,
                block_count: 2000,
                block_spacing_ms: hour_in_ms / 5, //speed things up
                proposers,
                current_proposers,
                current_proposer_versions: None, //restart the proposer versions
                current_time_ms: end_time_ms,
            },
            strategy.clone(),
            config.clone(),
            StrategyRandomness::SeedEntropy(40),
        );
        {
            let drive_cache = platform.drive.cache.borrow_mut();
            let counter = drive_cache
                .protocol_versions_counter
                .as_ref()
                .expect("expected a version counter");
            assert_eq!((counter.get(&1), counter.get(&2)), (Some(&170), Some(&23)));
            //a lot nodes reverted to previous version, however this won't impact things
            assert_eq!(
                platform
                    .state
                    .borrow()
                    .last_block_info
                    .as_ref()
                    .unwrap()
                    .epoch
                    .index,
                12
            );
            assert_eq!(
                platform
                    .state
                    .borrow()
                    .current_protocol_version_in_consensus,
                2
            );
            assert_eq!(platform.state.borrow().next_epoch_protocol_version, 1);
        }

        let block_start = platform
            .state
            .borrow()
            .last_block_info
            .as_ref()
            .unwrap()
            .height
            + 1;
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
                .protocol_versions_counter
                .as_ref()
                .expect("expected a version counter");
            assert_eq!((counter.get(&1), counter.get(&2)), (Some(&22), Some(&2)));
            assert_eq!(
                platform
                    .state
                    .borrow()
                    .last_block_info
                    .as_ref()
                    .unwrap()
                    .epoch
                    .index,
                13
            );
            assert_eq!(
                platform
                    .state
                    .borrow()
                    .current_protocol_version_in_consensus,
                1
            );
            assert_eq!(platform.state.borrow().next_epoch_protocol_version, 1);
        }
    }

    #[test]
    fn run_chain_version_upgrade_multiple_versions() {
        let strategy = Strategy {
            contracts: vec![],
            operations: vec![],
            identities_inserts: Frequency {
                times_per_block_range: Default::default(),
                chance_per_block: None,
            },
            total_hpmns: 200,
            upgrading_info: Some(UpgradingInfo {
                current_protocol_version: 1,
                proposed_protocol_versions_with_weight: vec![(1, 3), (2, 95), (3, 2)],
                upgrade_three_quarters_life: 0.75,
            }),
        };
        let config = PlatformConfig {
            verify_sum_trees: true,
            quorum_size: 50,
            validator_set_quorum_rotation_block_count: 60,
            ..Default::default()
        };
        let hour_in_ms = 1000 * 60 * 60;
        let ChainExecutionOutcome {
            platform,
            proposers,
            current_proposers,
            
            end_time_ms,
            ..
        } = run_chain_for_strategy(1400, hour_in_ms, strategy.clone(), config.clone(), 15);
        {
            let drive_cache = platform.drive.cache.borrow_mut();
            let counter = drive_cache
                .protocol_versions_counter
                .as_ref()
                .expect("expected a version counter");

            assert_eq!(
                platform
                    .state
                    .borrow()
                    .last_block_info
                    .as_ref()
                    .unwrap()
                    .epoch
                    .index,
                3
            );
            assert_eq!(
                platform
                    .state
                    .borrow()
                    .current_protocol_version_in_consensus,
                1
            );
            assert_eq!(platform.state.borrow().next_epoch_protocol_version, 2);
            assert_eq!(
                (counter.get(&1), counter.get(&2), counter.get(&3)),
                (Some(&3), Some(&59), Some(&4))
            ); //some nodes reverted to previous version
        }

        let strategy = Strategy {
            contracts: vec![],
            operations: vec![],
            identities_inserts: Frequency {
                times_per_block_range: Default::default(),
                chance_per_block: None,
            },
            total_hpmns: 200,
            upgrading_info: Some(UpgradingInfo {
                current_protocol_version: 1,
                proposed_protocol_versions_with_weight: vec![(2, 3), (3, 97)],
                upgrade_three_quarters_life: 0.5,
            }),
        };

        // we hit the required threshold to upgrade
        // let's go a little longer

        let block_start = platform
            .state
            .borrow()
            .last_block_info
            .as_ref()
            .unwrap()
            .height
            + 1;
        let ChainExecutionOutcome {
            platform,
            
            
            
            ..
        } = continue_chain_for_strategy(
            platform,
            ChainExecutionParameters {
                block_start,
                block_count: 700,
                block_spacing_ms: hour_in_ms,
                proposers,
                current_proposers,
                current_proposer_versions: None,
                current_time_ms: end_time_ms,
            },
            strategy.clone(),
            config.clone(),
            StrategyRandomness::SeedEntropy(7),
        );
        {
            let drive_cache = platform.drive.cache.borrow_mut();
            let counter = drive_cache
                .protocol_versions_counter
                .as_ref()
                .expect("expected a version counter");
            assert_eq!(
                platform
                    .state
                    .borrow()
                    .last_block_info
                    .as_ref()
                    .unwrap()
                    .epoch
                    .index,
                4
            );
            assert_eq!(
                platform
                    .state
                    .borrow()
                    .current_protocol_version_in_consensus,
                2
            );
            assert_eq!(platform.state.borrow().next_epoch_protocol_version, 3);
            assert_eq!(
                (counter.get(&1), counter.get(&2), counter.get(&3)),
                (None, Some(&6), Some(&154))
            );
        }
    }
}
