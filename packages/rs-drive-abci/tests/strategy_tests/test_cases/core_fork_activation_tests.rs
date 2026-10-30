//! The record of the Core forks Platform has seen active, across a chain's life: a restart, a
//! Core that stops answering, a protocol upgrade, and rounds that are thrown away.
//!
//! Platform asks Core whether V24 is active only in a block that moves the chain locked height,
//! reading the payload version of the coinbase at that height, until it records the first
//! height at which it is. Every validator must hold the same record after every block.
#[cfg(test)]
mod tests {
    use crate::addresses_with_balance::AddressesWithBalance;
    use crate::execution::{continue_chain_for_strategy, run_chain_for_strategy, GENESIS_TIME_MS};
    use crate::strategy::{
        ChainExecutionOutcome, ChainExecutionParameters, CoreHeightIncrease, FailureStrategy,
        NetworkStrategy, StrategyRandomness, UpgradingInfo,
    };
    use dash_platform_macros::stack_size;
    use dpp::dashcore::hashes::Hash;
    use dpp::dashcore::{BlockHash, ChainLock};
    use dpp::dashcore_rpc::Error as CoreRpcError;
    use dpp::version::PlatformVersion;
    use dpp::version::ProtocolVersion;
    use drive::drive::system::core_fork_activations::CoreFork;
    use drive_abci::abci::app::FullAbciApplication;
    use drive_abci::config::{
        ChainLockConfig, ExecutionConfig, InstantLockConfig, PlatformConfig, PlatformTestConfig,
        ValidatorSetConfig,
    };
    use drive_abci::mimic::CHAIN_ID;
    use drive_abci::platform_types::platform::Platform;
    use drive_abci::platform_types::platform_state::PlatformStateV0Methods;
    use drive_abci::rpc::core::MockCoreRPCLike;
    use drive_abci::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
    use std::collections::BTreeMap;
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
    use std::sync::{Arc, Mutex};
    use tenderdash_abci::proto::abci::response_process_proposal::ProposalStatus;
    use tenderdash_abci::proto::abci::{CommitInfo, RequestFinalizeBlock, RequestProcessProposal};
    use tenderdash_abci::proto::google::protobuf::Timestamp;
    use tenderdash_abci::proto::types::{
        Block, BlockId, Data, EvidenceList, Header, PartSetHeader,
    };
    use tenderdash_abci::proto::version::Consensus;
    use tenderdash_abci::proto::FromMillis;
    use tenderdash_abci::Application;

    const BLOCK_SPACING_MS: u64 = 1_000;
    const BLOCKS_PER_EPOCH: u64 = 60;
    /// The highest Core height these tests reach
    const MAX_CORE_HEIGHT: u32 = 8;

    /// Dash Core as these tests see it
    struct Core {
        /// The chain locked height Core reports
        chain_locked_height: AtomicU32,
        /// The first Core height whose coinbase has payload version 4, which Core requires
        /// exactly while V24 is active
        v24_activation_height: u32,
        /// Whether reading a coinbase fails, as it does while Core is down
        coinbase_down: AtomicBool,
        /// Every Core height Platform asked the coinbase of, in order
        coinbase_reads: Mutex<Vec<u32>>,
    }

    impl Core {
        fn new(v24_activation_height: u32) -> Arc<Self> {
            Arc::new(Self {
                chain_locked_height: AtomicU32::new(1),
                v24_activation_height,
                coinbase_down: AtomicBool::new(false),
                coinbase_reads: Mutex::new(vec![]),
            })
        }

        fn chain_lock_at(&self, core_height: u32) {
            self.chain_locked_height
                .store(core_height, Ordering::SeqCst);
        }

        fn coinbase_reads(&self) -> Vec<u32> {
            self.coinbase_reads.lock().unwrap().clone()
        }
    }

    fn strategy(upgrading_info: Option<UpgradingInfo>) -> NetworkStrategy {
        NetworkStrategy {
            total_hpmns: 50,
            validator_quorum_count: 24,
            chain_lock_quorum_count: 24,
            upgrading_info,
            // Mock Core block hashes for every height these tests chain lock; `Core` reports
            // which height is chain locked.
            core_height_increase: CoreHeightIncrease::KnownCoreHeightIncreases(
                (1..=MAX_CORE_HEIGHT).collect(),
            ),
            ..Default::default()
        }
    }

