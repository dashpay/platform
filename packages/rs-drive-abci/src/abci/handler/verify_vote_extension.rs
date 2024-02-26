use crate::abci::app::PlatformApplication;
use crate::error::Error;
use crate::execution::types::block_execution_context::v0::BlockExecutionContextV0Getters;
use crate::execution::types::block_state_info::v0::BlockStateInfoV0Getters;
use crate::rpc::core::CoreRPCLike;
use tenderdash_abci::proto::abci as proto;
use tenderdash_abci::proto::abci::response_verify_vote_extension::VerifyStatus;
use tenderdash_abci::proto::abci::ExtendVoteExtension;

/// Todo: Verify vote extension not really needed because extend vote is deterministic
pub fn verify_vote_extension<A, C>(
    app: &A,
    request: proto::RequestVerifyVoteExtension,
) -> Result<proto::ResponseVerifyVoteExtension, Error>
where
    A: PlatformApplication<C>,
    C: CoreRPCLike,
{
    let _timer = crate::metrics::abci_request_duration("verify_vote_extension");

    // Verify that this is a vote extension for our current executed block and our proposer
    let proto::RequestVerifyVoteExtension {
        height,
        round,
        vote_extensions,
        ..
    } = request;

    let height: u64 = height as u64;
    let round: u32 = round as u32;

    // Make sure we are in a block execution phase
    let guarded_block_execution_context = app.platform().block_execution_context.read().unwrap();
    let Some(block_execution_context) = guarded_block_execution_context.as_ref() else {
        tracing::warn!(
                "vote extensions for height: {}, round: {} are rejected because we are not in a block execution phase",
                height,
                round,
            );

        return Ok(proto::ResponseVerifyVoteExtension {
            status: VerifyStatus::Reject.into(),
        });
    };

    // Make sure vote extension is for our currently executing block

    let block_state_info = block_execution_context.block_state_info();

    // We might get vote extension to verify for previous (in case if other node is behind)
    // or future round (in case if the current node is behind), so we make sure that only height
    // is matching. It's fine because withdrawal transactions to sign are the same for any round
    // of the same height
    if block_state_info.height() != height {
        tracing::warn!(
            "vote extensions for height: {}, round: {} are rejected because we are at height: {}",
            height,
            round,
            block_state_info.height(),
        );

        return Ok(proto::ResponseVerifyVoteExtension {
            status: VerifyStatus::Reject.into(),
        });
    }

    // Verify that a validator is requesting a signatures
    // for a correct set of withdrawal transactions

    let expected_withdrawals = block_execution_context.unsigned_withdrawal_transactions();

    if expected_withdrawals != &vote_extensions {
        let expected_extensions: Vec<ExtendVoteExtension> = expected_withdrawals.into();

        tracing::error!(
            received_extensions = ?vote_extensions,
            ?expected_extensions,
            "vote extensions for height: {}, round: {} mismatch",
            height, round
        );

        return Ok(proto::ResponseVerifyVoteExtension {
            status: VerifyStatus::Reject.into(),
        });
    }

    tracing::debug!(
        "vote extensions for height: {}, round: {} are successfully verified",
        height,
        round,
    );

    Ok(proto::ResponseVerifyVoteExtension {
        status: VerifyStatus::Accept.into(),
    })
}
