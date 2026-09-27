//! Vote extensions of different rounds of one height.
//!
//! When a new chain lock arrives between two rounds, the later proposal asks validators to sign
//! different withdrawal transactions. Precommits of the earlier round are still valid and can
//! still arrive after this node processed the later proposal, so each vote must be verified
//! against the block it is for. A vote for a block this node has not accepted is rejected.
#[cfg(test)]
mod tests {
    use crate::execution::run_chain_for_strategy;
    use crate::strategy::{ChainExecutionOutcome, NetworkStrategy};
    use dpp::block::block_info::BlockInfo;
    use dpp::block::extended_block_info::v0::ExtendedBlockInfoV0Setters;
    use dpp::dashcore::hashes::Hash;
    use drive_abci::config::{PlatformConfig, PlatformTestConfig};
    use drive_abci::platform_types::platform::Platform;
    use drive_abci::platform_types::platform_state::PlatformStateV0Methods;
    use drive_abci::rpc::core::MockCoreRPCLike;
    use drive_abci::test::helpers::setup::TestPlatformBuilder;
    use drive_abci::test::helpers::withdrawals::untied_withdrawal_transaction_bytes;
    use platform_version::version::PlatformVersion;
    use std::sync::Arc;
    use tenderdash_abci::proto::abci::response_process_proposal::ProposalStatus;
    use tenderdash_abci::proto::abci::response_verify_vote_extension::VerifyStatus;
    use tenderdash_abci::proto::abci::{
        ExtendVoteExtension, RequestExtendVote, RequestProcessProposal, RequestVerifyVoteExtension,
    };
    use tenderdash_abci::proto::google::protobuf::Timestamp;
    use tenderdash_abci::proto::version::Consensus;
    use tenderdash_abci::proto::FromMillis;
    use tenderdash_abci::Application;

    const BLOCK_SPACING_MS: u64 = 3000;
    const ROUND_0_BLOCK: [u8; 32] = [0xA0; 32];
    const ROUND_1_BLOCK: [u8; 32] = [0xA1; 32];

    fn strategy() -> NetworkStrategy {
        NetworkStrategy {
            total_hpmns: 50,
            validator_quorum_count: 10,
            chain_lock_quorum_count: 10,
            ..Default::default()
        }
    }

    fn config() -> PlatformConfig {
        PlatformConfig {
            block_spacing_ms: BLOCK_SPACING_MS,
            testing_configs: PlatformTestConfig::default_minimal_verifications(),
            ..Default::default()
        }
    }