    fn config() -> PlatformConfig {
        PlatformConfig {
            validator_set: ValidatorSetConfig {
                quorum_size: 30,
                ..Default::default()
            },
            chain_lock: ChainLockConfig::default_100_67(),
            instant_lock: InstantLockConfig::default_100_67(),
            execution: ExecutionConfig {
                verify_sum_trees: true,
                epoch_time_length_s: BLOCKS_PER_EPOCH,
                ..Default::default()
            },
            block_spacing_ms: BLOCK_SPACING_MS,
            testing_configs: PlatformTestConfig {
                store_platform_state: true,
                ..PlatformTestConfig::default_minimal_verifications()
            },
            ..Default::default()
        }
    }

    /// A platform at `protocol_version` whose Core is `core`. Core's credit pool is large
    /// enough that the Core-anchored withdrawal limit never binds.
    fn platform_with_core(
        protocol_version: ProtocolVersion,
        core: &Arc<Core>,
    ) -> TempPlatform<MockCoreRPCLike> {
        let mut platform = TestPlatformBuilder::new()
            .with_config(config())
            .with_initial_protocol_version(protocol_version)
            .build_with_mock_rpc();

        // The default answers would shadow the ones below.
        platform.core_rpc.checkpoint();
        platform
            .core_rpc
            .expect_get_block_hash()
            .returning(|_| Ok(BlockHash::all_zeros()));
        platform
            .core_rpc
            .expect_get_block_json()
            .returning(|_| Ok(serde_json::json!({ "tx": [] })));
        // 10 million Dash, in duffs.
        platform
            .core_rpc
            .expect_get_credit_pool_balance()
            .returning(|_| Ok(1_000_000_000_000_000));

        let state = Arc::clone(core);
        platform
            .core_rpc
            .expect_get_coinbase_payload_version()
            .returning(move |core_height| {
                state.coinbase_reads.lock().unwrap().push(core_height);
                if state.coinbase_down.load(Ordering::SeqCst) {
                    return Err(CoreRpcError::UnexpectedStructure(
                        "Core is down".to_string(),
                    ));
                }
                let v24_active = core_height >= state.v24_activation_height;
                Ok(Some(if v24_active { 4 } else { 3 }))
            });

        let state = Arc::clone(core);
        platform
            .core_rpc
            .expect_get_best_chain_lock()
            .returning(move || {
                Ok(ChainLock {
                    block_height: state.chain_locked_height.load(Ordering::SeqCst),
                    block_hash: BlockHash::from_byte_array([1; 32]),
                    signature: [2; 96].into(),
                })
            });

        platform
    }

    /// The node restarted from its data directory, with the same Core
    fn restarted(platform: TempPlatform<MockCoreRPCLike>) -> TempPlatform<MockCoreRPCLike> {
        let TempPlatform {
            platform: mut stopped,
            tempdir,
        } = platform;
        let core_rpc = std::mem::take(&mut stopped.core_rpc);
        drop(stopped);
        let mut platform = TempPlatform::open_with_tempdir(tempdir, config());
        platform.platform.core_rpc = core_rpc;
        platform
    }

    fn continuation(outcome: &ChainExecutionOutcome, block_count: u64) -> ChainExecutionParameters {
        ChainExecutionParameters {
            block_start: outcome
                .abci_app
                .platform
                .state
                .load()
                .last_committed_block_height()
                + 1,
            core_height_start: 1,
            block_count,
            proposers: outcome.proposers.clone(),
            validator_quorums: outcome.validator_quorums.clone(),
            current_validator_quorum_hash: outcome.current_validator_quorum_hash,
            current_proposer_versions: Some(outcome.current_proposer_versions.clone()),
            current_identity_nonce_counter: outcome.identity_nonce_counter.clone(),
            current_identity_contract_nonce_counter: outcome
                .identity_contract_nonce_counter
                .clone(),
            current_votes: BTreeMap::default(),
            start_time_ms: GENESIS_TIME_MS,
            current_time_ms: outcome.end_time_ms,
            instant_lock_quorums: outcome.instant_lock_quorums.clone(),
            current_identities: Vec::new(),
            current_addresses_with_balance: AddressesWithBalance::default(),
        }
    }

