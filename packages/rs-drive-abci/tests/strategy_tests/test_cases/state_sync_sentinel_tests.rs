//! State sync QA: the restore sentinel and the never-wedge guarantee.
//!
//! A state sync restore destroys the node's database before it rebuilds it, and the
//! rebuild is not atomic with the platform state that has to describe it. Two things can
//! therefore leave a node holding a database its platform state knows nothing about:
//!
//! * the process dies between `commit_session` and `reconstruct_platform_state`;
//! * the restored snapshot turns out to be unusable (a pre-v15 snapshot with no reduced
//!   platform state, or one that fails verification) — which any peer can cause.
//!
//! Both used to wedge the node permanently, because the `info` handler panics on an
//! app-hash mismatch and restarting reloads exactly the state that causes the panic. The
//! fix is a sentinel file written next to the database before the wipe, cleared only when
//! the node is self-consistent again, plus a rejection path that wipes back to a clean
//! slate instead of returning an error.
//!
//! # These tests do not need the patched grovedb
//!
//! Everything here holds at BOTH grovedb pins. Nothing asserts that a restore SUCCEEDS —
//! the tests that do live in `state_sync_equivalence_tests` and need dashpay/grovedb#840,
//! because Dash Platform state always contains sum trees. What is asserted here is that a
//! restore which does not succeed leaves a recoverable node, and at the unpinned revision
//! the sum-tree defect simply supplies the failure for free: the transfer commits, the
//! post-restore verification fails, and the same rejection path runs.

#[cfg(test)]
mod tests {
    use crate::execution::run_chain_for_strategy;
    use crate::strategy::{ChainExecutionOutcome, NetworkStrategy};
    use crate::test_cases::state_sync_tests::tests::{
        install_reconstruction_core_mocks, sync_snapshot, SnapshotSyncOutcome,
    };
    use dpp::version::PlatformVersion;
    use drive_abci::abci::app::FullAbciApplication;
    use drive_abci::config::{
        ChainLockConfig, ExecutionConfig, InstantLockConfig, PlatformConfig, PlatformTestConfig,
        ValidatorSetConfig,
    };
    use drive_abci::platform_types::platform::Platform;
    use drive_abci::platform_types::platform_state::PlatformStateV0Methods;
    use drive_abci::platform_types::snapshot::{
        restore_sentinel_exists, write_restore_sentinel, RESTORE_IN_PROGRESS_FILE_NAME,
    };
    use drive_abci::rpc::core::MockCoreRPCLike;
    use drive_abci::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
    use strategy_tests::frequency::Frequency;
    use strategy_tests::{IdentityInsertInfo, StartAddresses, StartIdentities, Strategy};
    use tenderdash_abci::proto::abci as proto;
    use tenderdash_abci::proto::abci::response_offer_snapshot;
    use tenderdash_abci::Application;

    const SOURCE_CHAIN_BLOCKS: u64 = 6;
    const SOURCE_CHAIN_SEED: u64 = 15;

    fn sentinel_platform_config() -> PlatformConfig {
        let mut testing_configs = PlatformTestConfig::default_minimal_verifications();
        testing_configs.disable_checkpoints = false;
        testing_configs.store_platform_state = true;

        let mut config = PlatformConfig {
            validator_set: ValidatorSetConfig::default_100_67(),
            chain_lock: ChainLockConfig::default_100_67(),
            instant_lock: InstantLockConfig::default_100_67(),
            execution: ExecutionConfig {
                verify_sum_trees: true,
                ..ExecutionConfig::default()
            },
            block_spacing_ms: 3000,
            testing_configs,
            ..Default::default()
        };
        config.abci.state_sync.snapshots_enabled = true;
        config.abci.state_sync.snapshots_frequency_seconds = 1;
        config.abci.state_sync.max_num_snapshots = 3;
        config
    }

