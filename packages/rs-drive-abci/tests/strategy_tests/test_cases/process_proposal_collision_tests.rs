//! Two proposals for the same height and round.
//!
//! A node that hands over from block sync to consensus while still behind becomes a
//! proposer at a historical height (it is in that era's quorum) and builds a block of its own
//! there. The committed block for that height then reaches it through consensus catch-up:
//! same height, same round, a different block. Drive's `process_proposal` keeps an execution
//! context keyed only by height and round, so it answers the second block from the first
//! block's cache, or refuses it outright. Either answer wedges Tenderdash: a cached app hash
//! fails `ValidateBlockWithRoundState` (panic in `ApplyCommit`), and an ABCI error panics in
//! `mustEnsureProcess`. The WAL replays both proposals on restart, so the node never recovers.
//!
//! These tests assert the behaviour Drive needs: a proposal that is not the one the execution
//! context holds must be executed afresh, never served from the cache and never rejected with
//! an error. That has to hold in both states the context can be in — after Tenderdash has told
//! us a block hash, and in the window right after `PrepareProposal` where it has not computed
//! one yet and the prepared block is identified by its execution inputs alone.
#[cfg(test)]
mod tests {
    use crate::execution::run_chain_for_strategy;
    use crate::strategy::{ChainExecutionOutcome, NetworkStrategy};
    use dpp::block::block_info::BlockInfo;
    use dpp::block::epoch::Epoch;
    use dpp::dashcore::hashes::Hash;
    use drive_abci::config::{ExecutionConfig, PlatformConfig, PlatformTestConfig};
    use drive_abci::execution::types::block_execution_context::v0::BlockExecutionContextV0Getters;
    use drive_abci::execution::types::block_state_info::v0::BlockStateInfoV0Getters;
    use drive_abci::mimic::MimicExecuteBlockOptions;
    use drive_abci::platform_types::platform_state::PlatformStateV0Methods;
    use drive_abci::test::helpers::setup::TestPlatformBuilder;
    use platform_version::version::PlatformVersion;
    use strategy_tests::{IdentityInsertInfo, StartAddresses, StartIdentities, Strategy};
    use tenderdash_abci::proto::abci::response_process_proposal::ProposalStatus;
    use tenderdash_abci::proto::abci::tx_record::TxAction;
    use tenderdash_abci::proto::abci::{
        RequestPrepareProposal, RequestProcessProposal, ResponsePrepareProposal,
    };
    use tenderdash_abci::proto::google::protobuf::Timestamp;
    use tenderdash_abci::proto::version::Consensus;
    use tenderdash_abci::Application;

    const BLOCK_SPACING_MS: u64 = 3000;
    const MAX_TX_BYTES_PER_BLOCK: i64 = 40000;

