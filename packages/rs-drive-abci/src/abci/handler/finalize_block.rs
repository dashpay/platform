use crate::abci::app::{BlockExecutionApplication, PlatformApplication, TransactionalApplication};
use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::block_execution_context::v0::BlockExecutionContextV0Getters;
use crate::rpc::core::CoreRPCLike;
use tenderdash_abci::proto::abci as proto;

pub fn finalize_block<'a, A, C>(
    app: &A,
    request: proto::RequestFinalizeBlock,
) -> Result<proto::ResponseFinalizeBlock, Error>
where
    A: PlatformApplication<C> + TransactionalApplication<'a> + BlockExecutionApplication,
    C: CoreRPCLike,
{
    let _timer = crate::metrics::abci_request_duration("finalize_block");

    let transaction_guard = app.transaction().read().unwrap();
    let transaction =
        transaction_guard
            .as_ref()
            .ok_or(Error::Execution(ExecutionError::NotInTransaction(
                "trying to finalize block without a current transaction",
            )))?;

    // Get current block platform version
    let block_execution_context = app
        .block_execution_context()
        .write()
        .unwrap()
        .take()
        .ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
            "block execution context must be set in block begin handler for finalize block",
        )))?;

    let platform_version = block_execution_context
        .block_platform_state()
        .current_platform_version()?;

    let block_finalization_outcome = app.platform().finalize_block_proposal(
        request.try_into()?,
        block_execution_context,
        transaction,
        platform_version,
    )?;

    drop(transaction_guard);

    //FIXME: tell tenderdash about the problem instead
    // This can not go to production!
    if !block_finalization_outcome.validation_result.is_valid() {
        return Err(Error::Abci(
            block_finalization_outcome
                .validation_result
                .errors
                .into_iter()
                .next()
                .unwrap(),
        ));
    }

    app.commit_transaction(platform_version)?;

    Ok(proto::ResponseFinalizeBlock {
        events: vec![],
        retain_height: 0,
    })
}
