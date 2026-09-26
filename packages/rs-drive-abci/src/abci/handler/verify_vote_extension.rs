use crate::abci::app::{BlockExecutionApplication, PlatformApplication};
use crate::error::Error;
use crate::platform_types::platform_state::PlatformStateV0Methods;
use crate::rpc::core::CoreRPCLike;
use tenderdash_abci::proto::abci as proto;
use tenderdash_abci::proto::abci::response_verify_vote_extension::VerifyStatus;
use tenderdash_abci::proto::abci::ExtendVoteExtension;

/// Verifies that another validator's precommit asks for signatures on the withdrawal
/// transactions this node would sign for the same block.
///
/// Tenderdash asks about every non-nil precommit of another validator at the height it is
/// deciding, whatever the round, and drops the vote when it is rejected.
pub fn verify_vote_extension<A, C>(
    app: &A,
    request: proto::RequestVerifyVoteExtension,
) -> Result<proto::ResponseVerifyVoteExtension, Error>
where
    A: PlatformApplication<C> + BlockExecutionApplication,
    C: CoreRPCLike,
{
    let _timer = crate::metrics::abci_request_duration("verify_vote_extension");

    let proto::RequestVerifyVoteExtension {
        hash,
        height,
        round,
        vote_extensions,
        ..
    } = request;

    let height: u64 = height as u64;
    let round: u32 = round as u32;

    // The height being decided is the one after our last committed block
    let platform = app.platform();
    let current_height = platform
        .state
        .load()
        .last_committed_known_block_height_or(platform.config.abci.genesis_height.saturating_sub(1))
        .saturating_add(1);

    if height != current_height {
        tracing::warn!(
            "votes extensions for height: {}, round: {} are rejected because we are at height: {}",
            height,
            round,
            current_height,
        );

        return Ok(proto::ResponseVerifyVoteExtension {
            status: VerifyStatus::Reject.into(),
        });
    }

    // Each round of a height has its own proposal, and its withdrawal transactions carry that
    // proposal's chain-locked core height as their request height. A later round whose proposer
    // saw a newer chain lock asks validators to sign different transactions, so a vote is
    // compared with what we built for the block it is for, never with another round's.
    let withdrawals_by_round = app.unsigned_withdrawal_txs_by_round().read().unwrap();

    let Some(expected_withdrawals) = withdrawals_by_round.get(height, round, &hash) else {
        // We have not accepted the block this vote is for: its proposal has not reached us yet,
        // we skipped its round, or we are catching up. We cannot tell what the validator should
        // have signed, and the ABCI++ spec requires every correct validator's extensions to be
        // accepted, so we accept rather than drop a vote Tenderdash may need to commit.
        //
        // This cannot get a withdrawal signed that we would not have built. Tenderdash checked
        // the validator's signatures before asking. We only ever add our own signature share in
        // `extend_vote`, for the block we precommit. Tenderdash recovers a threshold signature
        // only from votes carrying identical extensions that hold the quorum's threshold of
        // voting power, and `finalize_block` matches the recovered extensions against our own
        // withdrawal transactions before broadcasting them.
        tracing::debug!(
            block_hash = hex::encode(&hash),
            "votes extensions for height: {}, round: {} are accepted without comparison because we have not accepted a proposal for that block",
            height,
            round,
        );

        return Ok(proto::ResponseVerifyVoteExtension {
            status: VerifyStatus::Accept.into(),
        });
    };

    if expected_withdrawals != vote_extensions.as_slice() {
        let expected_extensions: Vec<ExtendVoteExtension> = expected_withdrawals.into();

        tracing::error!(
            received_extensions = ?vote_extensions,
            ?expected_extensions,
            block_hash = hex::encode(&hash),
            "votes extensions for height: {}, round: {} mismatch",
            height, round
        );

        return Ok(proto::ResponseVerifyVoteExtension {
            status: VerifyStatus::Reject.into(),
        });
    }

    tracing::trace!(
        "votes extensions for height: {}, round: {} are successfully verified",
        height,
        round,
    );

    Ok(proto::ResponseVerifyVoteExtension {
        status: VerifyStatus::Accept.into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abci::app::FullAbciApplication;
    use crate::platform_types::withdrawal::unsigned_withdrawal_txs::v0::UnsignedWithdrawalTxs;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
    use crate::test::helpers::withdrawals::unsigned_withdrawal_transactions;

    const ROUND_0_BLOCK: [u8; 32] = [0xA0; 32];
    const ROUND_1_BLOCK: [u8; 32] = [0xA1; 32];
    const ROUND_0_CORE_HEIGHT: u32 = 1000;
    const ROUND_1_CORE_HEIGHT: u32 = 1001;

    fn platform() -> TempPlatform<MockCoreRPCLike> {
        TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
    }

    /// The height a platform with no committed block is deciding
    fn current_height(platform: &TempPlatform<MockCoreRPCLike>) -> u64 {
        platform.config.abci.genesis_height
    }

    fn extensions(transactions: &UnsignedWithdrawalTxs) -> Vec<ExtendVoteExtension> {
        transactions.into()
    }

    fn verify(
        app: &FullAbciApplication<MockCoreRPCLike>,
        height: u64,
        round: u32,
        block_hash: [u8; 32],
        vote_extensions: Vec<ExtendVoteExtension>,
    ) -> i32 {
        let request = proto::RequestVerifyVoteExtension {
            hash: block_hash.to_vec(),
            validator_pro_tx_hash: vec![0u8; 32],
            height: height as i64,
            round: round as i32,
            vote_extensions,
        };

        verify_vote_extension::<_, MockCoreRPCLike>(app, request)
            .expect("verification answers with a status")
            .status
    }

    /// Rounds 0 and 1 of the current height accepted, at different chain-locked core heights
    fn accept_two_rounds(app: &FullAbciApplication<MockCoreRPCLike>, height: u64) {
        let mut by_round = app.unsigned_withdrawal_txs_by_round.write().unwrap();
        by_round.insert(
            height,
            0,
            ROUND_0_BLOCK,
            unsigned_withdrawal_transactions(ROUND_0_CORE_HEIGHT),
        );
        by_round.insert(
            height,
            1,
            ROUND_1_BLOCK,
            unsigned_withdrawal_transactions(ROUND_1_CORE_HEIGHT),
        );
    }

    #[test]
    fn should_reject_a_vote_for_another_height() {
        let platform = platform();
        let app = FullAbciApplication::<MockCoreRPCLike>::new(&platform.platform);
        let height = current_height(&platform);

        assert_eq!(
            verify(&app, height + 1, 0, ROUND_0_BLOCK, vec![]),
            VerifyStatus::Reject as i32
        );
    }

    /// Round 1 was proposed at a newer chain-locked core height than round 0, and this node
    /// processed it last. A round 0 precommit carries round 0's withdrawal transactions and is
    /// valid; it used to be compared with round 1's and rejected.
    #[test]
    fn should_accept_a_vote_for_an_earlier_round_at_an_older_core_height() {
        let platform = platform();
        let app = FullAbciApplication::<MockCoreRPCLike>::new(&platform.platform);
        let height = current_height(&platform);
        accept_two_rounds(&app, height);

        let round_0_extensions = extensions(&unsigned_withdrawal_transactions(ROUND_0_CORE_HEIGHT));
        let round_1_extensions = extensions(&unsigned_withdrawal_transactions(ROUND_1_CORE_HEIGHT));
        assert_ne!(
            round_0_extensions, round_1_extensions,
            "test premise: the request height makes the two rounds' extensions differ"
        );

        assert_eq!(
            verify(&app, height, 0, ROUND_0_BLOCK, round_0_extensions),
            VerifyStatus::Accept as i32
        );
        assert_eq!(
            verify(&app, height, 1, ROUND_1_BLOCK, round_1_extensions),
            VerifyStatus::Accept as i32
        );
    }

    #[test]
    fn should_reject_a_vote_whose_withdrawals_differ_from_its_round() {
        let platform = platform();
        let app = FullAbciApplication::<MockCoreRPCLike>::new(&platform.platform);
        let height = current_height(&platform);
        accept_two_rounds(&app, height);

        // Round 1's withdrawal transactions in a round 0 vote
        assert_eq!(
            verify(
                &app,
                height,
                0,
                ROUND_0_BLOCK,
                extensions(&unsigned_withdrawal_transactions(ROUND_1_CORE_HEIGHT)),
            ),
            VerifyStatus::Reject as i32
        );

        // Bytes nobody built
        assert_eq!(
            verify(
                &app,
                height,
                0,
                ROUND_0_BLOCK,
                vec![ExtendVoteExtension {
                    r#type: 0,
                    extension: vec![1, 2, 3],
                    sign_request_id: None,
                }],
            ),
            VerifyStatus::Reject as i32
        );

        // No withdrawals at all
        assert_eq!(
            verify(&app, height, 0, ROUND_0_BLOCK, vec![]),
            VerifyStatus::Reject as i32
        );
    }

    #[test]
    fn should_accept_matching_empty_withdrawals() {
        let platform = platform();
        let app = FullAbciApplication::<MockCoreRPCLike>::new(&platform.platform);
        let height = current_height(&platform);
        app.unsigned_withdrawal_txs_by_round
            .write()
            .unwrap()
            .insert(height, 0, ROUND_0_BLOCK, UnsignedWithdrawalTxs::default());

        assert_eq!(
            verify(&app, height, 0, ROUND_0_BLOCK, vec![]),
            VerifyStatus::Accept as i32
        );
        assert_eq!(
            verify(
                &app,
                height,
                0,
                ROUND_0_BLOCK,
                extensions(&unsigned_withdrawal_transactions(ROUND_0_CORE_HEIGHT)),
            ),
            VerifyStatus::Reject as i32
        );
    }

    /// A vote for a round whose proposal this node has not accepted cannot be compared with
    /// anything, so it is accepted.
    #[test]
    fn should_accept_a_vote_for_a_round_this_node_has_not_processed() {
        let platform = platform();
        let app = FullAbciApplication::<MockCoreRPCLike>::new(&platform.platform);
        let height = current_height(&platform);
        app.unsigned_withdrawal_txs_by_round
            .write()
            .unwrap()
            .insert(
                height,
                0,
                ROUND_0_BLOCK,
                unsigned_withdrawal_transactions(ROUND_0_CORE_HEIGHT),
            );

        assert_eq!(
            verify(
                &app,
                height,
                1,
                ROUND_1_BLOCK,
                extensions(&unsigned_withdrawal_transactions(ROUND_1_CORE_HEIGHT)),
            ),
            VerifyStatus::Accept as i32
        );
    }

    /// The same holds for a block other than the one this node accepted in that round.
    #[test]
    fn should_accept_a_vote_for_a_block_this_node_has_not_processed() {
        let platform = platform();
        let app = FullAbciApplication::<MockCoreRPCLike>::new(&platform.platform);
        let height = current_height(&platform);
        app.unsigned_withdrawal_txs_by_round
            .write()
            .unwrap()
            .insert(
                height,
                0,
                ROUND_0_BLOCK,
                unsigned_withdrawal_transactions(ROUND_0_CORE_HEIGHT),
            );

        assert_eq!(
            verify(
                &app,
                height,
                0,
                ROUND_1_BLOCK,
                extensions(&unsigned_withdrawal_transactions(ROUND_1_CORE_HEIGHT)),
            ),
            VerifyStatus::Accept as i32
        );
    }

    /// Before this node processes any proposal of the height, for example while the first
    /// proposal is still on its way or right after a restart.
    #[test]
    fn should_accept_a_vote_before_any_proposal_of_the_height_is_accepted() {
        let platform = platform();
        let app = FullAbciApplication::<MockCoreRPCLike>::new(&platform.platform);
        let height = current_height(&platform);

        assert_eq!(
            verify(
                &app,
                height,
                0,
                ROUND_0_BLOCK,
                extensions(&unsigned_withdrawal_transactions(ROUND_0_CORE_HEIGHT)),
            ),
            VerifyStatus::Accept as i32
        );
    }

    /// Withdrawals kept for a height that has since been committed say nothing about the next
    /// one.
    #[test]
    fn should_not_compare_with_the_withdrawals_of_another_height() {
        let platform = platform();
        let app = FullAbciApplication::<MockCoreRPCLike>::new(&platform.platform);
        let height = current_height(&platform);
        app.unsigned_withdrawal_txs_by_round
            .write()
            .unwrap()
            .insert(
                height - 1,
                0,
                ROUND_0_BLOCK,
                unsigned_withdrawal_transactions(ROUND_0_CORE_HEIGHT),
            );

        assert_eq!(
            verify(
                &app,
                height,
                0,
                ROUND_0_BLOCK,
                extensions(&unsigned_withdrawal_transactions(ROUND_1_CORE_HEIGHT)),
            ),
            VerifyStatus::Accept as i32
        );
    }
}