    async fn run_chain(platform: &mut Platform<MockCoreRPCLike>) -> ChainExecutionOutcome<'_> {
        run_chain_for_strategy(platform, 2, strategy(), config(), 15, &mut None, &mut None).await
    }

    /// Puts one untied withdrawal transaction in the queue the next height dequeues from, and
    /// moves the committed app hash with it. Only the queue entry is written: there is no
    /// matching withdrawal document, the transaction index counter is not advanced, and the
    /// amount is recorded at block time 0.
    fn queue_withdrawal_transaction(outcome: &ChainExecutionOutcome) {
        let platform = outcome.abci_app.platform;
        let platform_version = PlatformVersion::latest();

        let transaction = platform.drive.grove.start_transaction();
        let mut drive_operations = vec![];
        platform
            .drive
            .add_enqueue_untied_withdrawal_transaction_operations(
                vec![(0, untied_withdrawal_transaction_bytes(0))],
                102_000_000,
                &mut drive_operations,
                platform_version,
            )
            .expect("expected to enqueue a withdrawal transaction");
        platform
            .drive
            .apply_drive_operations(
                drive_operations,
                true,
                &BlockInfo::default(),
                Some(&transaction),
                platform_version,
                None,
            )
            .expect("expected to apply the enqueue operations");
        platform
            .drive
            .commit_transaction(transaction, &platform_version.drive)
            .expect("expected to commit the queued withdrawal transaction");

        let app_hash = platform
            .drive
            .grove
            .root_hash(None, &platform_version.drive.grove_version)
            .unwrap()
            .expect("expected the committed root hash");
        let mut platform_state = platform.state.load().as_ref().clone();
        platform_state
            .last_committed_block_info_mut()
            .as_mut()
            .expect("a block was committed")
            .set_app_hash(app_hash);
        platform.state.store(Arc::new(platform_state));
    }

    /// The proposal of `round` for the next height, at chain-locked `core_chain_locked_height`
    fn proposal(
        outcome: &ChainExecutionOutcome,
        round: u32,
        core_chain_locked_height: u32,
        hash: [u8; 32],
    ) -> RequestProcessProposal {
        let platform_state = outcome.abci_app.platform.state.load();
        let height = platform_state.last_committed_block_height() + 1;
        let time_ms = outcome.end_time_ms + BLOCK_SPACING_MS + round as u64 * 1000;
        let proposer = outcome.proposers[round as usize].pro_tx_hash();

        RequestProcessProposal {
            txs: vec![],
            proposed_last_commit: None,
            misbehavior: vec![],
            hash: hash.to_vec(),
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

    /// The vote extensions this node signs when it precommits `proposal`, which it processed last
    fn extend_vote(
        outcome: &ChainExecutionOutcome,
        proposal: &RequestProcessProposal,
    ) -> Vec<ExtendVoteExtension> {
        outcome
            .abci_app
            .extend_vote(RequestExtendVote {
                hash: proposal.hash.clone(),
                height: proposal.height,
                round: proposal.round,
            })
            .expect("expected to extend the vote")
            .vote_extensions
    }

    /// Another validator's precommit for `proposal`, carrying `vote_extensions`
    fn verify(
        outcome: &ChainExecutionOutcome,
        proposal: &RequestProcessProposal,
        vote_extensions: Vec<ExtendVoteExtension>,
    ) -> i32 {
        outcome
            .abci_app
            .verify_vote_extension(RequestVerifyVoteExtension {
                hash: proposal.hash.clone(),
                validator_pro_tx_hash: outcome.proposers[5].pro_tx_hash().to_byte_array().to_vec(),
                height: proposal.height,
                round: proposal.round,
                vote_extensions,
            })
            .expect("expected to verify the vote extensions")
            .status
    }

    /// Round 0 is processed at the last chain-locked core height, then round 1 at a newer one.
    /// A round 0 precommit is still accepted afterwards. A vote whose withdrawals differ from its
    /// own round's is rejected, and so is a vote for a round not processed yet.
    #[tokio::test]
    async fn should_verify_each_rounds_vote_extensions_against_its_own_withdrawals() {
        let mut platform = TestPlatformBuilder::new()
            .with_config(config())
            .build_with_mock_rpc();
        let outcome = run_chain(&mut platform).await;

        queue_withdrawal_transaction(&outcome);

        let round_0_core_height = outcome
            .abci_app
            .platform
            .state
            .load()
            .last_committed_core_height();
        let round_0 = proposal(&outcome, 0, round_0_core_height, ROUND_0_BLOCK);
        let round_1 = proposal(&outcome, 1, round_0_core_height + 1, ROUND_1_BLOCK);

        let response = outcome
            .abci_app
            .process_proposal(round_0.clone())
            .expect("expected to process the round 0 proposal");
        assert_eq!(response.status, ProposalStatus::Accept as i32);
        let round_0_extensions = extend_vote(&outcome, &round_0);

        // Before round 1 is processed, a round 1 precommit carrying the same validator's round 0
        // extensions still verifies in Tenderdash and matches the only proposal this node
        // processed. It must be rejected.
        assert_eq!(
            verify(&outcome, &round_1, round_0_extensions.clone()),
            VerifyStatus::Reject as i32,
            "a vote for a round this node has not accepted must be rejected"
        );
        assert_eq!(
            verify(&outcome, &round_0, vec![]),
            VerifyStatus::Reject as i32,
            "a round 0 precommit whose extensions were stripped must be rejected"
        );

        let response = outcome
            .abci_app
            .process_proposal(round_1.clone())
            .expect("expected to process the round 1 proposal");
        assert_eq!(response.status, ProposalStatus::Accept as i32);
        let round_1_extensions = extend_vote(&outcome, &round_1);

        assert_eq!(
            round_0_extensions.len(),
            1,
            "test premise: the queued withdrawal transaction is signed at this height"
        );
        assert_ne!(
            round_0_extensions, round_1_extensions,
            "test premise: the newer core height changes the transaction validators sign"
        );

        assert_eq!(
            verify(&outcome, &round_0, round_0_extensions),
            VerifyStatus::Accept as i32,
            "a round 0 precommit must be accepted after round 1 was processed"
        );
        assert_eq!(
            verify(&outcome, &round_1, round_1_extensions.clone()),
            VerifyStatus::Accept as i32,
        );
        assert_eq!(
            verify(&outcome, &round_0, round_1_extensions),
            VerifyStatus::Reject as i32,
            "a round 0 precommit carrying round 1's withdrawals must be rejected"
        );
    }

    /// A proposal this node rejects is not kept, even though its block execution context replaced
    /// the accepted round's: a vote for it is rejected, and votes for the accepted round are still
    /// verified.
    #[tokio::test]
    async fn should_not_verify_votes_against_a_rejected_proposal() {
        let mut platform = TestPlatformBuilder::new()
            .with_config(config())
            .build_with_mock_rpc();
        let outcome = run_chain(&mut platform).await;

        queue_withdrawal_transaction(&outcome);

        let round_0_core_height = outcome
            .abci_app
            .platform
            .state
            .load()
            .last_committed_core_height();
        let round_0 = proposal(&outcome, 0, round_0_core_height, ROUND_0_BLOCK);
        let mut rejected_round_1 = proposal(&outcome, 1, round_0_core_height + 1, ROUND_1_BLOCK);
        // Bytes that decode to no state transition make the proposal unacceptable
        rejected_round_1.txs = vec![vec![0u8; 10]];

        let response = outcome
            .abci_app
            .process_proposal(round_0.clone())
            .expect("expected to process the round 0 proposal");
        assert_eq!(response.status, ProposalStatus::Accept as i32);
        let round_0_extensions = extend_vote(&outcome, &round_0);

        let response = outcome
            .abci_app
            .process_proposal(rejected_round_1.clone())
            .expect("expected to process the round 1 proposal");
        assert_eq!(response.status, ProposalStatus::Reject as i32);
        let rejected_extensions = extend_vote(&outcome, &rejected_round_1);

        assert_eq!(
            rejected_extensions.len(),
            1,
            "test premise: the rejected proposal built a withdrawal transaction"
        );

        assert_eq!(
            verify(&outcome, &rejected_round_1, rejected_extensions),
            VerifyStatus::Reject as i32,
            "a vote for a rejected proposal must be rejected"
        );
        assert_eq!(
            verify(&outcome, &round_0, round_0_extensions),
            VerifyStatus::Accept as i32,
            "votes for the accepted round must still be verified"
        );
    }

    /// A round 1 proposal rejected before it is executed leaves this node without a block
    /// execution context. Tenderdash can still precommit the round 0 block it locked, in round 1
    /// and without processing it again, and it must be given that block's withdrawals.
    #[tokio::test]
    async fn should_sign_a_locked_block_after_a_later_proposal_was_rejected_before_execution() {
        let mut platform = TestPlatformBuilder::new()
            .with_config(config())
            .build_with_mock_rpc();
        let outcome = run_chain(&mut platform).await;

        queue_withdrawal_transaction(&outcome);

        let round_0_core_height = outcome
            .abci_app
            .platform
            .state
            .load()
            .last_committed_core_height();
        let round_0 = proposal(&outcome, 0, round_0_core_height, ROUND_0_BLOCK);
        let mut rejected_round_1 = proposal(&outcome, 1, round_0_core_height + 1, ROUND_1_BLOCK);
        // A protocol version this node does not run is refused before the block is executed
        rejected_round_1.version = Some(Consensus {
            block: 0,
            app: PlatformVersion::latest().protocol_version as u64 + 1,
        });

        let response = outcome
            .abci_app
            .process_proposal(round_0.clone())
            .expect("expected to process the round 0 proposal");
        assert_eq!(response.status, ProposalStatus::Accept as i32);
        let round_0_extensions = extend_vote(&outcome, &round_0);

        let response = outcome
            .abci_app
            .process_proposal(rejected_round_1.clone())
            .expect("expected to process the round 1 proposal");
        assert_eq!(response.status, ProposalStatus::Reject as i32);

        assert!(
            outcome
                .abci_app
                .block_execution_context
                .read()
                .unwrap()
                .is_none(),
            "test premise: the rejected proposal left no block execution context"
        );
        assert_eq!(
            round_0_extensions.len(),
            1,
            "test premise: the queued withdrawal transaction is signed at this height"
        );

        let round_0_relocked_in_round_1 = RequestProcessProposal {
            round: 1,
            ..round_0
        };
        assert_eq!(
            extend_vote(&outcome, &round_0_relocked_in_round_1),
            round_0_extensions,
            "the locked block must be signed with the withdrawals built when it was accepted"
        );

        assert!(
            outcome
                .abci_app
                .extend_vote(RequestExtendVote {
                    hash: rejected_round_1.hash.clone(),
                    height: rejected_round_1.height,
                    round: rejected_round_1.round,
                })
                .is_err(),
            "the rejected block must not be signed"
        );
    }
}
