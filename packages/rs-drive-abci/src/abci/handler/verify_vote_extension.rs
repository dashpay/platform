use crate::abci::app::{BlockExecutionApplication, PlatformApplication};
use crate::error::Error;
use crate::execution::types::block_execution_context::v0::BlockExecutionContextV0Getters;
use crate::execution::types::block_state_info::v0::BlockStateInfoV0Getters;
use crate::rpc::core::CoreRPCLike;
use tenderdash_abci::proto::abci as proto;
use tenderdash_abci::proto::abci::response_verify_vote_extension::VerifyStatus;
use tenderdash_abci::proto::abci::ExtendVoteExtension;

/// Todo: Verify votes extension not really needed because extend votes is deterministic
pub fn verify_vote_extension<A, C>(
    app: &A,
    request: proto::RequestVerifyVoteExtension,
) -> Result<proto::ResponseVerifyVoteExtension, Error>
where
    A: PlatformApplication<C> + BlockExecutionApplication,
    C: CoreRPCLike,
{
    let _timer = crate::metrics::abci_request_duration("verify_vote_extension");

    // Verify that this is a votes extension for our current executed block and our proposer
    let proto::RequestVerifyVoteExtension {
        height,
        round,
        vote_extensions,
        ..
    } = request;

    let height: u64 = height as u64;
    let round: u32 = round as u32;

    // Make sure we are in a block execution phase
    let block_execution_context_ref = app.block_execution_context().read().unwrap();
    let Some(block_execution_context) = block_execution_context_ref.as_ref() else {
        tracing::warn!(
                "votes extensions for height: {}, round: {} are rejected because we are not in a block execution phase",
                height,
                round,
            );

        return Ok(proto::ResponseVerifyVoteExtension {
            status: VerifyStatus::Reject.into(),
        });
    };

    // Make sure votes extension is for our currently executing block

    let block_state_info = block_execution_context.block_state_info();

    // We might get votes extension to verify for previous (in case if other node is behind)
    // or future round (in case if the current node is behind), so we make sure that only height
    // is matching. It's fine because withdrawal transactions to sign are the same for any round
    // of the same height
    if block_state_info.height() != height {
        tracing::warn!(
            "votes extensions for height: {}, round: {} are rejected because we are at height: {}",
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

    if expected_withdrawals != vote_extensions.as_slice() {
        let expected_extensions: Vec<ExtendVoteExtension> = expected_withdrawals.into();

        tracing::error!(
            received_extensions = ?vote_extensions,
            ?expected_extensions,
            "votes extensions for height: {}, round: {} mismatch",
            height, round
        );

        return Ok(proto::ResponseVerifyVoteExtension {
            status: VerifyStatus::Reject.into(),
        });
    }

    tracing::debug!(
        "votes extensions for height: {}, round: {} are successfully verified",
        height,
        round,
    );

    Ok(proto::ResponseVerifyVoteExtension {
        status: VerifyStatus::Accept.into(),
    })
}
