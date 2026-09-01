use crate::abci::app::{BlockExecutionApplication, PlatformApplication, TransactionalApplication};
use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::block_execution_context::v0::BlockExecutionContextV0Getters;
use crate::platform_types::cleaned_abci_messages::finalized_block_cleaned_request::v0::FinalizeBlockCleanedRequest;
use crate::platform_types::platform_state::PlatformStateV0Methods;
use crate::rpc::core::CoreRPCLike;
use dpp::dashcore::Network;
use std::sync::atomic::Ordering;
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

    let request_finalize_block: FinalizeBlockCleanedRequest = request.try_into()?;

    let block_height = request_finalize_block.height;

    let block_finalization_outcome = app.platform().finalize_block_proposal(
        request_finalize_block,
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

    let result = app.commit_transaction(platform_version);

    // We had a sequence of errors on the mainnet started since block 32326.
    // We got RocksDB's "transaction is busy" error because of a bug (https://github.com/dashpay/platform/pull/2309).
    // Due to another bug in Tenderdash (https://github.com/dashpay/tenderdash/pull/966),
    // validators just proceeded to the next block partially committing the state and updating the cache.
    // Full nodes are stuck and proceeded after re-sync.
    // For the mainnet chain, we enable these fixes at the block when we consider the state is consistent.
    let config = &app.platform().config;

    if app.platform().config.network == Network::Mainnet
        && config.abci.chain_id == "evo1"
        && block_height < 33000
    {
        // Old behavior on mainnet below block 33000
        result?;
    } else {
        // In case if transaction commit failed we still have caches in memory that
        // corresponds to the data that we weren't able to commit.
        // The simplified solution is to restart the Drive, so all caches
        // will be restored from the disk and try to process this block again.
        // TODO: We need a better handling of the transaction is busy error with retry logic.
        result.expect("commit transaction");
    }

    app.platform()
        .committed_block_height_guard
        .store(block_height, Ordering::Relaxed);

    // Reclaim the storage of any TTL'd time-range buckets the block's
    // transaction flat-dropped (grovedb#848 / PR #849): the drops' redo
    // records became visible at the commit above, and the flush turns them
    // into DB-level range tombstones. Outside consensus by design — it
    // never touches the root hash, the report is telemetry, and a failure
    // leaves the records in place for the next flush (or the startup
    // flush) to retry, so the finalized block is unaffected.
    match app
        .platform()
        .drive
        .grove
        .flush_pending_prefix_drops(&platform_version.drive.grove_version)
    {
        Ok(report) => {
            if report.reclaimed_records > 0 || report.skipped_live > 0 {
                tracing::debug!(
                    reclaimed_records = report.reclaimed_records,
                    skipped_live = report.skipped_live,
                    "flushed pending prefix drops"
                );
            }
        }
        Err(error) => {
            tracing::warn!(
                ?error,
                "failed to flush pending prefix drops; records persist and will be retried"
            );
        }
    }

    // Create GroveDB checkpoint after the transaction is committed (so it captures committed state)
    if block_finalization_outcome.checkpoint_needed {
        app.platform().create_grovedb_checkpoint(platform_version)?;
    }

    Ok(proto::ResponseFinalizeBlock { retain_height: 0 })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abci::app::{FullAbciApplication, TransactionalApplication};
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TestPlatformBuilder;

    #[test]
    fn finalize_block_fails_when_no_transaction() {
        let platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc();

        let app = FullAbciApplication::<MockCoreRPCLike>::new(&platform.platform);

        // No transaction started, no block execution context
        let request = proto::RequestFinalizeBlock {
            hash: vec![0u8; 32],
            height: 1,
            round: 0,
            ..Default::default()
        };

        let result = finalize_block::<_, MockCoreRPCLike>(&app, request);
        assert!(
            matches!(
                result,
                Err(Error::Execution(ExecutionError::NotInTransaction(_)))
            ),
            "Expected NotInTransaction error, got: {result:?}"
        );
    }

    #[test]
    fn finalize_block_fails_when_no_block_execution_context() {
        let platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc();

        let app = FullAbciApplication::<MockCoreRPCLike>::new(&platform.platform);

        // Start a transaction but do not set block execution context
        app.start_transaction();

        let request = proto::RequestFinalizeBlock {
            hash: vec![0u8; 32],
            height: 1,
            round: 0,
            ..Default::default()
        };

        let result = finalize_block::<_, MockCoreRPCLike>(&app, request);
        assert!(
            matches!(
                result,
                Err(Error::Execution(ExecutionError::CorruptedCodeExecution(_)))
            ),
            "Expected CorruptedCodeExecution error, got: {result:?}"
        );
    }
}
