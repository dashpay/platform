use crate::abci::app::{PlatformApplication, TransactionalApplication};
use crate::abci::AbciError;
use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::block_execution_context::v0::BlockExecutionContextV0Getters;
use crate::execution::types::block_state_info::v0::{
    BlockStateInfoV0Getters, BlockStateInfoV0Methods,
};
use crate::rpc::core::CoreRPCLike;
use tenderdash_abci::proto::abci as proto;

pub fn extend_vote<'a, A, C>(
    app: &A,
    request: proto::RequestExtendVote,
) -> Result<proto::ResponseExtendVote, Error>
where
    A: PlatformApplication<C> + TransactionalApplication<'a>,
    C: CoreRPCLike,
{
    let _timer = crate::metrics::abci_request_duration("extend_vote");

    let proto::RequestExtendVote {
        hash: block_hash,
        height,
        round,
    } = request;
    let guarded_block_execution_context = app.platform().block_execution_context.read().unwrap();
    let block_execution_context =
        guarded_block_execution_context
            .as_ref()
            .ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                "block execution context must be set in block begin handler for extend vote",
            )))?;

    // Verify Tenderdash that it called this handler correctly
    let block_state_info = &block_execution_context.block_state_info();

    if !block_state_info.matches_current_block(height as u64, round as u32, block_hash.clone())? {
        return Err(Error::from(AbciError::RequestForWrongBlockReceived(format!(
            "received extend vote request for height: {} round: {}, block: {};  expected height: {} round: {}, block: {}",
            height, round, hex::encode(block_hash),
            block_state_info.height(), block_state_info.round(), block_state_info.block_hash().map(hex::encode).unwrap_or("None".to_string())
        )))
            .into());
    }

    // Extend vote with unsigned withdrawal transactions
    // we only want to sign the hash of the transaction
    let vote_extensions = block_execution_context
        .unsigned_withdrawal_transactions()
        .into();

    Ok(proto::ResponseExtendVote { vote_extensions })
}
