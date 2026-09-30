//! Vote extensions of different rounds of one height.
//!
//! When a new chain lock arrives between two rounds, the later proposal asks validators to sign
//! different withdrawal transactions. Precommits of the earlier round are still valid and can
//! still arrive after this node processed the later proposal, so each vote must be verified
//! against the block it is for. A vote for a block this node has not accepted is rejected.
//!
//! A block accepted in one round can also still be committed after this node refused a later
//! round's proposal, without Tenderdash processing it again, and it must be finalized as that
//! block.
#[cfg(test)]
mod tests {
    use crate::execution::run_chain_for_strategy;
    use crate::strategy::{ChainExecutionOutcome, NetworkStrategy};
    use dpp::block::block_info::BlockInfo;
    use dpp::block::extended_block_info::v0::ExtendedBlockInfoV0Setters;
    use dpp::dashcore::hashes::Hash;
    use drive_abci::config::{PlatformConfig, PlatformTestConfig};
    use drive_abci::execution::types::block_execution_context::v0::BlockExecutionContextV0Getters;
    use drive_abci::execution::types::block_state_info::v0::BlockStateInfoV0Getters;
    use drive_abci::mimic::CHAIN_ID;
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
        CommitInfo, ExtendVoteExtension, RequestExtendVote, RequestFinalizeBlock,
        RequestProcessProposal, RequestVerifyVoteExtension,
    };
    use tenderdash_abci::proto::google::protobuf::Timestamp;
    use tenderdash_abci::proto::types::{
        Block, BlockId, Data, EvidenceList, Header, PartSetHeader,
    };
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

    /// Runs the chain and queues one withdrawal transaction for the next height. Returns the
    /// outcome with the round 0 proposal for that height, at the last chain-locked core height.
    async fn run_chain_with_queued_withdrawal(
        platform: &mut Platform<MockCoreRPCLike>,
    ) -> (ChainExecutionOutcome<'_>, RequestProcessProposal) {
        let outcome = run_chain(platform).await;

        queue_withdrawal_transaction(&outcome);

        let round_0_core_height = outcome
            .abci_app
            .platform
            .state
            .load()
            .last_committed_core_height();
        let round_0 = proposal(&outcome, 0, round_0_core_height, ROUND_0_BLOCK);

        (outcome, round_0)
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

    /// The vote extensions this node signs when it precommits `proposal`
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

    /// The request Tenderdash finalizes `proposal` with once it is committed in its own round,
    /// with `app_hash` in the block header. The commit carries no withdrawal signatures, so the
    /// block must not have any withdrawal transactions to sign.
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

    /// Whether this node signs a precommit for `proposal`
    fn signs(outcome: &ChainExecutionOutcome, proposal: &RequestProcessProposal) -> bool {
        outcome
            .abci_app
            .extend_vote(RequestExtendVote {
                hash: proposal.hash.clone(),
                height: proposal.height,
                round: proposal.round,
            })
            .is_ok()
    }

    /// Processes `proposal`, which this node must accept, and returns the vote extensions it
    /// signs when it precommits it
    fn accept(
        outcome: &ChainExecutionOutcome,
        proposal: &RequestProcessProposal,
    ) -> Vec<ExtendVoteExtension> {
        let response = outcome
            .abci_app
            .process_proposal(proposal.clone())
            .expect("expected to process the proposal");
        assert_eq!(response.status, ProposalStatus::Accept as i32);

        extend_vote(outcome, proposal)
    }

    /// Processes `proposal`, which this node must reject
    fn reject(outcome: &ChainExecutionOutcome, proposal: &RequestProcessProposal) {
        let response = outcome
            .abci_app
            .process_proposal(proposal.clone())
            .expect("expected to process the proposal");
        assert_eq!(response.status, ProposalStatus::Reject as i32);
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
        let (outcome, round_0) = run_chain_with_queued_withdrawal(&mut platform).await;
        let round_1 = proposal(
            &outcome,
            1,
            round_0.core_chain_locked_height + 1,
            ROUND_1_BLOCK,
        );

        let round_0_extensions = accept(&outcome, &round_0);

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

        let round_1_extensions = accept(&outcome, &round_1);

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
        let (outcome, round_0) = run_chain_with_queued_withdrawal(&mut platform).await;
        let mut rejected_round_1 = proposal(
            &outcome,
            1,
            round_0.core_chain_locked_height + 1,
            ROUND_1_BLOCK,
        );
        // Bytes that decode to no state transition make the proposal unacceptable
        rejected_round_1.txs = vec![vec![0u8; 10]];

        let round_0_extensions = accept(&outcome, &round_0);
        reject(&outcome, &rejected_round_1);

        // The rejected proposal was executed, and its context replaced the accepted round's
        let rejected_extensions: Vec<ExtendVoteExtension> = outcome
            .abci_app
            .block_execution_context
            .read()
            .unwrap()
            .as_ref()
            .expect("the rejected proposal left its block execution context")
            .unsigned_withdrawal_transactions()
            .into();

        assert_eq!(
            rejected_extensions.len(),
            1,
            "test premise: the rejected proposal built a withdrawal transaction"
        );
        assert!(
            !signs(&outcome, &rejected_round_1),
            "a proposal this node rejected must not be signed"
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
        let (outcome, round_0) = run_chain_with_queued_withdrawal(&mut platform).await;
        let mut rejected_round_1 = proposal(
            &outcome,
            1,
            round_0.core_chain_locked_height + 1,
            ROUND_1_BLOCK,
        );
        // A protocol version this node does not run is refused before the block is executed
        rejected_round_1.version = Some(Consensus {
            block: 0,
            app: PlatformVersion::latest().protocol_version as u64 + 1,
        });

        let round_0_extensions = accept(&outcome, &round_0);
        reject(&outcome, &rejected_round_1);

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
            !signs(&outcome, &rejected_round_1),
            "the rejected block must not be signed"
        );
    }

    /// How a round 1 proposal is made unacceptable
    #[derive(Clone, Copy, Debug)]
    enum Refusal {
        /// A protocol version this node does not run, refused before the block is executed
        BeforeExecution,
        /// Bytes that decode to no state transition, refused once the block was executed
        AfterExecution,
    }

    impl Refusal {
        fn apply(self, proposal: &mut RequestProcessProposal) {
            match self {
                Refusal::BeforeExecution => {
                    proposal.version = Some(Consensus {
                        block: 0,
                        app: PlatformVersion::latest().protocol_version as u64 + 1,
                    })
                }
                Refusal::AfterExecution => proposal.txs = vec![vec![0u8; 10]],
            }
        }
    }

    /// The round of the block execution context this node holds, if it holds one
    fn block_execution_context_round(outcome: &ChainExecutionOutcome) -> Option<u32> {
        outcome
            .abci_app
            .block_execution_context
            .read()
            .unwrap()
            .as_ref()
            .map(|block_execution_context| block_execution_context.block_state_info().round())
    }

    /// The committed root hash of Drive
    fn committed_root_hash(outcome: &ChainExecutionOutcome) -> [u8; 32] {
        outcome
            .abci_app
            .platform
            .drive
            .grove
            .root_hash(None, &PlatformVersion::latest().drive.grove_version)
            .unwrap()
            .expect("expected the committed root hash")
    }

    /// Accepts the round 0 proposal of the next height and refuses the round 1 proposal made
    /// unacceptable by `refusal`, returning the round 0 proposal and the app hash it was accepted
    /// with.
    fn accept_round_0_and_refuse_round_1(
        outcome: &ChainExecutionOutcome,
        refusal: Refusal,
    ) -> (RequestProcessProposal, Vec<u8>) {
        let core_height = outcome
            .abci_app
            .platform
            .state
            .load()
            .last_committed_core_height();
        let round_0 = proposal(outcome, 0, core_height, ROUND_0_BLOCK);
        let mut refused_round_1 = proposal(outcome, 1, core_height, ROUND_1_BLOCK);
        refusal.apply(&mut refused_round_1);

        let response = outcome
            .abci_app
            .process_proposal(round_0.clone())
            .expect("expected to process the round 0 proposal");
        assert_eq!(response.status, ProposalStatus::Accept as i32);
        let round_0_app_hash = response.app_hash;

        reject(outcome, &refused_round_1);

        let expected_context_round = match refusal {
            Refusal::BeforeExecution => None,
            Refusal::AfterExecution => Some(1),
        };
        assert_eq!(
            block_execution_context_round(outcome),
            expected_context_round,
            "test premise: the {refusal:?} refusal left the round 0 block without its block \
             execution context"
        );

        (round_0, round_0_app_hash)
    }

    /// Tenderdash keeps the round state of the block it accepted when this node refuses a later
    /// round's proposal, so it commits that block without asking this node to process it again.
    /// The refused proposal has meanwhile replaced the accepted block's transaction and dropped
    /// or replaced its block execution context, and the block must still be committed with the
    /// app hash it was accepted with.
    async fn should_finalize_the_round_0_block_after_round_1_is_refused(refusal: Refusal) {
        let mut platform = TestPlatformBuilder::new()
            .with_config(config())
            .build_with_mock_rpc();
        let outcome = run_chain(&mut platform).await;

        let (round_0, round_0_app_hash) = accept_round_0_and_refuse_round_1(&outcome, refusal);

        outcome
            .abci_app
            .finalize_block(finalize_request(&round_0, round_0_app_hash.clone()))
            .expect("expected to finalize the round 0 block");

        let platform_state = outcome.abci_app.platform.state.load();
        assert_eq!(
            platform_state.last_committed_block_height(),
            round_0.height as u64
        );
        assert_eq!(
            platform_state
                .last_committed_block_app_hash()
                .map(Vec::from),
            Some(round_0_app_hash.clone())
        );
        assert_eq!(
            Vec::from(committed_root_hash(&outcome)),
            round_0_app_hash,
            "the committed state must be the one the round 0 block was accepted with"
        );
    }

    #[tokio::test]
    async fn should_finalize_a_block_accepted_in_an_earlier_round_after_a_later_proposal_was_refused_before_execution(
    ) {
        should_finalize_the_round_0_block_after_round_1_is_refused(Refusal::BeforeExecution).await;
    }

    #[tokio::test]
    async fn should_finalize_a_block_accepted_in_an_earlier_round_after_a_later_proposal_was_refused_after_execution(
    ) {
        should_finalize_the_round_0_block_after_round_1_is_refused(Refusal::AfterExecution).await;
    }

    /// Finalizing a block this node no longer holds the execution of does not trust the app
    /// hash in its header: when the block's execution gives a different one, nothing is
    /// committed.
    #[tokio::test]
    async fn should_not_finalize_a_block_whose_header_app_hash_differs_from_its_execution() {
        let mut platform = TestPlatformBuilder::new()
            .with_config(config())
            .build_with_mock_rpc();
        let outcome = run_chain(&mut platform).await;

        let committed_height = outcome
            .abci_app
            .platform
            .state
            .load()
            .last_committed_block_height();
        let root_hash_before = committed_root_hash(&outcome);

        let (round_0, round_0_app_hash) =
            accept_round_0_and_refuse_round_1(&outcome, Refusal::AfterExecution);
        let wrong_app_hash = vec![0xFF; 32];
        assert_ne!(round_0_app_hash, wrong_app_hash);

        let result = outcome
            .abci_app
            .finalize_block(finalize_request(&round_0, wrong_app_hash));
        assert!(
            result.is_err(),
            "a block whose execution gives another app hash must not be finalized"
        );

        assert_eq!(
            outcome
                .abci_app
                .platform
                .state
                .load()
                .last_committed_block_height(),
            committed_height
        );
        assert_eq!(committed_root_hash(&outcome), root_hash_before);
    }
}