    /// Commits `block_count` blocks after `outcome` at the height `core` chain locks
    async fn continue_chain<'a>(
        platform: &'a Platform<MockCoreRPCLike>,
        outcome: &ChainExecutionOutcome<'_>,
        block_count: u64,
        upgrading_info: Option<UpgradingInfo>,
        seed: u64,
    ) -> ChainExecutionOutcome<'a> {
        continue_chain_for_strategy(
            FullAbciApplication::new(platform),
            continuation(outcome, block_count),
            strategy(upgrading_info),
            config(),
            StrategyRandomness::SeedEntropy(seed),
        )
        .await
    }

    /// The committed V24 record
    fn committed_v24(platform: &Platform<MockCoreRPCLike>) -> Option<u32> {
        platform
            .drive
            .fetch_core_fork_activation_height(CoreFork::V24, None, PlatformVersion::latest())
            .expect("expected to read the V24 record")
    }

    /// Once V24 is recorded Platform never asks Core about it again, also after a restart from
    /// disk, so nothing node-local stands in for the record: a node whose Core then refuses to
    /// tell the coinbase payload version commits the same chain as a node whose Core answers.
    /// Only the V24 question fails here; the withdrawal scan still reads Core's credit pool in
    /// the blocks that move the chain locked height, as it does at protocol version 14.
    #[stack_size(4 * 1024 * 1024)]
    #[test]
    async fn should_never_ask_core_about_v24_again_once_it_is_recorded_even_after_a_restart() {
        let mut app_hashes = vec![];
        for core_down_after_a_restart in [false, true] {
            let core = Core::new(3);
            let mut platform =
                platform_with_core(PlatformVersion::latest().protocol_version, &core);

            let outcome = run_chain_for_strategy(
                &mut platform,
                3,
                strategy(None),
                config(),
                40,
                &mut None,
                &mut None,
            )
            .await;
            assert_eq!(
                core.coinbase_reads(),
                vec![1],
                "only init chain asks, at the genesis core height"
            );
            assert_eq!(committed_v24(outcome.abci_app.platform), None);

            core.chain_lock_at(3);
            let recorded = continue_chain(outcome.abci_app.platform, &outcome, 2, None, 41).await;
            drop(outcome);
            assert_eq!(committed_v24(recorded.abci_app.platform), Some(3));
            assert_eq!(core.coinbase_reads(), vec![1, 3]);
            let parameters = continuation(&recorded, 2);
            drop(recorded);

            if core_down_after_a_restart {
                platform = restarted(platform);
                core.coinbase_down.store(true, Ordering::SeqCst);
            }

            let mut parameters = parameters;
            for (core_height, seed) in [(4, 42), (6, 43)] {
                core.chain_lock_at(core_height);
                let outcome = continue_chain_for_strategy(
                    FullAbciApplication::new(&platform),
                    parameters,
                    strategy(None),
                    config(),
                    StrategyRandomness::SeedEntropy(seed),
                )
                .await;
                assert_eq!(
                    outcome
                        .abci_app
                        .platform
                        .state
                        .load()
                        .last_committed_core_height(),
                    core_height
                );
                parameters = continuation(&outcome, 2);
            }

            assert_eq!(committed_v24(&platform), Some(3));
            assert_eq!(
                core.coinbase_reads(),
                vec![1, 3],
                "Core must not be asked about V24 once it is recorded"
            );
            let state = platform.state.load();
            assert_eq!(state.last_committed_block_height(), 9);
            app_hashes.push(state.last_committed_block_app_hash());
        }
        assert_eq!(
            app_hashes[0], app_hashes[1],
            "the node whose Core went down after a restart must commit the same chain"
        );
    }

    /// The upgrade block from protocol version 14 asks Core nothing when it keeps the chain
    /// locked height, so V24 is recorded at the first protocol version 15 block that moves it.
    /// A node reopened from disk right after the upgrade block keeps its last committed core
    /// height, 2 rather than the genesis core height, asks nothing in the next block either, and
    /// commits the same chain as a warm node. When the upgrade block itself moves the height, it
    /// records V24 there.
    #[stack_size(4 * 1024 * 1024)]
    #[test]
    async fn should_record_v24_at_the_first_protocol_version_15_block_that_moves_the_chain_locked_height(
    ) {
        let previous_version = PlatformVersion::get(14).expect("expected protocol version 14");
        let latest_version = PlatformVersion::latest();
        let upgrading_info = Some(UpgradingInfo {
            current_protocol_version: previous_version.protocol_version,
            proposed_protocol_versions_with_weight: vec![(latest_version.protocol_version, 1)],
            upgrade_three_quarters_life: 0.0,
        });
        let mut app_hashes = vec![];
        for (activation_block_moves_core_height, reopen_after_activation) in
            [(false, false), (false, true), (true, false)]
        {
            // V24 is active at every Core height these blocks chain lock.
            let core = Core::new(1);
            let mut platform = platform_with_core(previous_version.protocol_version, &core);

            let epoch_1 = run_chain_for_strategy(
                &mut platform,
                2 * BLOCKS_PER_EPOCH - 1,
                strategy(upgrading_info.clone()),
                config(),
                13,
                &mut None,
                &mut None,
            )
            .await;
            // The last protocol version 14 block moves the chain locked height.
            core.chain_lock_at(2);
            let before_activation = continue_chain(
                epoch_1.abci_app.platform,
                &epoch_1,
                1,
                upgrading_info.clone(),
                17,
            )
            .await;
            drop(epoch_1);
            {
                let state = before_activation.abci_app.platform.state.load();
                assert_eq!(state.current_protocol_version_in_consensus(), 14);
                assert_eq!(
                    state.next_epoch_protocol_version(),
                    latest_version.protocol_version
                );
                assert_eq!(state.last_committed_core_height(), 2);
            }
            assert!(
                core.coinbase_reads().is_empty(),
                "protocol version 14 never asks Core about V24"
            );
            assert_eq!(committed_v24(before_activation.abci_app.platform), None);

            if activation_block_moves_core_height {
                core.chain_lock_at(3);
            }
            let activation = continue_chain(
                before_activation.abci_app.platform,
                &before_activation,
                1,
                upgrading_info.clone(),
                18,
            )
            .await;
            drop(before_activation);
            assert_eq!(
                activation
                    .abci_app
                    .platform
                    .state
                    .load()
                    .current_protocol_version_in_consensus(),
                latest_version.protocol_version
            );

            if activation_block_moves_core_height {
                assert_eq!(committed_v24(activation.abci_app.platform), Some(3));
                assert_eq!(core.coinbase_reads(), vec![3]);
                continue;
            }

            // The window: protocol version 15, V24 active in Core, not recorded yet.
            assert_eq!(committed_v24(activation.abci_app.platform), None);
            assert!(core.coinbase_reads().is_empty());
            let parameters = continuation(&activation, 1);
            drop(activation);

            if reopen_after_activation {
                platform = restarted(platform);
                assert_eq!(platform.state.load().last_committed_core_height(), 2);
            }

            let window = continue_chain_for_strategy(
                FullAbciApplication::new(&platform),
                parameters,
                strategy(upgrading_info.clone()),
                config(),
                StrategyRandomness::SeedEntropy(19),
            )
            .await;
            assert_eq!(committed_v24(window.abci_app.platform), None);
            assert!(core.coinbase_reads().is_empty());

            core.chain_lock_at(3);
            let recorded = continue_chain(
                window.abci_app.platform,
                &window,
                1,
                upgrading_info.clone(),
                20,
            )
            .await;
            drop(window);
            assert_eq!(committed_v24(recorded.abci_app.platform), Some(3));
            assert_eq!(core.coinbase_reads(), vec![3]);
            app_hashes.push(
                recorded
                    .abci_app
                    .platform
                    .state
                    .load()
                    .last_committed_block_app_hash(),
            );
        }
        assert_eq!(
            app_hashes[0], app_hashes[1],
            "a node reopened inside the window must commit the same chain as a warm node"
        );
    }

    /// The proposal of `round` for the next height, at chain locked `core_chain_locked_height`
    fn proposal(
        outcome: &ChainExecutionOutcome,
        round: u32,
        core_chain_locked_height: u32,
    ) -> RequestProcessProposal {
        let platform_state = outcome.abci_app.platform.state.load();
        let height = platform_state.last_committed_block_height() + 1;
        let time_ms = outcome.end_time_ms + BLOCK_SPACING_MS + round as u64 * 1000;
        let proposer = outcome.proposers[round as usize].pro_tx_hash();

        RequestProcessProposal {
            txs: vec![],
            proposed_last_commit: None,
            misbehavior: vec![],
            hash: [0xA0 + round as u8; 32].to_vec(),
            height: height as i64,
            time: Some(Timestamp::from_millis(time_ms).expect("expected a block time")),
            next_validators_hash: [0u8; 32].to_vec(),
            round: round as i32,
            core_chain_locked_height,
            core_chain_lock_update: None,
            proposer_pro_tx_hash: proposer.to_byte_array().to_vec(),
            proposed_app_version: PlatformVersion::latest().protocol_version as u64,
            version: Some(Consensus {
                block: 0,
                app: PlatformVersion::latest().protocol_version as u64,
            }),
            quorum_hash: outcome
                .current_quorum()
                .quorum_hash
                .to_byte_array()
                .to_vec(),
        }
    }

    /// The request Tenderdash finalizes `proposal` with once it is committed in its own round,
    /// with `app_hash` in the block header
    fn finalize_request(
        proposal: &RequestProcessProposal,
        app_hash: Vec<u8>,
    ) -> RequestFinalizeBlock {
        RequestFinalizeBlock {
            commit: Some(CommitInfo {
                round: proposal.round,
                quorum_hash: proposal.quorum_hash.clone(),
                block_signature: vec![0u8; 96],
                threshold_vote_extensions: vec![],
            }),
            misbehavior: vec![],
            hash: proposal.hash.clone(),
            height: proposal.height,
            round: proposal.round,
            block: Some(Block {
                header: Some(Header {
                    version: proposal.version,
                    chain_id: CHAIN_ID.to_string(),
                    height: proposal.height,
                    time: proposal.time,
                    last_block_id: None,
                    last_commit_hash: vec![],
                    data_hash: vec![0u8; 32],
                    validators_hash: proposal.quorum_hash.clone(),
                    next_validators_hash: proposal.quorum_hash.clone(),
                    consensus_hash: vec![0u8; 32],
                    next_consensus_hash: vec![0u8; 32],
                    app_hash,
                    results_hash: vec![0u8; 32],
                    evidence_hash: vec![],
                    proposed_app_version: proposal.proposed_app_version,
                    proposer_pro_tx_hash: proposal.proposer_pro_tx_hash.clone(),
                    core_chain_locked_height: proposal.core_chain_locked_height,
                }),
                data: Some(Data {
                    txs: proposal.txs.clone(),
                }),
                evidence: Some(EvidenceList { evidence: vec![] }),
                last_commit: None,
                core_chain_lock: proposal.core_chain_lock_update.clone(),
            }),
            block_id: Some(BlockId {
                hash: proposal.hash.clone(),
                part_set_header: Some(PartSetHeader {
                    total: 0,
                    hash: vec![0u8; 32],
                }),
                state_id: vec![0u8; 32],
            }),
        }
    }

    /// Processes `proposal`, which this node must accept, and returns its app hash
    fn accept(outcome: &ChainExecutionOutcome, proposal: &RequestProcessProposal) -> Vec<u8> {
        let response = outcome
            .abci_app
            .process_proposal(proposal.clone())
            .expect("expected to process the proposal");
        assert_eq!(response.status, ProposalStatus::Accept as i32);
        response.app_hash
    }

    /// The V24 record in the transaction of the block being executed
    fn uncommitted_v24(outcome: &ChainExecutionOutcome) -> Option<u32> {
        let transaction = outcome
            .abci_app
            .transaction
            .read()
            .expect("expected the block transaction");
        outcome
            .abci_app
            .platform
            .drive
            .fetch_core_fork_activation_height(
                CoreFork::V24,
                transaction.as_ref(),
                PlatformVersion::latest(),
            )
            .expect("expected to read the V24 record")
    }

    /// A round of a height runs in the transaction left by the last commit, or by init chain
    /// for height 1, and a round that is thrown away takes its record with it: the committed
    /// record is the committed round's. Round 0 carries a new chain locked height, 3, where V24
    /// is active, and records it, or fails on a Core error. Round 1 is committed, at the last
    /// committed height (no question to Core), at 3 again, or at 4. For height 1, a record
    /// written at init chain survives every round.
    #[stack_size(4 * 1024 * 1024)]
    #[test]
    async fn should_commit_only_the_v24_record_of_the_round_that_is_committed() {
        struct Case {
            blocks_before: u64,
            v24_activation_height: u32,
            round_0_core_down: bool,
            round_1_core_height: u32,
            after_round_0: Option<u32>,
            committed: Option<u32>,
            coinbase_reads: Vec<u32>,
        }
        let cases = [
            // Recorded at init chain, at the genesis core height.
            Case {
                blocks_before: 0,
                v24_activation_height: 1,
                round_0_core_down: false,
                round_1_core_height: 1,
                after_round_0: Some(1),
                committed: Some(1),
                coinbase_reads: vec![1],
            },
            Case {
                blocks_before: 0,
                v24_activation_height: 3,
                round_0_core_down: false,
                round_1_core_height: 1,
                after_round_0: Some(3),
                committed: None,
                coinbase_reads: vec![1, 3],
            },
            Case {
                blocks_before: 0,
                v24_activation_height: 3,
                round_0_core_down: false,
                round_1_core_height: 4,
                after_round_0: Some(3),
                committed: Some(4),
                coinbase_reads: vec![1, 3, 4],
            },
            Case {
                blocks_before: 2,
                v24_activation_height: 3,
                round_0_core_down: false,
                round_1_core_height: 1,
                after_round_0: Some(3),
                committed: None,
                coinbase_reads: vec![1, 3],
            },
            Case {
                blocks_before: 2,
                v24_activation_height: 3,
                round_0_core_down: false,
                round_1_core_height: 4,
                after_round_0: Some(3),
                committed: Some(4),
                coinbase_reads: vec![1, 3, 4],
            },
            // Round 1 asks Core again rather than reuse round 0's answer.
            Case {
                blocks_before: 2,
                v24_activation_height: 3,
                round_0_core_down: false,
                round_1_core_height: 3,
                after_round_0: Some(3),
                committed: Some(3),
                coinbase_reads: vec![1, 3, 3],
            },
            // Round 0 fails on the Core error and writes nothing.
            Case {
                blocks_before: 2,
                v24_activation_height: 3,
                round_0_core_down: true,
                round_1_core_height: 3,
                after_round_0: None,
                committed: Some(3),
                coinbase_reads: vec![1, 3, 3],
            },
        ];
        for case in cases {
            let core = Core::new(case.v24_activation_height);
            let mut platform =
                platform_with_core(PlatformVersion::latest().protocol_version, &core);
            let outcome = run_chain_for_strategy(
                &mut platform,
                case.blocks_before,
                NetworkStrategy {
                    // Without a block, the runner stops after init chain, whose transaction is
                    // not committed yet, instead of reading the committed state.
                    failure_testing: (case.blocks_before == 0).then(|| FailureStrategy {
                        dont_finalize_block: true,
                        ..Default::default()
                    }),
                    ..strategy(None)
                },
                config(),
                15,
                &mut None,
                &mut None,
            )
            .await;
            let state = outcome.abci_app.platform.state.load();
            assert_eq!(
                state.last_committed_block_height(),
                case.blocks_before,
                "the committed blocks"
            );
            assert_eq!(state.last_committed_core_height(), 1);
            drop(state);
            let label = format!(
                "after {} blocks, round 0 Core down {}, round 1 at core height {}",
                case.blocks_before, case.round_0_core_down, case.round_1_core_height
            );

            let round_0 = proposal(&outcome, 0, 3);
            core.coinbase_down
                .store(case.round_0_core_down, Ordering::SeqCst);
            let round_0_result = outcome.abci_app.process_proposal(round_0);
            core.coinbase_down.store(false, Ordering::SeqCst);
            if case.round_0_core_down {
                assert!(round_0_result.is_err(), "{label}: round 0 must fail");
            } else {
                let response = round_0_result.expect("expected to process round 0");
                assert_eq!(response.status, ProposalStatus::Accept as i32, "{label}");
            }
            assert_eq!(uncommitted_v24(&outcome), case.after_round_0, "{label}");

            let round_1 = proposal(&outcome, 1, case.round_1_core_height);
            let app_hash = accept(&outcome, &round_1);
            outcome
                .abci_app
                .finalize_block(finalize_request(&round_1, app_hash))
                .expect("expected to finalize round 1");
            assert_eq!(
                committed_v24(outcome.abci_app.platform),
                case.committed,
                "{label}"
            );
            assert_eq!(core.coinbase_reads(), case.coinbase_reads, "{label}");
        }
    }
}
