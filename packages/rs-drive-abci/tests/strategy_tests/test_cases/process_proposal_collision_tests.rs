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
//! These tests assert the behaviour Drive needs: a proposal whose hash differs from the one
//! in the execution context must be executed afresh, never served from the cache and never
//! rejected with an error.
#[cfg(test)]
mod tests {
    use crate::execution::run_chain_for_strategy;
    use crate::strategy::{ChainExecutionOutcome, NetworkStrategy};
    use dpp::block::block_info::BlockInfo;
    use dpp::block::epoch::Epoch;
    use dpp::dashcore::hashes::Hash;
    use drive_abci::config::{ExecutionConfig, PlatformConfig, PlatformTestConfig};
    use drive_abci::mimic::MimicExecuteBlockOptions;
    use drive_abci::platform_types::platform_state::PlatformStateV0Methods;
    use drive_abci::test::helpers::setup::TestPlatformBuilder;
    use platform_version::version::PlatformVersion;
    use strategy_tests::{IdentityInsertInfo, StartAddresses, StartIdentities, Strategy};
    use tenderdash_abci::proto::abci::response_process_proposal::ProposalStatus;
    use tenderdash_abci::proto::abci::RequestProcessProposal;
    use tenderdash_abci::proto::google::protobuf::Timestamp;
    use tenderdash_abci::proto::version::Consensus;
    use tenderdash_abci::Application;

    const BLOCK_SPACING_MS: u64 = 3000;

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

    /// A `RequestProcessProposal` for the next block after `outcome`, attributed to the
    /// proposer at `proposer_index`, with `hash` as its block hash. `time_offset_ms` shifts
    /// the block time so that two requests built from different offsets execute to
    /// different app hashes.
    fn next_block_request(
        outcome: &ChainExecutionOutcome,
        proposer_index: usize,
        hash: [u8; 32],
        time_offset_ms: u64,
    ) -> RequestProcessProposal {
        let platform_state = outcome.abci_app.platform.state.load();
        let height = platform_state.last_committed_block_height() + 1;
        let core_height = platform_state.last_committed_core_height();
        let time_ms = outcome.end_time_ms + BLOCK_SPACING_MS + time_offset_ms;
        let proposer = outcome.proposers[proposer_index].pro_tx_hash();

        RequestProcessProposal {
            txs: vec![],
            proposed_last_commit: None,
            misbehavior: vec![],
            hash: hash.to_vec(),
            height: height as i64,
            time: Some(Timestamp {
                seconds: (time_ms / 1000) as i64,
                nanos: ((time_ms % 1000) * 1_000_000) as i32,
            }),
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
            time_ms: outcome.end_time_ms + BLOCK_SPACING_MS,
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
                    max_tx_bytes_per_block: 40000,
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
}
