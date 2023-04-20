#[cfg(test)]
mod tests {
    use crate::{
        continue_chain_for_strategy, run_chain_for_strategy, ChainExecutionOutcome,
        ChainExecutionParameters, Frequency, Strategy, StrategyRandomness, UpgradingInfo,
    };

    use tenderdash_abci::proto::types::CoreChainLock;

    use drive_abci::config::{PlatformConfig, PlatformTestConfig};
    use drive_abci::test::helpers::setup::TestPlatformBuilder;

    #[test]
    fn run_chain_version_upgrade() {
        let strategy = Strategy {
            contracts_with_updates: vec![],
            operations: vec![],
            identities_inserts: Frequency {
                times_per_block_range: Default::default(),
                chance_per_block: None,
            },
            total_hpmns: 460,
            extra_normal_mns: 0,
            quorum_count: 24,
            upgrading_info: Some(UpgradingInfo {
                current_protocol_version: 1,
                proposed_protocol_versions_with_weight: vec![(2, 1)],
                upgrade_three_quarters_life: 0.05,
            }),
            core_height_increase: Frequency {
                times_per_block_range: Default::default(),
                chance_per_block: None,
            },
        };
        let twenty_minutes_in_ms = 1000 * 60 * 20;
        let mut config = PlatformConfig {
            verify_sum_trees: true,
            quorum_size: 100,
            validator_set_quorum_rotation_block_count: 125,
            block_spacing_ms: twenty_minutes_in_ms,
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
        let ChainExecutionOutcome {
            abci_app,
            proposers,
            quorums,
            current_quorum_hash,
            current_proposer_versions,
            end_time_ms,
            ..
        } = run_chain_for_strategy(&mut platform, 1300, strategy.clone(), config.clone(), 15);
        {
            let platform = abci_app.platform;
            let drive_cache = platform.drive.cache.read().unwrap();
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
                    .read()
                    .unwrap()
                    .last_committed_block_info
                    .as_ref()
                    .unwrap()
                    .epoch
                    .index,
                0
            );
            assert_eq!(
                platform
                    .state
                    .read()
                    .unwrap()
                    .current_protocol_version_in_consensus,
                1
            );
            assert_eq!((counter.get(&1), counter.get(&2)), (Some(&6), Some(&419)));
            //most nodes were hit (63 were not)
        }

        // we did not yet hit the epoch change
        // let's go a little longer

        let platform = abci_app.platform;

        let hour_in_ms = 1000 * 60 * 60;
        let block_start = platform
            .state
            .read()
            .unwrap()
            .last_committed_block_info
            .as_ref()
            .unwrap()
            .height
            + 1;

        //speed things up
        config.block_spacing_ms = hour_in_ms;

        let ChainExecutionOutcome {
            abci_app,
            proposers,
            quorums,
            current_quorum_hash,
            end_time_ms,
            ..
        } = continue_chain_for_strategy(
            abci_app,
            ChainExecutionParameters {
                block_start,
                core_height_start: 1,
                block_count: 200,
                proposers,
                quorums,
                current_quorum_hash,
                current_proposer_versions: Some(current_proposer_versions.clone()),
                current_time_ms: end_time_ms,
            },
            strategy.clone(),
            config.clone(),
            StrategyRandomness::SeedEntropy(7),
        );
        {
            let drive_cache = platform.drive.cache.read().unwrap();
            let counter = drive_cache
                .protocol_versions_counter
                .as_ref()
                .expect("expected a version counter");
            assert_eq!(
                platform
                    .state
                    .read()
                    .unwrap()
                    .last_committed_block_info
                    .as_ref()
                    .unwrap()
                    .epoch
                    .index,
                1
            );
            assert_eq!(
                platform
                    .state
                    .read()
                    .unwrap()
                    .current_protocol_version_in_consensus,
                1
            );
            assert_eq!(
                platform.state.read().unwrap().next_epoch_protocol_version,
                2
            );
            assert_eq!(counter.get(&1), None); //no one has proposed 1 yet
            assert_eq!(counter.get(&2), Some(&152));
        }

        // we locked in
        // let's go a little longer to see activation

        let block_start = platform
            .state
            .read()
            .unwrap()
            .last_committed_block_info
            .as_ref()
            .unwrap()
            .height
            + 1;
        let ChainExecutionOutcome { .. } = continue_chain_for_strategy(
            abci_app,
            ChainExecutionParameters {
                block_start,
                core_height_start: 1,
                block_count: 400,
                proposers,
                quorums,
                current_quorum_hash,
                current_proposer_versions: Some(current_proposer_versions),
                current_time_ms: end_time_ms,
            },
            strategy,
            config,
            StrategyRandomness::SeedEntropy(18),
        );
        {
            let drive_cache = platform.drive.cache.read().unwrap();
            let counter = drive_cache
                .protocol_versions_counter
                .as_ref()
                .expect("expected a version counter");
            assert_eq!(
                platform
                    .state
                    .read()
                    .unwrap()
                    .last_committed_block_info
                    .as_ref()
                    .unwrap()
                    .epoch
                    .index,
                2
            );
            assert_eq!(
                platform
                    .state
                    .read()
                    .unwrap()
                    .current_protocol_version_in_consensus,
                2
            );
            assert_eq!(
                platform.state.read().unwrap().next_epoch_protocol_version,
                2
            );
            assert_eq!(counter.get(&1), None); //no one has proposed 1 yet
            assert_eq!(counter.get(&2), Some(&124));
        }
    }