    fn strategy() -> NetworkStrategy {
        NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
                start_addresses: StartAddresses::default(),
                identity_inserts: IdentityInsertInfo::default(),
                identity_contract_nonce_gaps: None,
                signer: None,
            },
            total_hpmns: 50,
            extra_normal_mns: 0,
            validator_quorum_count: 10,
            chain_lock_quorum_count: 10,
            upgrading_info: None,
            proposer_strategy: Default::default(),
            rotate_quorums: false,
            failure_testing: None,
            query_testing: None,
            verify_state_transition_results: false,
            ..Default::default()
        }
    }

    fn config() -> PlatformConfig {
        PlatformConfig {
            execution: ExecutionConfig {
                verify_sum_trees: true,
                ..Default::default()
            },
            block_spacing_ms: BLOCK_SPACING_MS,
            testing_configs: PlatformTestConfig::default_minimal_verifications(),
            ..Default::default()
        }
    }

    /// The time of the next block after `outcome`. `time_offset_ms` shifts it so that two
    /// proposals built from different offsets are different blocks.
    fn next_block_time_ms(outcome: &ChainExecutionOutcome, time_offset_ms: u64) -> u64 {
        outcome.end_time_ms + BLOCK_SPACING_MS + time_offset_ms
    }

    fn timestamp(time_ms: u64) -> Timestamp {
        Timestamp {
            seconds: (time_ms / 1000) as i64,
            nanos: ((time_ms % 1000) * 1_000_000) as i32,
        }
    }

    /// A `RequestProcessProposal` for the next block after `outcome`, attributed to the
    /// proposer at `proposer_index`, with `hash` as its block hash.
    fn next_block_request(
        outcome: &ChainExecutionOutcome,
        proposer_index: usize,
        hash: [u8; 32],
        time_offset_ms: u64,
    ) -> RequestProcessProposal {
        let platform_state = outcome.abci_app.platform.state.load();
        let height = platform_state.last_committed_block_height() + 1;
        let core_height = platform_state.last_committed_core_height();
        let proposer = outcome.proposers[proposer_index].pro_tx_hash();

        RequestProcessProposal {
            txs: vec![],
            proposed_last_commit: None,
            misbehavior: vec![],
            hash: hash.to_vec(),
            height: height as i64,
            time: Some(timestamp(next_block_time_ms(outcome, time_offset_ms))),
            next_validators_hash: [0u8; 32].to_vec(),
            round: 0,
            core_chain_locked_height: core_height,
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

    /// The `RequestPrepareProposal` the node's own Tenderdash sends when it is the proposer at
    /// `proposer_index` for the next block after `outcome`. Deliberately paired with
    /// `next_block_request` at the same offset and proposer index: the two then describe the
    /// same block, so a test that varies exactly one field isolates that field.
    fn next_block_prepare_request(
        outcome: &ChainExecutionOutcome,
        proposer_index: usize,
        time_offset_ms: u64,
    ) -> RequestPrepareProposal {
        let platform_state = outcome.abci_app.platform.state.load();
        let height = platform_state.last_committed_block_height() + 1;
        let core_height = platform_state.last_committed_core_height();
        let proposer = outcome.proposers[proposer_index].pro_tx_hash();

        RequestPrepareProposal {
            max_tx_bytes: MAX_TX_BYTES_PER_BLOCK,
            txs: vec![],
            local_last_commit: None,
            misbehavior: vec![],
            height: height as i64,
            time: Some(timestamp(next_block_time_ms(outcome, time_offset_ms))),
            next_validators_hash: [0u8; 32].to_vec(),
            round: 0,
            core_chain_locked_height: core_height,
            proposer_pro_tx_hash: proposer.to_byte_array().to_vec(),
            proposed_app_version: PlatformVersion::latest().protocol_version as u64,
            // Tenderdash sends 0 on prepare proposal; the app version it puts in the header
            // is the one that comes back in the response.
            version: Some(Consensus { block: 0, app: 0 }),
            quorum_hash: outcome
                .current_quorum()
                .quorum_hash
                .to_byte_array()
                .to_vec(),
        }
    }

    /// The `RequestProcessProposal` Tenderdash sends back for the block it just had prepared:
    /// the transactions the proposer kept, in order, the app version the response asked for in
    /// the header, and the block hash Tenderdash has computed by now.
    fn process_request_for_prepared_block(
        prepare_request: &RequestPrepareProposal,
        prepare_response: &ResponsePrepareProposal,
        time_ms: u64,
        hash: [u8; 32],
    ) -> RequestProcessProposal {
        let txs = prepare_response
            .tx_records
            .iter()
            .filter(|record| {
                record.action != TxAction::Removed as i32
                    && record.action != TxAction::Delayed as i32
            })
            .map(|record| record.tx.clone())
            .collect();

        RequestProcessProposal {
            txs,
            proposed_last_commit: None,
            misbehavior: vec![],
            hash: hash.to_vec(),
            height: prepare_request.height,
            time: Some(timestamp(time_ms)),
            next_validators_hash: prepare_request.next_validators_hash.clone(),
            round: prepare_request.round,
            core_chain_locked_height: prepare_response
                .core_chain_lock_update
                .as_ref()
                .map(|chain_lock| chain_lock.core_block_height)
                .unwrap_or(prepare_request.core_chain_locked_height),
            core_chain_lock_update: prepare_response.core_chain_lock_update.clone(),
            proposer_pro_tx_hash: prepare_request.proposer_pro_tx_hash.clone(),
            proposed_app_version: prepare_request.proposed_app_version,
            version: Some(Consensus {
                block: 0,
                app: prepare_response.app_version,
            }),
            quorum_hash: prepare_request.quorum_hash.clone(),
        }
    }

    /// The block time the execution context currently holds, which is the block Drive last
    /// executed. Reading it tells a re-execution apart from a cache hit without depending on
    /// the two blocks reaching different app hashes.
    fn context_block_time_ms(outcome: &ChainExecutionOutcome) -> u64 {
        outcome
            .abci_app
            .block_execution_context
            .read()
            .unwrap()
            .as_ref()
            .expect("a block execution context must be held after process proposal")
            .block_state_info()
            .block_time_ms()
    }

    fn context_block_hash(outcome: &ChainExecutionOutcome) -> Option<[u8; 32]> {
        outcome
            .abci_app
            .block_execution_context
            .read()
            .unwrap()
            .as_ref()
            .expect("a block execution context must be held after process proposal")
            .block_state_info()
            .block_hash()
    }

    /// The node prepared its own block at (H, 0) and then receives the network's block for
    /// (H, 0) — a different block. Drive must execute it and return *its* app hash, not the
    /// app hash of the block the node itself proposed.
    #[tokio::test]
    async fn process_proposal_for_a_different_block_must_not_be_served_from_the_proposer_cache() {
        let config = config();
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let outcome = run_chain_for_strategy(
            &mut platform,
            5,
            strategy(),
            config,
            7,
            &mut None,
            &mut None,
        )
        .await;

        let platform_state = outcome.abci_app.platform.state.load();
        let height = platform_state.last_committed_block_height() + 1;
        let block_info = BlockInfo {
            time_ms: next_block_time_ms(&outcome, 0),
            height,
            core_height: platform_state.last_committed_core_height(),
            epoch: Epoch::new(outcome.end_epoch_index).expect("epoch"),
        };
        drop(platform_state);

        // PrepareProposal + ProcessProposal of our own block at (H, 0), not finalized: this is
        // the state a proposer is in while it waits for votes that will never come.
        let own_block = outcome
            .abci_app
            .mimic_execute_block(
                outcome.proposers[0].pro_tx_hash().to_byte_array(),
                outcome.current_quorum(),
                PlatformVersion::latest().protocol_version,
                block_info,
                0,
                &[],
                false,
                vec![],
                MimicExecuteBlockOptions {
                    dont_finalize_block: true,
                    rounds_before_finalization: None,
                    max_tx_bytes_per_block: MAX_TX_BYTES_PER_BLOCK as u64,
                    independent_process_proposal_verification: false,
                },
            )
            .expect("our own block should prepare and process");

        // The network's block for the same height and round: different proposer, different
        // time, different hash.
        let network_block = next_block_request(&outcome, 1, [0x42u8; 32], 1000);

        let response = outcome
            .abci_app
            .process_proposal(network_block.clone())
            .expect("a different block at the same height/round must be processed, not refused");

        assert_eq!(response.status, ProposalStatus::Accept as i32);

        // What the network block really executes to: run it against a clean context.
        outcome
            .abci_app
            .block_execution_context
            .write()
            .unwrap()
            .take();
        let reference = outcome
            .abci_app
            .process_proposal(network_block)
            .expect("clean execution of the network block");

        assert_ne!(
            reference.app_hash,
            own_block.root_app_hash.to_vec(),
            "test premise: the two blocks must execute to different app hashes"
        );
        assert_eq!(
            response.app_hash, reference.app_hash,
            "Drive answered the network's block with the app hash of the node's own proposal"
        );
    }

    /// Two different blocks processed for the same height and round, neither of them ours:
    /// the second must be executed, not answered with an ABCI error (which Tenderdash turns
    /// into a panic that the WAL replay reproduces on every restart).
    #[tokio::test]
    async fn process_proposal_for_a_second_different_block_in_the_same_round_must_execute() {
        let config = config();
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let outcome = run_chain_for_strategy(
            &mut platform,
            5,
            strategy(),
            config,
            7,
            &mut None,
            &mut None,
        )
        .await;

        let first = next_block_request(&outcome, 1, [0x42u8; 32], 0);
        let second = next_block_request(&outcome, 2, [0x43u8; 32], 1000);

        let first_response = outcome
            .abci_app
            .process_proposal(first)
            .expect("first block processes");
        assert_eq!(first_response.status, ProposalStatus::Accept as i32);

        let second_response = outcome.abci_app.process_proposal(second.clone()).expect(
            "a second, different block at the same height/round must be executed, not refused",
        );
        assert_eq!(second_response.status, ProposalStatus::Accept as i32);

        outcome
            .abci_app
            .block_execution_context
            .write()
            .unwrap()
            .take();
        let reference = outcome
            .abci_app
            .process_proposal(second)
            .expect("clean execution of the second block");

        assert_ne!(first_response.app_hash, reference.app_hash);
        assert_eq!(
            second_response.app_hash, reference.app_hash,
            "Drive must answer the second block with its own app hash"
        );
    }

    /// The window right after `PrepareProposal`, before Tenderdash has computed a hash for the
    /// block it just had prepared: the execution context holds a proposer result and no block
    /// hash, so there is no hash to compare a competing block against.
    ///
    /// The competing block here matches the prepared one in everything the height-and-round
    /// cache used to look at — same transaction count (none either side) and the same absent
    /// core chain lock update — and even in block time, differing only in its proposer. That is
    /// enough to make it a different block: the proposer is written to state as the block's
    /// author and as the caster of its app version vote, so it moves the app hash. Drive must
    /// execute it rather than hand back the prepared block's app hash.
    #[tokio::test]
    async fn process_proposal_for_a_different_block_before_any_block_hash_is_known_must_not_be_served_from_the_proposer_cache(
    ) {
        let config = config();
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let outcome = run_chain_for_strategy(
            &mut platform,
            5,
            strategy(),
            config,
            7,
            &mut None,
            &mut None,
        )
        .await;

        // Our own PrepareProposal, and nothing after it: no ProcessProposal has run, so
        // nothing has told the context a block hash.
        let prepare_response = outcome
            .abci_app
            .prepare_proposal(next_block_prepare_request(&outcome, 0, 0))
            .expect("our own block should prepare");
        assert_eq!(
            context_block_hash(&outcome),
            None,
            "test premise: the context left by prepare proposal must carry no block hash"
        );

        // The competing block: same height, same round, same block time, no transactions and
        // no core chain lock update on either side. Only the proposer differs.
        let competing_block = next_block_request(&outcome, 1, [0x42u8; 32], 0);

        let response = outcome
            .abci_app
            .process_proposal(competing_block.clone())
            .expect("a different block at the same height/round must be processed, not refused");

        assert_eq!(response.status, ProposalStatus::Accept as i32);

        // What the competing block really executes to: run it against a clean context.
        outcome
            .abci_app
            .block_execution_context
            .write()
            .unwrap()
            .take();
        let reference = outcome
            .abci_app
            .process_proposal(competing_block)
            .expect("clean execution of the competing block");

        assert_ne!(
            reference.app_hash, prepare_response.app_hash,
            "test premise: the two blocks must execute to different app hashes"
        );
        assert_eq!(
            response.app_hash, reference.app_hash,
            "Drive answered a competing block with the app hash of the block it had prepared"
        );
    }

    /// The same window, with a competing block that differs from the prepared one only in its
    /// block time. Nothing in the pair reaches state through a path that a shifted time alone
    /// would redirect, so the two execute to the same app hash and the served hash proves
    /// nothing; what the context holds afterwards does. A cache hit would leave the prepared
    /// block's time in place and merely stamp the request's hash onto it, so finding the
    /// competing block's time there is what says Drive executed the block it was asked about.
    #[tokio::test]
    async fn process_proposal_for_a_block_differing_only_in_time_before_any_block_hash_is_known_must_execute(
    ) {
        let config = config();
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let outcome = run_chain_for_strategy(
            &mut platform,
            5,
            strategy(),
            config,
            7,
            &mut None,
            &mut None,
        )
        .await;

        outcome
            .abci_app
            .prepare_proposal(next_block_prepare_request(&outcome, 0, 0))
            .expect("our own block should prepare");
        assert_eq!(
            context_block_time_ms(&outcome),
            next_block_time_ms(&outcome, 0),
            "test premise: the context holds the prepared block's time"
        );

        // Same proposer, same height and round, no transactions and no core chain lock update
        // on either side: a shifted block time is the only difference.
        let competing_block = next_block_request(&outcome, 0, [0x44u8; 32], 1000);

        let response = outcome
            .abci_app
            .process_proposal(competing_block)
            .expect("a different block at the same height/round must be processed, not refused");

        assert_eq!(response.status, ProposalStatus::Accept as i32);
        assert_eq!(
            context_block_time_ms(&outcome),
            next_block_time_ms(&outcome, 1000),
            "Drive answered a block one second later than the one it prepared out of the prepared context"
        );
        assert_eq!(
            context_block_hash(&outcome),
            Some([0x44u8; 32]),
            "the context must be the competing block's own"
        );
    }

    /// The legitimate case the proposer cache exists for: Tenderdash comes back with the very
    /// block it just had prepared, now carrying a hash. Identity is established from the
    /// execution inputs, the prepared result is replayed instead of executing the block twice,
    /// and the hash Tenderdash computed is recorded on the context that extend vote and
    /// finalize block go on to use.
    #[tokio::test]
    async fn process_proposal_for_the_prepared_block_must_still_be_served_from_the_proposer_cache()
    {
        let config = config();
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let outcome = run_chain_for_strategy(
            &mut platform,
            5,
            strategy(),
            config,
            7,
            &mut None,
            &mut None,
        )
        .await;

        let prepare_request = next_block_prepare_request(&outcome, 0, 0);
        let prepare_response = outcome
            .abci_app
            .prepare_proposal(prepare_request.clone())
            .expect("our own block should prepare");

        let prepared_block = process_request_for_prepared_block(
            &prepare_request,
            &prepare_response,
            next_block_time_ms(&outcome, 0),
            [0x45u8; 32],
        );

        let response = outcome
            .abci_app
            .process_proposal(prepared_block)
            .expect("the prepared block must be processed");

        assert_eq!(response.status, ProposalStatus::Accept as i32);
        assert_eq!(
            response.app_hash, prepare_response.app_hash,
            "the block we prepared must still be answered from the proposer cache"
        );
        assert_eq!(
            context_block_hash(&outcome),
            Some([0x45u8; 32]),
            "the cached context must record the hash Tenderdash computed for it"
        );
        assert_eq!(
            context_block_time_ms(&outcome),
            next_block_time_ms(&outcome, 0),
            "the cached context must be the one prepare proposal built"
        );
    }
}
