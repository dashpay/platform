//! Two-instance ABCI state sync integration tests: a source chain serves snapshots from
//! its checkpoint registry and a fresh target restores one chunk by chunk, then
//! reconstructs its platform state.
//!
//! KNOWN LIMITATION at the pinned grovedb revision (6c882c3): state sync does not
//! faithfully restore SumTree subtrees — the copied node hashes reproduce the source
//! root hash, but re-opening a restored sum tree recomputes a different root (latent
//! corruption), which the strict `verify_grovedb` call in `apply_snapshot_chunk`
//! correctly refuses. See `tests/sum_tree_sync_probe.rs` for the minimal upstream
//! reproducer. The full happy-path test below is therefore `#[ignore]`d until the
//! grovedb pin includes the sum-tree restore fix (dashpay/grovedb#840), and an active
//! test pins today's refusal behavior instead.

#[cfg(test)]
pub(crate) mod tests {
    use crate::execution::run_chain_for_strategy;
    use crate::strategy::{ChainExecutionOutcome, NetworkStrategy};
    use dpp::dashcore::hashes::Hash;
    use dpp::dashcore::{BlockHash, QuorumHash};
    use dpp::dashcore_rpc::dashcore_rpc_json::{
        ExtendedQuorumDetails, MasternodeListDiff, MasternodeListItem, QuorumInfoResult,
    };
    use dpp::dashcore_rpc::json::{ExtendedQuorumListResult, QuorumType};
    use dpp::version::PlatformVersion;
    use drive_abci::abci::app::FullAbciApplication;
    use drive_abci::config::{
        ChainLockConfig, ExecutionConfig, InstantLockConfig, PlatformConfig, PlatformTestConfig,
        ValidatorSetConfig,
    };
    use drive_abci::mimic::test_quorum::TestQuorumInfo;
    use drive_abci::platform_types::platform::Platform;
    use drive_abci::platform_types::platform_state::PlatformStateV0Methods;
    use drive_abci::rpc::core::MockCoreRPCLike;
    use drive_abci::test::helpers::setup::TestPlatformBuilder;
    use std::collections::{BTreeMap, HashMap, VecDeque};
    use strategy_tests::frequency::Frequency;
    use strategy_tests::{IdentityInsertInfo, StartAddresses, StartIdentities, Strategy};
    use tenderdash_abci::proto::abci as proto;
    use tenderdash_abci::proto::abci::{response_apply_snapshot_chunk, response_offer_snapshot};
    use tenderdash_abci::Application;