    #[test]
    fn run_chain_version_upgrade_slow_upgrade() {
        let strategy = Strategy {
            contracts_with_updates: vec![],
            operations: vec![],
            identities_inserts: Frequency {
                times_per_block_range: Default::default(),
                chance_per_block: None,
            },
            total_hpmns: 120,
            extra_normal_mns: 0,
            quorum_count: 200,
            upgrading_info: Some(UpgradingInfo {
                current_protocol_version: 1,
                proposed_protocol_versions_with_weight: vec![(2, 1)],
                upgrade_three_quarters_life: 5.0, //it will take many epochs before we get enough nodes
            }),
            core_height_increase: Frequency {
                times_per_block_range: Default::default(),
                chance_per_block: None,
            },
        };
        let hour_in_ms = 1000 * 60 * 60;
        let config = PlatformConfig {
            verify_sum_trees: true,
            quorum_size: 40,
            validator_set_quorum_rotation_block_count: 15,
            block_spacing_ms: hour_in_ms,

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

        let ChainExecutionOutcome {
            abci_app,
            proposers,
            quorums,
            current_quorum_hash,
            current_proposer_versions,
            end_time_ms,
            ..
        } = run_chain_for_strategy(&mut platform, 2500, strategy.clone(), config.clone(), 16);
        {
            let platform = abci_app.platform;
            let drive_cache = platform.drive.cache.read().unwrap();
            let _counter = drive_cache
                .protocol_versions_counter
                .as_ref()
                .expect("expected a version counter");
            assert_eq!(
                platform
                    .state
                    .read()
                    .unwrap()
                    .last_committed_block_info
                    .as_ref()
                    .unwrap()
                    .epoch
                    .index,
                5
            );
            assert_eq!(
                platform
                    .state
                    .read()
                    .unwrap()
                    .current_protocol_version_in_consensus,
                1
            );
            assert_eq!(
                platform.state.read().unwrap().next_epoch_protocol_version,
                1
            );
            let counter = drive_cache
                .protocol_versions_counter
                .as_ref()
                .expect("expected a version counter");
            assert_eq!((counter.get(&1), counter.get(&2)), (Some(&47), Some(&65)));
        }

        // we did not yet hit the required threshold to upgrade
        // let's go a little longer

        let platform = abci_app.platform;
        let block_start = platform
            .state
            .read()
            .unwrap()
            .last_committed_block_info
            .as_ref()
            .unwrap()
            .height
            + 1;
        let ChainExecutionOutcome {
            abci_app,
            proposers,
            quorums,
            current_quorum_hash,
            end_time_ms,
            ..
        } = continue_chain_for_strategy(
            abci_app,
            ChainExecutionParameters {
                block_start,
                core_height_start: 1,
                block_count: 2100,
                proposers,
                quorums,
                current_quorum_hash,
                current_proposer_versions: Some(current_proposer_versions.clone()),
                current_time_ms: end_time_ms,
            },
            strategy.clone(),
            config.clone(),
            StrategyRandomness::SeedEntropy(7),
        );
        {
            let drive_cache = platform.drive.cache.read().unwrap();
            let counter = drive_cache
                .protocol_versions_counter
                .as_ref()
                .expect("expected a version counter");
            assert_eq!(
                platform
                    .state
                    .read()
                    .unwrap()
                    .last_committed_block_info
                    .as_ref()
                    .unwrap()
                    .epoch
                    .index,
                10
            );
            assert_eq!(
                platform
                    .state
                    .read()
                    .unwrap()
                    .current_protocol_version_in_consensus,
                1
            );
            assert_eq!(
                platform.state.read().unwrap().next_epoch_protocol_version,
                2
            );
            // the counter is for the current voting during that window
            assert_eq!((counter.get(&1), counter.get(&2)), (Some(&21), Some(&87)));
        }

        // we are now locked in, the current protocol version will change on next epoch

        let block_start = platform
            .state
            .read()
            .unwrap()
            .last_committed_block_info
            .as_ref()
            .unwrap()
            .height
            + 1;
        let ChainExecutionOutcome { .. } = continue_chain_for_strategy(
            abci_app,
            ChainExecutionParameters {
                block_start,
                core_height_start: 1,
                block_count: 400,
                proposers,
                quorums,
                current_quorum_hash,
                current_proposer_versions: Some(current_proposer_versions),
                current_time_ms: end_time_ms,
            },
            strategy,
            config,
            StrategyRandomness::SeedEntropy(8),
        );
        {
            let _drive_cache = platform.drive.cache.read().unwrap();
            assert_eq!(
                platform
                    .state
                    .read()
                    .unwrap()
                    .last_committed_block_info
                    .as_ref()
                    .unwrap()
                    .epoch
                    .index,
                11
            );
            assert_eq!(
                platform
                    .state
                    .read()
                    .unwrap()
                    .current_protocol_version_in_consensus,
                2
            );
            assert_eq!(
                platform.state.read().unwrap().next_epoch_protocol_version,
                2
            );
        }
    }

    #[test]
    fn run_chain_version_upgrade_slow_upgrade_quick_reversion_after_lock_in() {
        let strategy = Strategy {
            contracts_with_updates: vec![],
            operations: vec![],
            identities_inserts: Frequency {
                times_per_block_range: Default::default(),
                chance_per_block: None,
            },
            total_hpmns: 200,
            extra_normal_mns: 0,
            quorum_count: 100,
            upgrading_info: Some(UpgradingInfo {
                current_protocol_version: 1,
                proposed_protocol_versions_with_weight: vec![(2, 1)],
                upgrade_three_quarters_life: 5.0,
            }),
            core_height_increase: Frequency {
                times_per_block_range: Default::default(),
                chance_per_block: None,
            },
        };
        let hour_in_ms = 1000 * 60 * 60;
        let mut config = PlatformConfig {
            verify_sum_trees: true,
            quorum_size: 50,
            validator_set_quorum_rotation_block_count: 30,
            block_spacing_ms: hour_in_ms,

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
        let ChainExecutionOutcome {
            abci_app,
            proposers,
            quorums,
            current_quorum_hash,
            current_proposer_versions,
            end_time_ms,
            ..
        } = run_chain_for_strategy(&mut platform, 2000, strategy.clone(), config.clone(), 15);
        {
            let platform = abci_app.platform;
            let drive_cache = platform.drive.cache.read().unwrap();
            let _counter = drive_cache
                .protocol_versions_counter
                .as_ref()
                .expect("expected a version counter");
            assert_eq!(
                platform
                    .state
                    .read()
                    .unwrap()
                    .last_committed_block_info
                    .as_ref()
                    .unwrap()
                    .epoch
                    .index,
                4
            );
            assert_eq!(
                platform
                    .state
                    .read()
                    .unwrap()
                    .current_protocol_version_in_consensus,
                1
            );
        }

        // we still did not yet hit the required threshold to upgrade
        // let's go a just a little longer
        let platform = abci_app.platform;
        let block_start = platform
            .state
            .read()
            .unwrap()
            .last_committed_block_info
            .as_ref()
            .unwrap()
            .height
            + 1;
        let ChainExecutionOutcome {
            abci_app,
            proposers,
            quorums,
            current_quorum_hash,
            end_time_ms,
            ..
        } = continue_chain_for_strategy(
            abci_app,
            ChainExecutionParameters {
                block_start,
                core_height_start: 1,
                block_count: 2600,
                proposers,
                quorums,
                current_quorum_hash,
                current_proposer_versions: Some(current_proposer_versions),
                current_time_ms: end_time_ms,
            },
            strategy,
            config.clone(),
            StrategyRandomness::SeedEntropy(99),
        );
        {
            let drive_cache = platform.drive.cache.read().unwrap();
            let counter = drive_cache
                .protocol_versions_counter
                .as_ref()
                .expect("expected a version counter");
            assert_eq!(
                platform
                    .state
                    .read()
                    .unwrap()
                    .last_committed_block_info
                    .as_ref()
                    .unwrap()
                    .epoch
                    .index,
                10
            );
            assert_eq!(
                platform
                    .state
                    .read()
                    .unwrap()
                    .current_protocol_version_in_consensus,
                1
            );
            assert_eq!(
                platform.state.read().unwrap().next_epoch_protocol_version,
                2
            );
            assert_eq!((counter.get(&1), counter.get(&2)), (Some(&17), Some(&116)));
            //not all nodes have upgraded
        }

        // we are now locked in, the current protocol version will change on next epoch
        // however most nodes now revert

        let strategy = Strategy {
            contracts_with_updates: vec![],
            operations: vec![],
            identities_inserts: Frequency {
                times_per_block_range: Default::default(),
                chance_per_block: None,
            },
            total_hpmns: 200,
            extra_normal_mns: 0,
            quorum_count: 100,
            upgrading_info: Some(UpgradingInfo {
                current_protocol_version: 2,
                proposed_protocol_versions_with_weight: vec![(1, 9), (2, 1)],
                upgrade_three_quarters_life: 0.1,
            }),
            core_height_increase: Frequency {
                times_per_block_range: Default::default(),
                chance_per_block: None,
            },
        };

        let block_start = platform
            .state
            .read()
            .unwrap()
            .last_committed_block_info
            .as_ref()
            .unwrap()
            .height
            + 1;
        config.block_spacing_ms = hour_in_ms / 5; //speed things up
        let ChainExecutionOutcome {
            abci_app,
            proposers,
            quorums,
            current_quorum_hash,
            current_proposer_versions,
            end_time_ms,
            ..
        } = continue_chain_for_strategy(
            abci_app,
            ChainExecutionParameters {
                block_start,
                core_height_start: 1,
                block_count: 2000,
                proposers,
                quorums,
                current_quorum_hash,
                current_proposer_versions: None, //restart the proposer versions
                current_time_ms: end_time_ms,
            },
            strategy.clone(),
            config.clone(),
            StrategyRandomness::SeedEntropy(40),
        );
        {
            let drive_cache = platform.drive.cache.read().unwrap();
            let counter = drive_cache
                .protocol_versions_counter
                .as_ref()
                .expect("expected a version counter");
            assert_eq!((counter.get(&1), counter.get(&2)), (Some(&174), Some(&23)));
            //a lot nodes reverted to previous version, however this won't impact things
            assert_eq!(
                platform
                    .state
                    .read()
                    .unwrap()
                    .last_committed_block_info
                    .as_ref()
                    .unwrap()
                    .epoch
                    .index,
                11
            );
            assert_eq!(
                platform
                    .state
                    .read()
                    .unwrap()
                    .current_protocol_version_in_consensus,
                2
            );
            assert_eq!(
                platform.state.read().unwrap().next_epoch_protocol_version,
                1
            );
        }

        let block_start = platform
            .state
            .read()
            .unwrap()
            .last_committed_block_info
            .as_ref()
            .unwrap()
            .height
            + 1;
        config.block_spacing_ms = hour_in_ms * 4; //let's try to move to next epoch
        let ChainExecutionOutcome { .. } = continue_chain_for_strategy(
            abci_app,
            ChainExecutionParameters {
                block_start,
                core_height_start: 1,
                block_count: 100,
                proposers,
                quorums,
                current_quorum_hash,
                current_proposer_versions: Some(current_proposer_versions),
                current_time_ms: end_time_ms,
            },
            strategy,
            config,
            StrategyRandomness::SeedEntropy(40),
        );
        {
            let drive_cache = platform.drive.cache.read().unwrap();
            let counter = drive_cache
                .protocol_versions_counter
                .as_ref()
                .expect("expected a version counter");
            assert_eq!((counter.get(&1), counter.get(&2)), (Some(&30), Some(&2)));
            assert_eq!(
                platform
                    .state
                    .read()
                    .unwrap()
                    .last_committed_block_info
                    .as_ref()
                    .unwrap()
                    .epoch
                    .index,
                12
            );
            assert_eq!(
                platform
                    .state
                    .read()
                    .unwrap()
                    .current_protocol_version_in_consensus,
                1
            );
            assert_eq!(
                platform.state.read().unwrap().next_epoch_protocol_version,
                1
            );
        }
    }

    #[test]
    fn run_chain_version_upgrade_multiple_versions() {
        let strategy = Strategy {
            contracts_with_updates: vec![],
            operations: vec![],
            identities_inserts: Frequency {
                times_per_block_range: Default::default(),
                chance_per_block: None,
            },
            total_hpmns: 200,
            extra_normal_mns: 0,
            quorum_count: 100,
            upgrading_info: Some(UpgradingInfo {
                current_protocol_version: 1,
                proposed_protocol_versions_with_weight: vec![(1, 3), (2, 95), (3, 4)],
                upgrade_three_quarters_life: 0.75,
            }),
            core_height_increase: Frequency {
                times_per_block_range: Default::default(),
                chance_per_block: None,
            },
        };
        let hour_in_ms = 1000 * 60 * 60;
        let config = PlatformConfig {
            verify_sum_trees: true,
            quorum_size: 50,
            validator_set_quorum_rotation_block_count: 30,
            block_spacing_ms: hour_in_ms,

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
        let ChainExecutionOutcome {
            abci_app,
            proposers,
            quorums,
            current_quorum_hash,
            end_time_ms,
            ..
        } = run_chain_for_strategy(&mut platform, 1400, strategy, config.clone(), 15);
        {
            let platform = abci_app.platform;
            let drive_cache = platform.drive.cache.read().unwrap();
            let counter = drive_cache
                .protocol_versions_counter
                .as_ref()
                .expect("expected a version counter");

            assert_eq!(
                platform
                    .state
                    .read()
                    .unwrap()
                    .last_committed_block_info
                    .as_ref()
                    .unwrap()
                    .epoch
                    .index,
                3
            );
            assert_eq!(
                platform
                    .state
                    .read()
                    .unwrap()
                    .current_protocol_version_in_consensus,
                1
            );
            assert_eq!(
                platform.state.read().unwrap().next_epoch_protocol_version,
                2
            );
            assert_eq!(
                (counter.get(&1), counter.get(&2), counter.get(&3)),
                (Some(&6), Some(&67), Some(&2))
            ); //some nodes reverted to previous version
        }

        let strategy = Strategy {
            contracts_with_updates: vec![],
            operations: vec![],
            identities_inserts: Frequency {
                times_per_block_range: Default::default(),
                chance_per_block: None,
            },
            total_hpmns: 200,
            extra_normal_mns: 0,
            quorum_count: 24,
            upgrading_info: Some(UpgradingInfo {
                current_protocol_version: 1,
                proposed_protocol_versions_with_weight: vec![(2, 3), (3, 97)],
                upgrade_three_quarters_life: 0.5,
            }),
            core_height_increase: Frequency {
                times_per_block_range: Default::default(),
                chance_per_block: None,
            },
        };

        // we hit the required threshold to upgrade
        // let's go a little longer
        let platform = abci_app.platform;
        let block_start = platform
            .state
            .read()
            .unwrap()
            .last_committed_block_info
            .as_ref()
            .unwrap()
            .height
            + 1;
        let ChainExecutionOutcome { .. } = continue_chain_for_strategy(
            abci_app,
            ChainExecutionParameters {
                block_start,
                core_height_start: 1,
                block_count: 700,
                proposers,
                quorums,
                current_quorum_hash,
                current_proposer_versions: None,
                current_time_ms: end_time_ms,
            },
            strategy,
            config,
            StrategyRandomness::SeedEntropy(7),
        );
        {
            let drive_cache = platform.drive.cache.read().unwrap();
            let counter = drive_cache
                .protocol_versions_counter
                .as_ref()
                .expect("expected a version counter");
            assert_eq!(
                platform
                    .state
                    .read()
                    .unwrap()
                    .last_committed_block_info
                    .as_ref()
                    .unwrap()
                    .epoch
                    .index,
                4
            );
            assert_eq!(
                platform
                    .state
                    .read()
                    .unwrap()
                    .current_protocol_version_in_consensus,
                2
            );
            assert_eq!(
                platform.state.read().unwrap().next_epoch_protocol_version,
                3
            );
            assert_eq!(
                (counter.get(&1), counter.get(&2), counter.get(&3)),
                (None, Some(&5), Some(&159))
            );
        }
    }
}