    fn sentinel_strategy() -> NetworkStrategy {
        NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
                start_addresses: StartAddresses::default(),
                identity_inserts: IdentityInsertInfo {
                    frequency: Frequency {
                        times_per_block_range: 1..3,
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
            verify_state_transition_results: false,
            ..Default::default()
        }
    }

    fn root_hash(platform: &Platform<MockCoreRPCLike>) -> [u8; 32] {
        platform
            .drive
            .grove
            .root_hash(None, &PlatformVersion::latest().drive.grove_version)
            .unwrap()
            .expect("root hash")
    }

    fn info_request() -> proto::RequestInfo {
        proto::RequestInfo {
            version: tenderdash_abci::proto::meta::TENDERDASH_VERSION.to_string(),
            block_version: 0,
            p2p_version: 0,
            abci_version: tenderdash_abci::proto::meta::ABCI_VERSION.to_string(),
        }
    }

    /// Models process death: drops the `Platform` (releasing grovedb's lock and every
    /// in-memory session, cache and platform state) and re-opens the SAME directory, which
    /// is what a restarted drive-abci does. Only what was durably written survives.
    fn restart(
        target: TempPlatform<MockCoreRPCLike>,
        config: &PlatformConfig,
    ) -> TempPlatform<MockCoreRPCLike> {
        let TempPlatform {
            platform, tempdir, ..
        } = target;
        drop(platform);
        TempPlatform::open_with_tempdir(tempdir, config.clone())
    }

    /// Calling `info` must not panic. The handler panics on an app-hash mismatch between
    /// the platform state and grovedb, which is the exact shape of the wedge, so "did it
    /// panic" is the property under test rather than the returned value.
    fn info_does_not_panic(platform: &TempPlatform<MockCoreRPCLike>) -> bool {
        let app = FullAbciApplication::new(platform);
        let previous_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let result =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| app.info(info_request())));
        std::panic::set_hook(previous_hook);
        result.is_ok()
    }

    /// `offer_snapshot` must record the sentinel BEFORE it wipes, so there is no window in
    /// which the database has been destroyed and nothing says so.
    #[tokio::test]
    async fn offer_snapshot_records_the_restore_sentinel_before_wiping() {
        let config = sentinel_platform_config();
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();
        let outcome = run_chain_for_strategy(
            &mut platform.platform,
            SOURCE_CHAIN_BLOCKS,
            sentinel_strategy(),
            config.clone(),
            SOURCE_CHAIN_SEED,
            &mut None,
            &mut None,
        )
        .await;
        let app = outcome.abci_app;
        let db_path = app.platform.config.db_path.clone();

        assert!(
            !restore_sentinel_exists(&db_path),
            "a node that never state-synced must not carry the sentinel"
        );

        let response = app
            .offer_snapshot(proto::RequestOfferSnapshot {
                snapshot: Some(proto::Snapshot {
                    height: 1000,
                    version: 1,
                    hash: vec![7u8; 32],
                    metadata: vec![],
                }),
                app_hash: vec![7u8; 32],
            })
            .expect("the offer must be accepted");
        assert_eq!(
            response.result,
            i32::from(response_offer_snapshot::Result::Accept)
        );

        assert!(
            restore_sentinel_exists(&db_path),
            "accepting an offer wipes the database, so it must first record that a restore \
             is in progress"
        );
        // The sentinel is a plain file NEXT TO the database, not aux storage: `wipe()`
        // clears the aux column family too, so a sentinel stored there would be destroyed
        // by the very wipe it exists to survive.
        assert!(
            db_path.join(RESTORE_IN_PROGRESS_FILE_NAME).is_file(),
            "the sentinel must live outside everything grovedb wipes"
        );
    }

    /// A rejected offer must not record a sentinel — nothing was wiped, so nothing needs
    /// recovering. Without this, any peer could make a healthy node wipe itself on the next
    /// restart just by offering a snapshot in a format it cannot speak.
    #[tokio::test]
    async fn a_rejected_offer_records_no_sentinel() {
        let config = sentinel_platform_config();
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();
        let outcome = run_chain_for_strategy(
            &mut platform.platform,
            SOURCE_CHAIN_BLOCKS,
            sentinel_strategy(),
            config.clone(),
            SOURCE_CHAIN_SEED,
            &mut None,
            &mut None,
        )
        .await;
        let app = outcome.abci_app;
        let db_path = app.platform.config.db_path.clone();

        let response = app
            .offer_snapshot(proto::RequestOfferSnapshot {
                snapshot: Some(proto::Snapshot {
                    height: 1000,
                    version: u32::MAX,
                    hash: vec![7u8; 32],
                    metadata: vec![],
                }),
                app_hash: vec![7u8; 32],
            })
            .expect("an unsupported version must be answered, not error");
        assert_eq!(
            response.result,
            i32::from(response_offer_snapshot::Result::RejectFormat)
        );
        assert!(
            !restore_sentinel_exists(&db_path),
            "a rejected offer wipes nothing and must leave no sentinel behind"
        );
    }

    /// The startup recovery itself: a node whose sentinel is still present comes up EMPTY
    /// rather than crash-looping.
    ///
    /// The database here is deliberately a healthy, fully populated chain — the strongest
    /// form of "grovedb holds state the platform state will not describe". Recovery must
    /// throw it away, because there is no way to tell how far an interrupted restore got.
    #[tokio::test]
    async fn a_node_restarting_mid_restore_wipes_and_comes_up_empty() {
        let config = sentinel_platform_config();
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();
        let outcome = run_chain_for_strategy(
            &mut platform.platform,
            SOURCE_CHAIN_BLOCKS,
            sentinel_strategy(),
            config.clone(),
            SOURCE_CHAIN_SEED,
            &mut None,
            &mut None,
        )
        .await;
        let db_path = outcome.abci_app.platform.config.db_path.clone();
        assert_eq!(
            outcome
                .abci_app
                .platform
                .state
                .load()
                .last_committed_block_height(),
            SOURCE_CHAIN_BLOCKS
        );
        drop(outcome);

        // A restore was in progress when the process died.
        write_restore_sentinel(&db_path, &[9u8; 32], 1000).expect("write sentinel");

        let restarted = restart(platform, &config);

        assert_eq!(
            restarted.state.load().last_committed_block_height(),
            0,
            "an unfinished restore must not leave the node claiming a height it cannot back up"
        );
        assert_eq!(
            root_hash(&restarted.platform),
            [0u8; 32],
            "the database must have been wiped to an empty, self-consistent state"
        );
        assert!(
            !restore_sentinel_exists(&db_path),
            "once the node is empty it is self-consistent again, so the sentinel is cleared"
        );
        assert!(
            info_does_not_panic(&restarted),
            "THE WHOLE POINT: the info handshake must succeed, so the node can be offered \
             another snapshot or fall back to block sync instead of crash-looping"
        );
    }

    /// The complement, and the regression that matters most: a node WITHOUT a sentinel
    /// must never be wiped. If startup recovery ever fires unconditionally it would
    /// silently destroy every node's chain on restart.
    #[tokio::test]
    async fn a_normal_restart_keeps_its_state() {
        let config = sentinel_platform_config();
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();
        let outcome = run_chain_for_strategy(
            &mut platform.platform,
            SOURCE_CHAIN_BLOCKS,
            sentinel_strategy(),
            config.clone(),
            SOURCE_CHAIN_SEED,
            &mut None,
            &mut None,
        )
        .await;
        let db_path = outcome.abci_app.platform.config.db_path.clone();
        let healthy_root_hash = root_hash(outcome.abci_app.platform);
        drop(outcome);

        assert!(!restore_sentinel_exists(&db_path));
        let restarted = restart(platform, &config);

        assert_eq!(
            restarted.state.load().last_committed_block_height(),
            SOURCE_CHAIN_BLOCKS,
            "a normal restart must come back at the tip"
        );
        assert_eq!(
            root_hash(&restarted.platform),
            healthy_root_hash,
            "a normal restart must not touch the database"
        );
        assert!(info_does_not_panic(&restarted));
    }

    /// The block-sync arm of the recovery. If every offered snapshot is rejected,
    /// Tenderdash gives up on state sync and block-syncs from genesis. `init_chain` is
    /// where the node becomes self-consistent again, so it must clear a sentinel left over
    /// from the abandoned restore — otherwise the NEXT restart would wipe a perfectly good
    /// chain.
    #[tokio::test]
    async fn init_chain_clears_a_sentinel_left_by_an_abandoned_restore() {
        let config = sentinel_platform_config();
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();
        let db_path = platform.platform.config.db_path.clone();

        // An abandoned restore: the marker is present and the database is empty.
        write_restore_sentinel(&db_path, &[9u8; 32], 1000).expect("write sentinel");

        // Block sync from genesis, which begins with init_chain.
        let outcome = run_chain_for_strategy(
            &mut platform.platform,
            SOURCE_CHAIN_BLOCKS,
            sentinel_strategy(),
            config.clone(),
            SOURCE_CHAIN_SEED,
            &mut None,
            &mut None,
        )
        .await;

        assert_eq!(
            outcome
                .abci_app
                .platform
                .state
                .load()
                .last_committed_block_height(),
            SOURCE_CHAIN_BLOCKS,
            "the node must block-sync normally after an abandoned restore"
        );
        assert!(
            !restore_sentinel_exists(&db_path),
            "init_chain makes the node self-consistent, so it must clear the sentinel — \
             otherwise the next restart would wipe this chain"
        );
        drop(outcome);

        // And prove it: a restart keeps the block-synced chain.
        let restarted = restart(platform, &config);
        assert_eq!(
            restarted.state.load().last_committed_block_height(),
            SOURCE_CHAIN_BLOCKS,
            "the chain built after an abandoned restore must survive a restart"
        );
    }

    /// End to end for the remotely-triggerable case: a peer offers a snapshot this node
    /// cannot use, and the node must end up able to sync rather than wedged.
    ///
    /// The snapshot here is a pre-v15 one (a v14 chain's checkpoint, which carries no
    /// reduced platform state). Nothing stops a peer from advertising it: `proto::Snapshot`
    /// carries a height, a wire version and a hash, and no protocol version at all.
    ///
    /// This test is pin-agnostic on purpose. With grovedb #840 the refusal comes from the
    /// missing reduced platform state; at the unpatched revision the sum-tree defect makes
    /// the post-restore verification fail first. Either way the snapshot is refused AFTER
    /// the session was committed, which is precisely the path that has to leave the node
    /// recoverable.
    #[tokio::test]
    async fn an_unusable_snapshot_leaves_the_node_able_to_sync() {
        let config = sentinel_platform_config();

        let mut source_platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .with_initial_protocol_version(14)
            .build_with_mock_rpc();
        let ChainExecutionOutcome {
            abci_app: source_app,
            proposers,
            validator_quorums,
            ..
        } = run_chain_for_strategy(
            &mut source_platform.platform,
            SOURCE_CHAIN_BLOCKS,
            sentinel_strategy(),
            config.clone(),
            SOURCE_CHAIN_SEED,
            &mut None,
            &mut None,
        )
        .await;

        let (height, checkpoint) = {
            let checkpoints = source_app.platform.drive.checkpoints.load();
            let (height, info) = checkpoints
                .last_key_value()
                .expect("at least one checkpoint");
            (*height, std::sync::Arc::clone(&info.checkpoint))
        };
        let platform_version = PlatformVersion::latest();
        let checkpoint_root = checkpoint
            .grove_db
            .root_hash(None, &platform_version.drive.grove_version)
            .unwrap()
            .expect("checkpoint root hash");
        let forged_snapshot = proto::Snapshot {
            height,
            version: platform_version.drive_abci.state_sync.protocol_version as u32,
            hash: checkpoint_root.to_vec(),
            metadata: vec![],
        };

        let mut target_platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();
        install_reconstruction_core_mocks(
            &mut target_platform.platform,
            proposers
                .iter()
                .map(|proposer| proposer.masternode.clone())
                .collect(),
            &validator_quorums,
        );
        let db_path = target_platform.platform.config.db_path.clone();

        {
            let target_app = FullAbciApplication::new(&target_platform);
            let outcome = sync_snapshot(&source_app, &target_app, &forged_snapshot, false)
                .expect("an unusable snapshot must be answered, not errored");
            assert_eq!(
                outcome,
                SnapshotSyncOutcome::Rejected,
                "an unusable snapshot must be answered with REJECT_SNAPSHOT so Tenderdash \
                 tries the next one instead of aborting state sync"
            );

            // The node wiped itself back to a clean slate rather than keeping state it
            // cannot use...
            assert_ne!(
                root_hash(&target_platform.platform).to_vec(),
                forged_snapshot.hash,
                "the refused snapshot must not be left on disk"
            );
            assert_eq!(
                root_hash(&target_platform.platform),
                [0u8; 32],
                "the refusal must leave an empty database"
            );
            // ...and the sentinel stays, because the in-memory platform state may still
            // describe the chain the offer wiped. Whatever happens next resolves it.
            assert!(
                restore_sentinel_exists(&db_path),
                "the node is empty but not yet provably consistent, so the marker stays \
                 until a restore succeeds, an init_chain runs, or a restart wipes"
            );
        }

        // A restart is the worst case, and it recovers.
        let restarted = restart(target_platform, &config);
        assert_eq!(restarted.state.load().last_committed_block_height(), 0);
        assert_eq!(root_hash(&restarted.platform), [0u8; 32]);
        assert!(
            !restore_sentinel_exists(&db_path),
            "startup recovery leaves the node self-consistent and clears the marker"
        );
        assert!(
            info_does_not_panic(&restarted),
            "THE FIX: a peer offering an unusable snapshot must not be able to wedge this \
             node. Before the fix, info panicked here and drive-abci crash-looped."
        );
    }
}