    /// A quiet chain with a trickle of identity inserts, no masternode churn and no
    /// quorum rotation, so the target's from-scratch Core re-derivation sees exactly
    /// the same masternodes and quorums the source chain ran with.
    fn state_sync_network_strategy() -> NetworkStrategy {
        NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
                start_addresses: StartAddresses::default(),
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
            verify_state_transition_results: false,
            ..Default::default()
        }
    }

    /// Snapshot serving on with a 1s frequency (every 3s block crosses the boundary,
    /// so every block after the first creates a checkpoint), keeping 3 checkpoints.
    fn state_sync_platform_config() -> PlatformConfig {
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

    /// Installs on a fresh target the Core RPC answers its platform state
    /// reconstruction will ask for: the full masternode list (the target requests it
    /// from scratch, base height None) and the same quorums the source ran with.
    pub(crate) fn install_reconstruction_core_mocks(
        platform: &mut Platform<MockCoreRPCLike>,
        masternodes: Vec<MasternodeListItem>,
        validator_quorums: &BTreeMap<QuorumHash, TestQuorumInfo>,
    ) {
        platform
            .core_rpc
            .expect_get_protx_diff_with_masternodes()
            .returning(move |base_block, block| {
                assert!(
                    base_block.is_none(),
                    "state reconstruction must request the full masternode list from scratch"
                );
                Ok(MasternodeListDiff {
                    base_height: 0,
                    block_height: block,
                    added_mns: masternodes.clone(),
                    removed_mns: vec![],
                    updated_mns: vec![],
                })
            });

        let quorum_details: Vec<(QuorumHash, ExtendedQuorumDetails)> = validator_quorums
            .keys()
            .map(|quorum_hash| {
                (
                    *quorum_hash,
                    ExtendedQuorumDetails {
                        creation_height: 0,
                        quorum_index: None,
                        mined_block_hash: BlockHash::all_zeros(),
                        num_valid_members: 0,
                        health_ratio: 0.0,
                    },
                )
            })
            .collect();
        platform
            .core_rpc
            .expect_get_quorum_listextended()
            .returning(move |_| {
                Ok(ExtendedQuorumListResult {
                    quorums_by_type: HashMap::from([(
                        QuorumType::Llmq100_67,
                        quorum_details.clone().into_iter().collect(),
                    )]),
                })
            });

        let quorum_infos: HashMap<QuorumHash, QuorumInfoResult> = validator_quorums
            .iter()
            .map(|(quorum_hash, test_quorum_info)| (*quorum_hash, test_quorum_info.into()))
            .collect();
        platform.core_rpc.expect_get_quorum_info().returning(
            move |_, quorum_hash: &QuorumHash, _| {
                Ok(quorum_infos
                    .get::<QuorumHash>(quorum_hash)
                    .unwrap_or_else(|| {
                        panic!("expected to get quorum {}", hex::encode(quorum_hash))
                    })
                    .clone())
            },
        );
    }

    /// Drives the chunk transfer loop between a serving app and a restoring app,
    /// modeled on grovedb's run_sync driver: start from the root chunk (id == app
    /// hash) and keep requesting whatever the target asks for next.
    ///
    /// When `tamper_with_first_chunk` is set, the first served chunk is corrupted to
    /// prove the target answers RETRY with a refetch of exactly that chunk (banning
    /// the sender) instead of killing the session. At the current grovedb revision the
    /// refetched chunk cannot be re-applied within the session (grovedb removes a
    /// chunk id from its pending set before processing), so the target then answers
    /// RETRY_SNAPSHOT; the driver handles that the way Tenderdash would, by
    /// re-offering the same snapshot and restarting the transfer.
    /// How a snapshot transfer ended.
    ///
    /// `Rejected` is not an error: the target restored the snapshot, found it unusable,
    /// wiped itself back to a clean slate and asked Tenderdash for a different one. The
    /// driver reports it so tests can tell a clean refusal from a transport failure.
    #[derive(Debug, PartialEq, Eq)]
    pub(crate) enum SnapshotSyncOutcome {
        /// The target restored and accepted the snapshot.
        Completed,
        /// The target answered REJECT_SNAPSHOT; Tenderdash would move on to the next one.
        Rejected,
    }

    pub(crate) fn sync_snapshot(
        source_app: &FullAbciApplication<MockCoreRPCLike>,
        target_app: &FullAbciApplication<MockCoreRPCLike>,
        snapshot: &proto::Snapshot,
        tamper_with_first_chunk: bool,
    ) -> Result<SnapshotSyncOutcome, proto::ResponseException> {
        let mut tamper_next = tamper_with_first_chunk;
        let mut restarts = 0usize;

        'snapshot_attempt: loop {
            let offer_response = target_app.offer_snapshot(proto::RequestOfferSnapshot {
                snapshot: Some(snapshot.clone()),
                app_hash: snapshot.hash.clone(),
            })?;
            assert_eq!(
                offer_response.result,
                i32::from(response_offer_snapshot::Result::Accept),
                "target must accept the offered snapshot"
            );

            let mut chunk_queue: VecDeque<Vec<u8>> = VecDeque::from([snapshot.hash.clone()]);

            while let Some(chunk_id) = chunk_queue.pop_front() {
                let chunk = source_app
                    .load_snapshot_chunk(proto::RequestLoadSnapshotChunk {
                        height: snapshot.height,
                        version: snapshot.version,
                        chunk_id: chunk_id.clone(),
                    })?
                    .chunk;

                if tamper_next {
                    tamper_next = false;
                    let mut tampered = chunk.clone();
                    let last = tampered.len() - 1;
                    tampered[last] ^= 0xff;

                    let response =
                        target_app.apply_snapshot_chunk(proto::RequestApplySnapshotChunk {
                            chunk_id: chunk_id.clone(),
                            chunk: tampered,
                            sender: "malicious-peer".to_string(),
                        })?;
                    assert_eq!(
                        response.result,
                        i32::from(response_apply_snapshot_chunk::Result::Retry),
                        "a tampered chunk must be answered with a retry, not kill the session"
                    );
                    assert_eq!(
                        response.refetch_chunks,
                        vec![chunk_id.clone()],
                        "the tampered chunk must be refetched"
                    );
                    assert_eq!(response.reject_senders, vec!["malicious-peer".to_string()]);
                    assert!(
                        target_app
                            .snapshot_fetching_session
                            .read()
                            .unwrap()
                            .is_some(),
                        "the session must survive a tampered chunk"
                    );
                }

                let response =
                    target_app.apply_snapshot_chunk(proto::RequestApplySnapshotChunk {
                        chunk_id,
                        chunk,
                        sender: "honest-peer".to_string(),
                    })?;

                match response.result {
                    result
                        if result == i32::from(response_apply_snapshot_chunk::Result::Accept) =>
                    {
                        chunk_queue.extend(response.next_chunks);
                    }
                    result
                        if result
                            == i32::from(
                                response_apply_snapshot_chunk::Result::CompleteSnapshot,
                            ) =>
                    {
                        assert!(
                            chunk_queue.is_empty(),
                            "transfer completed with chunks still queued"
                        );
                        return Ok(SnapshotSyncOutcome::Completed);
                    }
                    result
                        if result
                            == i32::from(response_apply_snapshot_chunk::Result::RejectSnapshot) =>
                    {
                        // The target restored the snapshot, found it unusable and wiped
                        // itself back to a clean slate. Tenderdash would try the next
                        // snapshot; there is nothing more for this driver to do.
                        assert!(
                            target_app
                                .snapshot_fetching_session
                                .read()
                                .unwrap()
                                .is_none(),
                            "a rejected snapshot must not leave a session open"
                        );
                        return Ok(SnapshotSyncOutcome::Rejected);
                    }
                    result
                        if result
                            == i32::from(response_apply_snapshot_chunk::Result::RetrySnapshot) =>
                    {
                        restarts += 1;
                        assert!(restarts <= 2, "too many snapshot restarts");
                        continue 'snapshot_attempt;
                    }
                    other => panic!("unexpected apply_snapshot_chunk result {}", other),
                }
            }

            panic!("chunk transfer ran out of chunks without completing");
        }
    }

    struct SourceChain<'a> {
        source_app: FullAbciApplication<'a, MockCoreRPCLike>,
        proposers: Vec<MasternodeListItem>,
        validator_quorums: BTreeMap<QuorumHash, TestQuorumInfo>,
        snapshot: proto::Snapshot,
    }

    /// Runs the source chain past several checkpoints and picks its newest offered
    /// snapshot.
    async fn run_source_chain<'a>(
        source_platform: &'a mut drive_abci::test::helpers::setup::TempPlatform<MockCoreRPCLike>,
        config: &PlatformConfig,
    ) -> SourceChain<'a> {
        let ChainExecutionOutcome {
            abci_app: source_app,
            proposers,
            validator_quorums,
            ..
        } = run_chain_for_strategy(
            source_platform,
            15,
            state_sync_network_strategy(),
            config.clone(),
            15,
            &mut None,
            &mut None,
        )
        .await;

        let snapshots = source_app
            .list_snapshots(Default::default())
            .expect("source should list snapshots")
            .snapshots;
        assert!(
            !snapshots.is_empty(),
            "the source chain must have produced at least one restorable snapshot"
        );
        let snapshot = snapshots
            .iter()
            .max_by_key(|snapshot| snapshot.height)
            .expect("at least one snapshot")
            .clone();

        SourceChain {
            source_app,
            proposers: proposers
                .iter()
                .map(|proposer| proposer.masternode.clone())
                .collect(),
            validator_quorums,
            snapshot,
        }
    }

    /// End to end: run a source chain past several checkpoints, serve its newest
    /// snapshot, restore it chunk by chunk on a fresh target (with one tampered chunk
    /// along the way to prove refetch/restart recovery), reconstruct the target
    /// platform state, and verify the target matches the source checkpoint exactly.
    // QA BRANCH: un-ignored because the workspace root Cargo.toml carries a TEMPORARY
    // patch redirecting grovedb to the sum-tree restore fix (dashpay/grovedb#840). Restore
    // the `#[ignore]` if that patch is dropped without bumping the real grovedb pin.
    #[tokio::test]
    async fn run_state_sync_between_two_platforms() {
        let config = state_sync_platform_config();
        let mut source_platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();
        let source = run_source_chain(&mut source_platform, &config).await;
        let snapshot = &source.snapshot;

        // The platform state the source had at exactly the snapshot height
        let source_platform_state = source
            .source_app
            .platform
            .checkpoint_platform_states
            .load()
            .get(&snapshot.height)
            .expect("source must cache the platform state of its checkpoint")
            .clone();

        // A fresh target node, knowing nothing but Core RPC
        let mut target_platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();
        install_reconstruction_core_mocks(
            &mut target_platform.platform,
            source.proposers.clone(),
            &source.validator_quorums,
        );
        let target_app = FullAbciApplication::new(&target_platform);

        assert_eq!(
            sync_snapshot(&source.source_app, &target_app, snapshot, true)
                .expect("state sync must not error"),
            SnapshotSyncOutcome::Completed,
            "state sync must complete"
        );

        let platform_version = PlatformVersion::latest();
        let grove_version = &platform_version.drive.grove_version;

        // Grove roots agree between source checkpoint and target
        let target_root_hash = target_platform
            .drive
            .grove
            .root_hash(None, grove_version)
            .unwrap()
            .expect("target root hash");
        assert_eq!(target_root_hash.to_vec(), snapshot.hash);

        // The restored grovedb is internally consistent
        let verification_issues = target_platform
            .drive
            .grove
            .verify_grovedb(None, true, false, grove_version)
            .expect("expected to verify grovedb");
        assert!(
            verification_issues.is_empty(),
            "restored grovedb must verify cleanly: {:?}",
            verification_issues
        );

        // The reconstructed platform state matches the source's state at the snapshot
        // height, except for the fields that are not replicated (block signature and
        // block id hash restore as zeroes)
        let target_state = target_platform.state.load();
        assert_eq!(
            target_state.current_protocol_version_in_consensus(),
            source_platform_state.current_protocol_version_in_consensus()
        );
        assert_eq!(
            target_state.next_epoch_protocol_version(),
            source_platform_state.next_epoch_protocol_version()
        );
        assert_eq!(
            target_state.last_committed_block_height(),
            snapshot.height,
            "target must be at the snapshot height"
        );
        assert_eq!(
            target_state.last_committed_block_app_hash(),
            source_platform_state.last_committed_block_app_hash()
        );
        assert_eq!(
            target_state.current_validator_set_quorum_hash(),
            source_platform_state.current_validator_set_quorum_hash()
        );
        assert_eq!(
            target_state.next_validator_set_quorum_hash(),
            source_platform_state.next_validator_set_quorum_hash()
        );
        assert_eq!(
            target_state.validator_sets().keys().collect::<Vec<_>>(),
            source_platform_state
                .validator_sets()
                .keys()
                .collect::<Vec<_>>(),
            "validator set order must be restored from the recorded quorum positions"
        );
        assert_eq!(
            target_state.validator_sets(),
            source_platform_state.validator_sets(),
            "validator sets must match"
        );
        assert_eq!(
            target_state.full_masternode_list(),
            source_platform_state.full_masternode_list()
        );
        assert_eq!(
            target_state.hpmn_masternode_list(),
            source_platform_state.hpmn_masternode_list()
        );
        assert_eq!(
            target_state.previous_fee_versions(),
            source_platform_state.previous_fee_versions(),
            "fee versions of previous epochs must be restored faithfully"
        );

        // The target's info handler must pass its own app-hash consistency check and
        // report the snapshot height and hash to Tenderdash's post-sync verifyApp
        let info = target_app
            .info(proto::RequestInfo {
                version: tenderdash_abci::proto::meta::TENDERDASH_VERSION.to_string(),
                block_version: 0,
                p2p_version: 0,
                abci_version: tenderdash_abci::proto::meta::ABCI_VERSION.to_string(),
            })
            .expect("target info handler must succeed");
        assert_eq!(info.last_block_height as u64, snapshot.height);
        assert_eq!(info.last_block_app_hash, snapshot.hash);
    }

    /// Pins today's behavior at the pinned grovedb revision: the transfer itself
    /// completes (including recovery from a tampered chunk via RETRY and a snapshot
    /// restart), but the strict post-restore verification detects that grovedb did not
    /// faithfully restore the sum trees and refuses the snapshot instead of accepting
    /// latent corruption. When this test starts failing because the sync SUCCEEDS,
    /// grovedb has been fixed: un-ignore `run_state_sync_between_two_platforms` and
    /// drop this pin.
    // QA BRANCH ONLY — this test asserts the grovedb sum-tree restore DEFECT is present.
    // The workspace root Cargo.toml carries a TEMPORARY patch redirecting grovedb to the
    // fix (dashpay/grovedb#840), so the sync now SUCCEEDS and this defect-present pin
    // fails by design. Un-ignore (and delete it, as its own doc comment instructs) in the
    // same change that drops the root Cargo.toml patch section.
    #[tokio::test]
    #[ignore = "QA branch: the root Cargo.toml patch to dashpay/grovedb#840 fixes this defect, \
                so this defect-present pin fails by design — see the comment above"]
    async fn state_sync_transfer_detects_sum_tree_restore_defect() {
        let config = state_sync_platform_config();
        let mut source_platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();
        let source = run_source_chain(&mut source_platform, &config).await;

        let mut target_platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();
        install_reconstruction_core_mocks(
            &mut target_platform.platform,
            source.proposers.clone(),
            &source.validator_quorums,
        );
        let target_app = FullAbciApplication::new(&target_platform);

        let outcome = sync_snapshot(&source.source_app, &target_app, &source.snapshot, true)
            .expect("a refused snapshot is answered, not errored");
        assert_eq!(
            outcome,
            SnapshotSyncOutcome::Rejected,
            "at grovedb rev 6c882c3 the restored sum trees must fail verification — if this \
             now completes, grovedb is fixed: un-ignore run_state_sync_between_two_platforms \
             and remove this pin"
        );

        // The target refused the snapshot: it never advanced past genesis, and — since the
        // refusal happens after the session was already committed — it wiped itself back to
        // a clean slate rather than keeping the unusable state.
        assert_eq!(
            target_platform.state.load().last_committed_block_height(),
            0
        );
        assert_ne!(
            target_platform
                .drive
                .grove
                .root_hash(None, &PlatformVersion::latest().drive.grove_version)
                .unwrap()
                .expect("target root hash")
                .to_vec(),
            source.snapshot.hash,
            "a refused snapshot must not be left on disk"
        );
    }

    /// Exercises the platform state reconstruction end to end without going through
    /// the (currently defective, see above) grovedb chunk restore: the source chain's
    /// own grovedb IS a faithfully "restored" snapshot of itself, so reconstructing
    /// on it must (a) not change the grovedb root hash — the proof that re-deriving
    /// masternode identities from Core is byte-idempotent — and (b) reproduce the
    /// source's in-memory platform state from the reduced platform state alone.
    #[tokio::test]
    async fn platform_state_reconstruction_is_idempotent_and_matches_source_state() {
        let config = state_sync_platform_config();
        let mut source_platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();
        let source = run_source_chain(&mut source_platform, &config).await;
        let platform = source.source_app.platform;

        let platform_version = PlatformVersion::latest();
        let grove_version = &platform_version.drive.grove_version;

        let original_state = platform.state.load().clone();
        let tip_app_hash = platform
            .drive
            .grove
            .root_hash(None, grove_version)
            .unwrap()
            .expect("source root hash");
        assert_eq!(
            original_state.last_committed_block_app_hash(),
            Some(tip_app_hash),
            "sanity: chain tip state matches grove root"
        );

        // The run_chain mocks already answer the from-scratch masternode/quorum
        // requests reconstruction makes, exactly as they did for the chain itself.
        platform
            .reconstruct_platform_state(&tip_app_hash, platform_version)
            .expect("platform state reconstruction must succeed");

        // (a) idempotence: re-deriving masternode identities wrote nothing new
        let root_hash_after = platform
            .drive
            .grove
            .root_hash(None, grove_version)
            .unwrap()
            .expect("source root hash after reconstruction");
        assert_eq!(
            root_hash_after, tip_app_hash,
            "reconstruction must not change the grovedb root hash"
        );

        // (b) the reconstructed state matches the original, except the fields the
        // reduced state cannot carry (block signature / block id hash)
        let reconstructed_state = platform.state.load();
        assert_eq!(
            reconstructed_state.current_protocol_version_in_consensus(),
            original_state.current_protocol_version_in_consensus()
        );
        assert_eq!(
            reconstructed_state.next_epoch_protocol_version(),
            original_state.next_epoch_protocol_version()
        );
        assert_eq!(
            reconstructed_state.last_committed_block_height(),
            original_state.last_committed_block_height()
        );
        assert_eq!(
            reconstructed_state.last_committed_block_app_hash(),
            original_state.last_committed_block_app_hash()
        );
        assert_eq!(
            reconstructed_state.last_committed_core_height(),
            original_state.last_committed_core_height()
        );
        assert_eq!(
            reconstructed_state.current_validator_set_quorum_hash(),
            original_state.current_validator_set_quorum_hash()
        );
        assert_eq!(
            reconstructed_state.next_validator_set_quorum_hash(),
            original_state.next_validator_set_quorum_hash()
        );
        assert_eq!(
            reconstructed_state
                .validator_sets()
                .keys()
                .collect::<Vec<_>>(),
            original_state.validator_sets().keys().collect::<Vec<_>>(),
            "validator set order must be restored from the recorded quorum positions"
        );
        assert_eq!(
            reconstructed_state.validator_sets(),
            original_state.validator_sets()
        );
        assert_eq!(
            reconstructed_state.full_masternode_list(),
            original_state.full_masternode_list()
        );
        assert_eq!(
            reconstructed_state.hpmn_masternode_list(),
            original_state.hpmn_masternode_list()
        );
        assert_eq!(
            reconstructed_state.previous_fee_versions(),
            original_state.previous_fee_versions()
        );

        // The info handler accepts the reconstructed state (it panics on an app-hash
        // mismatch between the in-memory state and the grove root)
        let info = source
            .source_app
            .info(proto::RequestInfo {
                version: tenderdash_abci::proto::meta::TENDERDASH_VERSION.to_string(),
                block_version: 0,
                p2p_version: 0,
                abci_version: tenderdash_abci::proto::meta::ABCI_VERSION.to_string(),
            })
            .expect("info handler must accept the reconstructed state");
        assert_eq!(
            info.last_block_height as u64,
            original_state.last_committed_block_height()
        );
        assert_eq!(info.last_block_app_hash, tip_app_hash.to_vec());
    }

    /// A snapshot from a chain that never wrote the reduced platform state (pre-v15)
    /// is not offered by the source, and a target driven at it anyway refuses to
    /// restore it.
    #[tokio::test]
    async fn pre_v15_snapshot_is_not_served_and_cannot_be_restored() {
        let config = state_sync_platform_config();

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
            &mut source_platform,
            15,
            state_sync_network_strategy(),
            config.clone(),
            15,
            &mut None,
            &mut None,
        )
        .await;

        // The v14 chain created checkpoints, but none carries the reduced platform
        // state, so none may be offered.
        assert!(
            !source_app.platform.drive.checkpoints.load().is_empty(),
            "the source must have created checkpoints"
        );
        let snapshots = source_app
            .list_snapshots(Default::default())
            .expect("source should list snapshots")
            .snapshots;
        assert!(
            snapshots.is_empty(),
            "pre-v15 checkpoints are unrestorable and must not be offered"
        );

        // Even if a peer maliciously offers such a snapshot, the target must refuse to
        // restore it. (At the current grovedb revision the refusal comes from the
        // post-restore verification; once grovedb faithfully restores sum trees it
        // comes from the missing reduced platform state at the reconstruction step.
        // Either way the snapshot must not be accepted.)
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
            version: 1,
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
        let target_app = FullAbciApplication::new(&target_platform);

        let outcome = sync_snapshot(&source_app, &target_app, &forged_snapshot, false)
            .expect("a refused snapshot is answered, not errored");
        assert_eq!(
            outcome,
            SnapshotSyncOutcome::Rejected,
            "a snapshot without the reduced platform state must be refused"
        );

        // The target holds no usable platform state: it never advanced past genesis
        assert_eq!(
            target_platform.state.load().last_committed_block_height(),
            0
        );
        // ...and it did not keep the state it could not use: the refusal wipes back to a
        // clean slate so Tenderdash can offer another snapshot or fall back to block sync.
        assert_ne!(
            target_platform
                .drive
                .grove
                .root_hash(None, &platform_version.drive.grove_version)
                .unwrap()
                .expect("target root hash")
                .to_vec(),
            forged_snapshot.hash,
            "a refused snapshot must not be left on disk"
        );
    }
}
