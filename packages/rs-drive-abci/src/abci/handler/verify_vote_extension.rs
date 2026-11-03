use crate::abci::app::{BlockExecutionApplication, PlatformApplication};
use crate::error::Error;
use crate::rpc::core::CoreRPCLike;
use tenderdash_abci::proto::abci as proto;
use tenderdash_abci::proto::abci::response_verify_vote_extension::VerifyStatus;

/// Verifies that another validator's precommit asks for signatures on exactly the withdrawal
/// transactions this node built for the same block.
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

    // A vote is compared with what we built for the block it is for, never with the last
    // proposal processed (see `UnsignedWithdrawalTxsByRound`).
    let withdrawals_by_round = app.unsigned_withdrawal_txs_by_round().read().unwrap();

    let Some(expected_extensions) = withdrawals_by_round.get(height, round, &hash) else {
        // We have not accepted the block this vote is for: its proposal has not reached us yet,
        // or it belongs to another height. The block signature does not cover vote extensions,
        // and ours carry a sign request id that binds them to neither height nor round, so any
        // peer can drop some or all of a precommit's extensions, or swap in the same validator's
        // extensions from another round, and the vote still verifies. Counting such votes could
        // let extensions other than the block's reach the recovery threshold, and the commit
        // they form would then fail in `finalize_block`.
        //
        // A rejected vote is sent again only while we stay in its round and a peer holds +2/3
        // precommits for one block. Otherwise this node catches up once the others commit.
        tracing::debug!(
            block_hash = hex::encode(&hash),
            "votes extensions for height: {}, round: {} are rejected because we have not accepted a proposal for that block",
            height,
            round,
        );

        return Ok(proto::ResponseVerifyVoteExtension {
            status: VerifyStatus::Reject.into(),
        });
    };

    if expected_extensions != vote_extensions.as_slice() {
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
    use tenderdash_abci::proto::abci::ExtendVoteExtension;

    const HEIGHT: u64 = 10;
    const ROUND_0_BLOCK: [u8; 32] = [0xA0; 32];
    const ROUND_1_BLOCK: [u8; 32] = [0xA1; 32];
    const ROUND_0_CORE_HEIGHT: u32 = 1000;
    const ROUND_1_CORE_HEIGHT: u32 = 1001;

    fn platform() -> TempPlatform<MockCoreRPCLike> {
        TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
    }

    fn extensions(transactions: &UnsignedWithdrawalTxs) -> Vec<ExtendVoteExtension> {
        transactions.into()
    }

    fn round_0_extensions() -> Vec<ExtendVoteExtension> {
        extensions(&unsigned_withdrawal_transactions(ROUND_0_CORE_HEIGHT))
    }

    fn round_1_extensions() -> Vec<ExtendVoteExtension> {
        extensions(&unsigned_withdrawal_transactions(ROUND_1_CORE_HEIGHT))
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

    /// Round 0 accepted at `HEIGHT`, at `ROUND_0_CORE_HEIGHT`
    fn accept_round_0(app: &FullAbciApplication<MockCoreRPCLike>) {
        app.unsigned_withdrawal_txs_by_round
            .write()
            .unwrap()
            .insert(HEIGHT, 0, ROUND_0_BLOCK, round_0_extensions());
    }

    /// Round 1 accepted at `HEIGHT`, at the newer `ROUND_1_CORE_HEIGHT`
    fn accept_round_1(app: &FullAbciApplication<MockCoreRPCLike>) {
        app.unsigned_withdrawal_txs_by_round
            .write()
            .unwrap()
            .insert(HEIGHT, 1, ROUND_1_BLOCK, round_1_extensions());
    }

    /// Round 1 was proposed at a newer chain-locked core height than round 0, and this node
    /// processed it last. A round 0 precommit carries round 0's withdrawal transactions and is
    /// valid.
    #[test]
    fn should_accept_a_vote_for_an_earlier_round_at_an_older_core_height() {
        let platform = platform();
        let app = FullAbciApplication::<MockCoreRPCLike>::new(&platform.platform);
        accept_round_0(&app);
        accept_round_1(&app);

        assert_ne!(
            round_0_extensions(),
            round_1_extensions(),
            "test premise: the request height makes the two rounds' extensions differ"
        );

        assert_eq!(
            verify(&app, HEIGHT, 0, ROUND_0_BLOCK, round_0_extensions()),
            VerifyStatus::Accept as i32
        );
        assert_eq!(
            verify(&app, HEIGHT, 1, ROUND_1_BLOCK, round_1_extensions()),
            VerifyStatus::Accept as i32
        );
    }

    #[test]
    fn should_reject_a_vote_whose_withdrawals_differ_from_its_round() {
        let platform = platform();
        let app = FullAbciApplication::<MockCoreRPCLike>::new(&platform.platform);
        accept_round_0(&app);
        accept_round_1(&app);

        // Round 1's withdrawal transactions in a round 0 vote
        assert_eq!(
            verify(&app, HEIGHT, 0, ROUND_0_BLOCK, round_1_extensions()),
            VerifyStatus::Reject as i32
        );

        // Bytes nobody built
        assert_eq!(
            verify(
                &app,
                HEIGHT,
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

        // Extensions stripped by a relaying peer, entirely or in part
        assert_eq!(
            verify(&app, HEIGHT, 0, ROUND_0_BLOCK, vec![]),
            VerifyStatus::Reject as i32
        );
        assert_eq!(
            verify(
                &app,
                HEIGHT,
                0,
                ROUND_0_BLOCK,
                round_0_extensions().into_iter().take(1).collect(),
            ),
            VerifyStatus::Reject as i32
        );

        // The right extensions in another order
        assert_eq!(
            verify(
                &app,
                HEIGHT,
                0,
                ROUND_0_BLOCK,
                round_0_extensions().into_iter().rev().collect(),
            ),
            VerifyStatus::Reject as i32
        );
    }

    #[test]
    fn should_accept_matching_empty_withdrawals() {
        let platform = platform();
        let app = FullAbciApplication::<MockCoreRPCLike>::new(&platform.platform);
        app.unsigned_withdrawal_txs_by_round
            .write()
            .unwrap()
            .insert(HEIGHT, 0, ROUND_0_BLOCK, vec![]);

        assert_eq!(
            verify(&app, HEIGHT, 0, ROUND_0_BLOCK, vec![]),
            VerifyStatus::Accept as i32
        );
        assert_eq!(
            verify(&app, HEIGHT, 0, ROUND_0_BLOCK, round_0_extensions()),
            VerifyStatus::Reject as i32
        );
    }

    /// Only round 0 is accepted. Nothing tells an honest round 1 vote from one whose extensions
    /// a relaying peer stripped or replaced with the same validator's round 0 extensions.
    #[test]
    fn should_reject_a_vote_for_a_round_this_node_has_not_accepted() {
        let platform = platform();
        let app = FullAbciApplication::<MockCoreRPCLike>::new(&platform.platform);
        accept_round_0(&app);

        for vote_extensions in [round_1_extensions(), vec![], round_0_extensions()] {
            assert_eq!(
                verify(&app, HEIGHT, 1, ROUND_1_BLOCK, vote_extensions),
                VerifyStatus::Reject as i32
            );
        }
    }

    /// The same holds for a block other than the one this node accepted in that round.
    #[test]
    fn should_reject_a_vote_for_a_block_this_node_has_not_accepted() {
        let platform = platform();
        let app = FullAbciApplication::<MockCoreRPCLike>::new(&platform.platform);
        accept_round_0(&app);

        assert_eq!(
            verify(&app, HEIGHT, 0, ROUND_1_BLOCK, round_0_extensions()),
            VerifyStatus::Reject as i32
        );
    }

    /// Before this node accepts any proposal of the height, for example while the first
    /// proposal is still on its way or right after a restart.
    #[test]
    fn should_reject_a_vote_before_any_proposal_of_the_height_is_accepted() {
        let platform = platform();
        let app = FullAbciApplication::<MockCoreRPCLike>::new(&platform.platform);

        for vote_extensions in [round_0_extensions(), vec![]] {
            assert_eq!(
                verify(&app, HEIGHT, 0, ROUND_0_BLOCK, vote_extensions),
                VerifyStatus::Reject as i32
            );
        }
    }

    /// Withdrawals kept for one height say nothing about another.
    #[test]
    fn should_reject_a_vote_for_another_height() {
        let platform = platform();
        let app = FullAbciApplication::<MockCoreRPCLike>::new(&platform.platform);
        accept_round_0(&app);

        for height in [HEIGHT - 1, HEIGHT + 1] {
            assert_eq!(
                verify(&app, height, 0, ROUND_0_BLOCK, round_0_extensions()),
                VerifyStatus::Reject as i32
            );
        }
    }

    /// A block this node accepted in round 0, re-proposed and voted for in round 1, asks for the
    /// same signatures.
    #[test]
    fn should_accept_a_vote_for_a_block_accepted_in_another_round() {
        let platform = platform();
        let app = FullAbciApplication::<MockCoreRPCLike>::new(&platform.platform);
        accept_round_0(&app);

        assert_eq!(
            verify(&app, HEIGHT, 1, ROUND_0_BLOCK, round_0_extensions()),
            VerifyStatus::Accept as i32
        );
        assert_eq!(
            verify(&app, HEIGHT, 1, ROUND_0_BLOCK, vec![]),
            VerifyStatus::Reject as i32
        );
    }

    /// An empty vote for a block this node has not accepted is rejected too, even when the block
    /// it accepted at the height has no withdrawals.
    #[test]
    fn should_reject_an_empty_vote_for_a_block_this_node_has_not_accepted() {
        let platform = platform();
        let app = FullAbciApplication::<MockCoreRPCLike>::new(&platform.platform);
        app.unsigned_withdrawal_txs_by_round
            .write()
            .unwrap()
            .insert(HEIGHT, 0, ROUND_0_BLOCK, vec![]);

        assert_eq!(
            verify(&app, HEIGHT, 1, ROUND_1_BLOCK, vec![]),
            VerifyStatus::Reject as i32
        );
    }
}
