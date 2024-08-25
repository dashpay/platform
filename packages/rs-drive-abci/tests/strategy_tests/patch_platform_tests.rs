#[cfg(test)]
mod tests {
    use dpp::block::extended_block_info::v0::ExtendedBlockInfoV0Getters;
    use drive::config::DriveConfig;
    use std::collections::{BTreeMap, HashMap};

    use crate::execution::{continue_chain_for_strategy, run_chain_for_strategy};
    use crate::strategy::{
        ChainExecutionOutcome, ChainExecutionParameters, NetworkStrategy, StrategyRandomness,
        UpgradingInfo,
    };
    use drive_abci::config::{
        ChainLockConfig, ExecutionConfig, InstantLockConfig, PlatformConfig, PlatformTestConfig,
        ValidatorSetConfig,
    };
    use drive_abci::platform_types::platform_state::v0::PlatformStateV0Methods;
    use drive_abci::test::helpers::setup::TestPlatformBuilder;
    use platform_version::version;
    use platform_version::version::mocks::v2_test::TEST_PROTOCOL_VERSION_2;
    use platform_version::version::patches::PatchFn;
    use platform_version::version::PlatformVersion;

    #[test]
    fn test_patch_version() {
        // Define the desired stack size
        let stack_size = 4 * 1024 * 1024; // Let's set the stack size to be higher than the default 2MB

        let builder = std::thread::Builder::new()
            .stack_size(stack_size)
            .name("custom_stack_size_thread".into());

        pub fn patch_1_5_test(mut platform_version: PlatformVersion) -> PlatformVersion {
            platform_version
                .drive_abci
                .query
                .document_query
                .default_current_version = 5;

            platform_version
        }

        pub fn patch_1_10_test(mut platform_version: PlatformVersion) -> PlatformVersion {
            platform_version.drive_abci.query.document_query.max_version = 10;

            platform_version
        }

        pub fn patch_2_30_test(mut platform_version: PlatformVersion) -> PlatformVersion {
            platform_version.drive_abci.query.document_query.min_version = 30;

            platform_version
        }

        let mut patches = version::patches::PATCHES.write().unwrap();

        *patches = HashMap::from_iter(vec![
            {
                (
                    1,
                    BTreeMap::from_iter(vec![
                        (5, patch_1_5_test as PatchFn),
                        (10, patch_1_10_test as PatchFn),
                    ]),
                )
            },
            {
                (
                    TEST_PROTOCOL_VERSION_2,
                    BTreeMap::from_iter(vec![(30, patch_2_30_test as PatchFn)]),
                )
            },
        ]);

        drop(patches);

        let handler = builder
            .spawn(|| {
                let strategy = NetworkStrategy {
                    total_hpmns: 4,
                    upgrading_info: Some(UpgradingInfo {
                        current_protocol_version: 1,
                        proposed_protocol_versions_with_weight: vec![(TEST_PROTOCOL_VERSION_2, 1)],
                        upgrade_three_quarters_life: 0.0,
                    }),
                    ..Default::default()
                };

                let config = PlatformConfig {
                    validator_set: ValidatorSetConfig {
                        quorum_size: 4,
                        ..Default::default()
                    },
                    chain_lock: ChainLockConfig::default_100_67(),
                    instant_lock: InstantLockConfig::default_100_67(),
                    execution: ExecutionConfig {
                        epoch_time_length_s: 60 * 60,
                        ..Default::default()
                    },
                    drive: DriveConfig::default(),
                    block_spacing_ms: 1000 * 60 * 5,
                    testing_configs: PlatformTestConfig::default_minimal_verifications(),

                    ..Default::default()
                };

                let mut platform = TestPlatformBuilder::new()
                    .with_config(config.clone())
                    .build_with_mock_rpc();

                // Run chain before the first patch

                let ChainExecutionOutcome {
                    abci_app,
                    proposers,
                    validator_quorums: quorums,
                    current_validator_quorum_hash: current_quorum_hash,
                    current_proposer_versions,
                    end_time_ms,
                    identity_nonce_counter,
                    identity_contract_nonce_counter,
                    instant_lock_quorums,
                    ..
                } = run_chain_for_strategy(
                    &mut platform,
                    4,
                    strategy.clone(),
                    config.clone(),
                    13,
                    &mut None,
                );

                let platform = abci_app.platform;

                // Make sure patch 1 5 is not applied yet
                let state = platform.state.load();
                let platform_version = state
                    .current_platform_version()
                    .expect("getting patched version shouldn't fail");

                assert_eq!(state.last_committed_block_epoch().index, 0);
                assert_eq!(state.current_protocol_version_in_consensus(), 1);
                assert_eq!(
                    platform_version
                        .drive_abci
                        .query
                        .document_query
                        .default_current_version,
                    0
                );

                // Run for 2 more blocks to make sure patch 1 5 is applied,
                // and it persists for the further blocks

                let block_start = state
                    .last_committed_block_info()
                    .as_ref()
                    .unwrap()
                    .basic_info()
                    .height
                    + 1;

                let ChainExecutionOutcome {
                    abci_app,
                    proposers,
                    validator_quorums: quorums,
                    current_validator_quorum_hash: current_quorum_hash,
                    current_proposer_versions,
                    end_time_ms,
                    identity_nonce_counter,
                    identity_contract_nonce_counter,
                    instant_lock_quorums,
                    ..
                } = continue_chain_for_strategy(
                    abci_app,
                    ChainExecutionParameters {
                        block_start,
                        core_height_start: 1,
                        block_count: 2,
                        proposers,
                        validator_quorums: quorums,
                        current_validator_quorum_hash: current_quorum_hash,
                        instant_lock_quorums,
                        current_proposer_versions: Some(current_proposer_versions.clone()),
                        current_identity_nonce_counter: identity_nonce_counter,
                        current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                        current_votes: BTreeMap::default(),
                        start_time_ms: 1681094380000,
                        current_time_ms: end_time_ms,
                        current_identities: Vec::new(),
                    },
                    strategy.clone(),
                    config.clone(),
                    StrategyRandomness::SeedEntropy(7),
                );

                // Make sure patch 1 5 is applied
                let state = platform.state.load();
                let platform_version = state
                    .current_platform_version()
                    .expect("getting patched version shouldn't fail");

                assert_eq!(state.last_committed_block_epoch().index, 0);
                assert_eq!(state.current_protocol_version_in_consensus(), 1);
                assert_eq!(
                    platform_version
                        .drive_abci
                        .query
                        .document_query
                        .default_current_version,
                    5
                );

                // Run chain for 9 more blocks to apply patch 1 15

                let block_start = state
                    .last_committed_block_info()
                    .as_ref()
                    .unwrap()
                    .basic_info()
                    .height
                    + 1;

                let ChainExecutionOutcome {
                    abci_app,
                    proposers,
                    validator_quorums: quorums,
                    current_validator_quorum_hash: current_quorum_hash,
                    current_proposer_versions,
                    end_time_ms,
                    identity_nonce_counter,
                    identity_contract_nonce_counter,
                    instant_lock_quorums,
                    ..
                } = continue_chain_for_strategy(
                    abci_app,
                    ChainExecutionParameters {
                        block_start,
                        core_height_start: 1,
                        block_count: 4,
                        proposers,
                        validator_quorums: quorums,
                        current_validator_quorum_hash: current_quorum_hash,
                        current_proposer_versions: Some(current_proposer_versions.clone()),
                        current_identity_nonce_counter: identity_nonce_counter,
                        current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                        current_votes: BTreeMap::default(),
                        start_time_ms: 1681094380000,
                        current_time_ms: end_time_ms,
                        instant_lock_quorums,
                        current_identities: Vec::new(),
                    },
                    strategy.clone(),
                    config.clone(),
                    StrategyRandomness::SeedEntropy(7),
                );

                // Make sure patch 1 5 and 10 is applied
                let state = platform.state.load();
                let platform_version = state
                    .current_platform_version()
                    .expect("getting patched version shouldn't fail");

                assert_eq!(state.last_committed_block_epoch().index, 0);
                assert_eq!(state.current_protocol_version_in_consensus(), 1);
                assert_eq!(
                    platform_version.drive_abci.query.document_query.max_version,
                    10
                );
                assert_eq!(
                    platform_version
                        .drive_abci
                        .query
                        .document_query
                        .default_current_version,
                    5
                );

                // Run chain for 10 more blocks to upgrade to version 2

                let block_start = state
                    .last_committed_block_info()
                    .as_ref()
                    .unwrap()
                    .basic_info()
                    .height
                    + 1;

                let ChainExecutionOutcome {
                    abci_app,
                    proposers,
                    validator_quorums: quorums,
                    current_validator_quorum_hash: current_quorum_hash,
                    current_proposer_versions,
                    end_time_ms,
                    identity_nonce_counter,
                    identity_contract_nonce_counter,
                    instant_lock_quorums,
                    ..
                } = continue_chain_for_strategy(
                    abci_app,
                    ChainExecutionParameters {
                        block_start,
                        core_height_start: 1,
                        block_count: 15,
                        proposers,
                        validator_quorums: quorums,
                        current_validator_quorum_hash: current_quorum_hash,
                        current_proposer_versions: Some(current_proposer_versions),
                        current_identity_nonce_counter: identity_nonce_counter,
                        current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                        current_votes: BTreeMap::default(),
                        start_time_ms: 1681094380000,
                        current_time_ms: end_time_ms,
                        instant_lock_quorums,
                        current_identities: Vec::new(),
                    },
                    strategy.clone(),
                    config.clone(),
                    StrategyRandomness::SeedEntropy(18),
                );

                // Make sure we switched version and drop all patches
                let state = platform.state.load();
                let platform_version = state
                    .current_platform_version()
                    .expect("getting patched version shouldn't fail");

                assert_eq!(state.last_committed_block_epoch().index, 2);
                assert_eq!(
                    state.current_protocol_version_in_consensus(),
                    TEST_PROTOCOL_VERSION_2
                );
                assert_eq!(
                    platform_version
                        .drive_abci
                        .query
                        .document_query
                        .default_current_version,
                    0
                );
                assert_eq!(
                    platform_version.drive_abci.query.document_query.min_version,
                    0
                );
                assert_eq!(
                    platform_version.drive_abci.query.document_query.max_version,
                    0
                );

                // Run chain for 10 more blocks to apply 2 45 patch

                let block_start = state
                    .last_committed_block_info()
                    .as_ref()
                    .unwrap()
                    .basic_info()
                    .height
                    + 1;

                let ChainExecutionOutcome { .. } = continue_chain_for_strategy(
                    abci_app,
                    ChainExecutionParameters {
                        block_start,
                        core_height_start: 1,
                        block_count: 10,
                        proposers,
                        validator_quorums: quorums,
                        current_validator_quorum_hash: current_quorum_hash,
                        current_proposer_versions: Some(current_proposer_versions),
                        current_identity_nonce_counter: identity_nonce_counter,
                        current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                        current_votes: BTreeMap::default(),
                        start_time_ms: 1681094380000,
                        current_time_ms: end_time_ms,
                        instant_lock_quorums,
                        current_identities: Vec::new(),
                    },
                    strategy,
                    config,
                    StrategyRandomness::SeedEntropy(18),
                );

                // Make sure we applied 2 30 and patches for version 1 is ignored
                let state = platform.state.load();
                let platform_version = state
                    .current_platform_version()
                    .expect("getting patched version shouldn't fail");

                assert_eq!(
                    state.current_protocol_version_in_consensus(),
                    TEST_PROTOCOL_VERSION_2
                );
                assert_eq!(
                    platform_version
                        .drive_abci
                        .query
                        .document_query
                        .default_current_version,
                    0
                );
                assert_eq!(
                    platform_version.drive_abci.query.document_query.min_version,
                    30
                );
                assert_eq!(
                    platform_version.drive_abci.query.document_query.max_version,
                    0
                );
            })
            .expect("Failed to create thread with custom stack size");

        fn cleanup_version_patches() {
            let mut patches = version::patches::PATCHES.write().unwrap();
            patches.clear();
        }

        // Wait for the thread to finish and assert that it didn't panic.
        handler
            .join()
            .map(|result| {
                cleanup_version_patches();

                result
            })
            .map_err(|e| {
                cleanup_version_patches();

                e
            })
            .expect("Thread has panicked");
    }
}
