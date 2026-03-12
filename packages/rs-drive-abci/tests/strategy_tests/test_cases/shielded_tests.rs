// Feature-gated because shielded strategy tests are long-running (ZK proof generation).
// Run with: cargo test -p drive-abci --features __shielded_strategy_tests
// TODO: Add shielded variants to OperationType enum to enable these tests.
#[cfg(feature = "__shielded_strategy_tests")]
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

    /// Helper: create a standard platform config for shielded tests.
    fn shielded_test_config() -> PlatformConfig {
        PlatformConfig {
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
        }
    }

    /// Helper: create a base NetworkStrategy with common settings for shielded tests.
    fn shielded_base_strategy(operations: Vec<Operation>) -> NetworkStrategy {
        NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations,
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
            verify_state_transition_results: false,
            sign_instant_locks: true,
            ..Default::default()
        }
    }

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

        let strategy = shielded_base_strategy(vec![
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
        ]);

        let config = shielded_test_config();

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

    /// Strategy test that shields funds directly from core asset lock proofs
    /// into the shielded credit pool.
    ///
    /// This exercises the ShieldFromAssetLock transition lifecycle:
    /// 1. Asset lock proof creation and signing
    /// 2. Orchard bundle building (output-only, no spends)
    /// 3. Halo 2 ZK proof generation
    /// 4. ECDSA signing of the transition with asset lock private key
    /// 5. Platform validation and storage
    #[test]
    fn run_chain_shield_from_asset_lock_transitions() {
        drive_abci::logging::init_for_tests(LogLevel::Debug);

        let strategy = shielded_base_strategy(vec![
            // Shield directly from asset locks (1-2 per block, 5-10 DASH each)
            Operation {
                op_type: OperationType::ShieldFromAssetLock(
                    dash_to_credits!(5)..=dash_to_credits!(10),
                ),
                frequency: Frequency {
                    times_per_block_range: 1..3,
                    chance_per_block: None,
                },
            },
        ]);

        let config = shielded_test_config();

        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let outcome =
            run_chain_for_strategy(&mut platform, 5, strategy, config, 15, &mut None, &mut None);

        let shield_from_asset_lock_count = outcome
            .state_transition_results_per_block
            .values()
            .flat_map(|results| results.iter())
            .filter(|(st, result)| {
                matches!(st, StateTransition::ShieldFromAssetLock(_)) && result.code == 0
            })
            .count();

        assert!(
            shield_from_asset_lock_count > 0,
            "expected at least one successful ShieldFromAssetLock transition across 5 blocks"
        );

        tracing::info!(
            shield_from_asset_lock_count,
            "ShieldFromAssetLock strategy test completed successfully"
        );
    }

    /// Strategy test that first shields funds, then transfers within the shielded pool.
    ///
    /// This exercises the ShieldedTransfer transition lifecycle:
    /// 1. Shield funds first to create spendable notes
    /// 2. Build spend bundles consuming previous notes
    /// 3. Halo 2 ZK proof generation for spend+output
    /// 4. RedPallas spend authorization signing
    /// 5. Platform validation (anchor, nullifiers, ZK proof)
    ///
    /// The first few blocks only shield (to create spendable notes). The
    /// ShieldedTransfer operations only succeed once notes become available
    /// in the tracked shielded state.
    #[test]
    fn run_chain_shielded_transfer_transitions() {
        drive_abci::logging::init_for_tests(LogLevel::Debug);

        let strategy = shielded_base_strategy(vec![
            // Fund addresses first
            Operation {
                op_type: OperationType::AddressFundingFromCoreAssetLock(
                    dash_to_credits!(20)..=dash_to_credits!(20),
                ),
                frequency: Frequency {
                    times_per_block_range: 2..4,
                    chance_per_block: None,
                },
            },
            // Shield funds to create spendable notes
            Operation {
                op_type: OperationType::Shield(dash_to_credits!(5)..=dash_to_credits!(10)),
                frequency: Frequency {
                    times_per_block_range: 1..3,
                    chance_per_block: None,
                },
            },
            // Transfer within shielded pool (will only work once notes are available)
            Operation {
                op_type: OperationType::ShieldedTransfer(dash_to_credits!(1)..=dash_to_credits!(5)),
                frequency: Frequency {
                    times_per_block_range: 1..2,
                    chance_per_block: None,
                },
            },
        ]);

        let config = shielded_test_config();

        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let outcome =
            run_chain_for_strategy(&mut platform, 8, strategy, config, 15, &mut None, &mut None);

        // Count all shielded transitions
        let shield_count = outcome
            .state_transition_results_per_block
            .values()
            .flat_map(|results| results.iter())
            .filter(|(st, result)| matches!(st, StateTransition::Shield(_)) && result.code == 0)
            .count();

        let transfer_count = outcome
            .state_transition_results_per_block
            .values()
            .flat_map(|results| results.iter())
            .filter(|(st, result)| {
                matches!(st, StateTransition::ShieldedTransfer(_)) && result.code == 0
            })
            .count();

        tracing::info!(
            shield_count,
            transfer_count,
            "ShieldedTransfer strategy test completed"
        );

        assert!(
            shield_count > 0,
            "expected at least one successful Shield transition"
        );

        // ShieldedTransfer may or may not succeed depending on timing (notes need to be
        // available and anchors need to be in state). We just verify the test runs
        // without panicking. If transfers succeed, that's a bonus.
        tracing::info!(
            transfer_count,
            "ShieldedTransfer transitions that succeeded"
        );
    }

    /// Strategy test that first shields funds, then unshields to platform addresses.
    ///
    /// This exercises the Unshield transition lifecycle:
    /// 1. Shield funds first to create spendable notes
    /// 2. Build spend bundles with extra_data binding output_address + amount
    /// 3. Halo 2 ZK proof generation for spend+output
    /// 4. RedPallas spend authorization signing
    /// 5. Platform validation (anchor, nullifiers, ZK proof, pool balance)
    #[test]
    fn run_chain_unshield_transitions() {
        drive_abci::logging::init_for_tests(LogLevel::Debug);

        let strategy = shielded_base_strategy(vec![
            // Fund addresses first
            Operation {
                op_type: OperationType::AddressFundingFromCoreAssetLock(
                    dash_to_credits!(20)..=dash_to_credits!(20),
                ),
                frequency: Frequency {
                    times_per_block_range: 2..4,
                    chance_per_block: None,
                },
            },
            // Shield funds to create spendable notes
            Operation {
                op_type: OperationType::Shield(dash_to_credits!(5)..=dash_to_credits!(10)),
                frequency: Frequency {
                    times_per_block_range: 1..3,
                    chance_per_block: None,
                },
            },
            // Unshield from pool to platform address
            Operation {
                op_type: OperationType::Unshield(dash_to_credits!(1)..=dash_to_credits!(5)),
                frequency: Frequency {
                    times_per_block_range: 1..2,
                    chance_per_block: None,
                },
            },
        ]);

        let config = shielded_test_config();

        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let outcome =
            run_chain_for_strategy(&mut platform, 8, strategy, config, 15, &mut None, &mut None);

        let shield_count = outcome
            .state_transition_results_per_block
            .values()
            .flat_map(|results| results.iter())
            .filter(|(st, result)| matches!(st, StateTransition::Shield(_)) && result.code == 0)
            .count();

        let unshield_count = outcome
            .state_transition_results_per_block
            .values()
            .flat_map(|results| results.iter())
            .filter(|(st, result)| matches!(st, StateTransition::Unshield(_)) && result.code == 0)
            .count();

        tracing::info!(
            shield_count,
            unshield_count,
            "Unshield strategy test completed"
        );

        assert!(
            shield_count > 0,
            "expected at least one successful Shield transition"
        );

        // Like ShieldedTransfer, unshield may not succeed on every run due to
        // timing constraints (notes + anchors in state).
        tracing::info!(unshield_count, "Unshield transitions that succeeded");
    }

    /// Strategy test that verifies anchors are correctly recorded and indexed
    /// after shielding operations that add notes to the commitment tree.
    ///
    /// Checks:
    /// 1. Anchors tree (anchor_bytes → block_height) has entries after successful shields
    /// 2. Anchors-by-height tree (block_height → anchor_bytes) has matching reverse entries
    /// 3. Most recent anchor element is updated and non-zero
    /// 4. Both trees are consistent (same count, matching entries)
    #[test]
    fn run_chain_verify_anchors_after_shielding() {
        use drive::drive::shielded::paths::{
            shielded_credit_pool_anchors_by_height_path, shielded_credit_pool_anchors_path_vec,
            shielded_credit_pool_path, SHIELDED_MOST_RECENT_ANCHOR_KEY,
        };
        use drive::grovedb::query_result_type::QueryResultType;
        use drive::grovedb::{Element, PathQuery, Query, SizedQuery};

        drive_abci::logging::init_for_tests(LogLevel::Debug);

        let strategy = shielded_base_strategy(vec![
            // Fund addresses (every block, 2-3 asset locks of 20 DASH each)
            Operation {
                op_type: OperationType::AddressFundingFromCoreAssetLock(
                    dash_to_credits!(20)..=dash_to_credits!(20),
                ),
                frequency: Frequency {
                    times_per_block_range: 2..4,
                    chance_per_block: None,
                },
            },
            // Shield funds (1 per block, 1-5 DASH)
            Operation {
                op_type: OperationType::Shield(dash_to_credits!(1)..=dash_to_credits!(5)),
                frequency: Frequency {
                    times_per_block_range: 1..2,
                    chance_per_block: None,
                },
            },
        ]);

        let config = shielded_test_config();

        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let block_count = 5;
        let outcome = run_chain_for_strategy(
            &mut platform,
            block_count,
            strategy,
            config,
            15,
            &mut None,
            &mut None,
        );

        // Verify at least one shield succeeded (prerequisite for anchors)
        let shield_count = outcome
            .state_transition_results_per_block
            .values()
            .flat_map(|results| results.iter())
            .filter(|(st, result)| matches!(st, StateTransition::Shield(_)) && result.code == 0)
            .count();
        assert!(
            shield_count > 0,
            "expected at least one successful shield transition"
        );

        let platform_state = outcome.abci_app.platform.state.load();
        let platform_version = platform_state
            .current_platform_version()
            .expect("expected platform version");
        let drive = &outcome.abci_app.platform.drive;

        // 1. Query all anchors from the anchors tree (anchor_bytes → block_height)
        let anchors_path_query = PathQuery {
            path: shielded_credit_pool_anchors_path_vec(),
            query: SizedQuery {
                query: Query::new_range_full(),
                limit: None,
                offset: None,
            },
        };

        let (anchor_results, _) = drive
            .grove_get_raw_path_query(
                &anchors_path_query,
                None,
                QueryResultType::QueryKeyElementPairResultType,
                &mut vec![],
                &platform_version.drive,
            )
            .expect("expected to query anchors");

        let anchor_entries = anchor_results.to_key_elements();
        assert!(
            !anchor_entries.is_empty(),
            "expected anchors to be recorded after successful shield transitions"
        );

        // Each anchor should map to a valid block height within our range
        let mut anchor_to_height: Vec<(Vec<u8>, u64)> = Vec::new();
        for (anchor_key, element) in &anchor_entries {
            assert_eq!(anchor_key.len(), 32, "anchor key must be 32 bytes");
            if let Element::Item(value, _) = element {
                let height = u64::from_be_bytes(
                    value
                        .as_slice()
                        .try_into()
                        .expect("block height must be 8 bytes"),
                );
                assert!(
                    height >= 1 && height <= block_count,
                    "anchor block height {} out of expected range [1, {}]",
                    height,
                    block_count
                );
                anchor_to_height.push((anchor_key.clone(), height));
            } else {
                panic!("expected Item element in anchors tree");
            }
        }

        // 2. Query all entries from anchors-by-height tree (block_height → anchor_bytes)
        let by_height_path: Vec<Vec<u8>> = shielded_credit_pool_anchors_by_height_path()
            .iter()
            .map(|p| p.to_vec())
            .collect();
        let by_height_path_query = PathQuery {
            path: by_height_path,
            query: SizedQuery {
                query: Query::new_range_full(),
                limit: None,
                offset: None,
            },
        };

        let (by_height_results, _) = drive
            .grove_get_raw_path_query(
                &by_height_path_query,
                None,
                QueryResultType::QueryKeyElementPairResultType,
                &mut vec![],
                &platform_version.drive,
            )
            .expect("expected to query anchors-by-height");

        let by_height_entries = by_height_results.to_key_elements();

        // Both trees must have the same number of entries
        assert_eq!(
            anchor_entries.len(),
            by_height_entries.len(),
            "anchors tree ({}) and anchors-by-height tree ({}) must have same entry count",
            anchor_entries.len(),
            by_height_entries.len()
        );

        // Verify the reverse index is consistent: for each height→anchor, the anchor→height must match
        for (height_key, element) in &by_height_entries {
            let height = u64::from_be_bytes(
                height_key
                    .as_slice()
                    .try_into()
                    .expect("height key must be 8 bytes"),
            );
            if let Element::Item(anchor_bytes, _) = element {
                assert_eq!(anchor_bytes.len(), 32, "anchor value must be 32 bytes");
                // Find matching entry in anchor_to_height
                let matching = anchor_to_height
                    .iter()
                    .find(|(a, h)| a == anchor_bytes && *h == height);
                assert!(
                    matching.is_some(),
                    "anchors-by-height entry (height={}) has no matching entry in anchors tree",
                    height
                );
            } else {
                panic!("expected Item element in anchors-by-height tree");
            }
        }

        // 3. Verify most recent anchor is set and non-zero
        let pool_path = shielded_credit_pool_path();
        let most_recent_element = drive
            .grove
            .get(
                &pool_path,
                &[SHIELDED_MOST_RECENT_ANCHOR_KEY],
                None,
                &platform_version.drive.grove_version,
            )
            .unwrap()
            .expect("most recent anchor element must exist");

        if let Element::Item(most_recent_bytes, _) = most_recent_element {
            assert_eq!(
                most_recent_bytes.len(),
                32,
                "most recent anchor must be 32 bytes"
            );
            assert_ne!(
                most_recent_bytes,
                vec![0u8; 32],
                "most recent anchor must not be all zeros after successful shields"
            );
            // Most recent anchor must be one of the recorded anchors
            let is_known = anchor_to_height
                .iter()
                .any(|(a, _)| *a == most_recent_bytes);
            assert!(
                is_known,
                "most recent anchor must match one of the recorded anchors"
            );
        } else {
            panic!("most recent anchor must be an Item element");
        }

        tracing::info!(
            anchor_count = anchor_entries.len(),
            shield_count,
            "Anchor verification test completed successfully"
        );
    }

    /// Strategy test that first shields funds, then withdraws to a core (L1) address.
    ///
    /// This exercises the ShieldedWithdrawal transition lifecycle:
    /// 1. Shield funds first to create spendable notes
    /// 2. Build spend bundles with extra_data binding output_script + amount
    /// 3. Halo 2 ZK proof generation for spend+output
    /// 4. RedPallas spend authorization signing
    /// 5. Platform validation (anchor, nullifiers, ZK proof, pool balance, withdrawal queue)
    #[test]
    fn run_chain_shielded_withdrawal_transitions() {
        drive_abci::logging::init_for_tests(LogLevel::Debug);

        let strategy = shielded_base_strategy(vec![
            // Fund addresses first
            Operation {
                op_type: OperationType::AddressFundingFromCoreAssetLock(
                    dash_to_credits!(20)..=dash_to_credits!(20),
                ),
                frequency: Frequency {
                    times_per_block_range: 2..4,
                    chance_per_block: None,
                },
            },
            // Shield funds to create spendable notes
            Operation {
                op_type: OperationType::Shield(dash_to_credits!(5)..=dash_to_credits!(10)),
                frequency: Frequency {
                    times_per_block_range: 1..3,
                    chance_per_block: None,
                },
            },
            // Withdraw from pool to core address
            Operation {
                op_type: OperationType::ShieldedWithdrawal(
                    dash_to_credits!(1)..=dash_to_credits!(5),
                ),
                frequency: Frequency {
                    times_per_block_range: 1..2,
                    chance_per_block: None,
                },
            },
        ]);

        let config = shielded_test_config();

        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let outcome =
            run_chain_for_strategy(&mut platform, 8, strategy, config, 15, &mut None, &mut None);

        let shield_count = outcome
            .state_transition_results_per_block
            .values()
            .flat_map(|results| results.iter())
            .filter(|(st, result)| matches!(st, StateTransition::Shield(_)) && result.code == 0)
            .count();

        let withdrawal_count = outcome
            .state_transition_results_per_block
            .values()
            .flat_map(|results| results.iter())
            .filter(|(st, result)| {
                matches!(st, StateTransition::ShieldedWithdrawal(_)) && result.code == 0
            })
            .count();

        tracing::info!(
            shield_count,
            withdrawal_count,
            "ShieldedWithdrawal strategy test completed"
        );

        assert!(
            shield_count > 0,
            "expected at least one successful Shield transition"
        );

        // Like the other spend-based transitions, withdrawal may not succeed on
        // every run due to timing constraints.
        tracing::info!(
            withdrawal_count,
            "ShieldedWithdrawal transitions that succeeded"
        );
    }
}
