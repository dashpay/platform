use crate::abci::app::{BlockExecutionApplication, PlatformApplication, TransactionalApplication};
use crate::abci::AbciError;
use crate::error::Error;
use crate::execution::engine::consensus_params_update::consensus_params_update;
use crate::execution::types::block_execution_context::v0::{
    BlockExecutionContextV0Getters, BlockExecutionContextV0Setters,
};
use crate::platform_types::block_execution_outcome;
use crate::platform_types::block_proposal::v0::BlockProposal;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult;
use crate::rpc::core::CoreRPCLike;
use dpp::dashcore::hashes::Hash;
use dpp::dashcore::Network;
use drive::grovedb_storage::Error::RocksDBError;
use platform_version::TryIntoPlatformVersioned;
use tenderdash_abci::proto::abci as proto;
use tenderdash_abci::proto::abci::tx_record::TxAction;
use tenderdash_abci::proto::abci::{ExecTxResult, TxRecord};
use tenderdash_abci::proto::types::CoreChainLock;

pub fn prepare_proposal<'a, A, C>(
    app: &A,
    mut request: proto::RequestPrepareProposal,
) -> Result<proto::ResponsePrepareProposal, Error>
where
    A: PlatformApplication<C> + TransactionalApplication<'a> + BlockExecutionApplication,
    C: CoreRPCLike,
{
    let timer = crate::metrics::abci_request_duration("prepare_proposal");

    // We should get the latest CoreChainLock from core
    // It is possible that we will not get a chain lock from core, in this case, just don't
    // propose one
    // This is done before all else

    let platform_state = app.platform().state.load();

    // Verify that Platform State corresponds to Drive commited state
    let platform_state_app_hash = platform_state
        .last_committed_block_app_hash()
        .unwrap_or_default();

    let grove_version = &platform_state
        .current_platform_version()?
        .drive
        .grove_version;

    let drive_storage_root_hash = app
        .platform()
        .drive
        .grove
        .root_hash(None, grove_version)
        .unwrap()?;

    // We had a sequence of errors on the mainnet started since block 32326.
    // We got RocksDB's "transaction is busy" error because of a bug (https://github.com/dashpay/platform/pull/2309).
    // Due to another bug in Tenderdash (https://github.com/dashpay/tenderdash/pull/966),
    // validators just proceeded to the next block partially committing the state and updating the cache.
    // Full nodes are stuck and proceeded after re-sync.
    // For the mainnet chain, we enable these fixes at the block when we consider the state is consistent.
    let config = &app.platform().config;

    #[allow(clippy::collapsible_if)]
    if !(config.network == Network::Dash
        && config.abci.chain_id == "evo1"
        && request.height < 33000)
    {
        // App hash in memory must be equal to app hash on disk
        if drive_storage_root_hash != platform_state_app_hash {
            // We panic because we can't recover from this situation.
            // Better to restart the Drive, so we might self-heal the node
            // reloading state form the disk
            panic!(
                "drive and platform state app hash mismatch (prepare proposal): drive_storage_root_hash: {:?}, platform_state_app_hash: {:?}",
                drive_storage_root_hash, platform_state_app_hash
            );
        }
    }

    let last_committed_core_height = platform_state.last_committed_core_height();

    let starting_platform_version = platform_state.current_platform_version()?;

    let core_chain_lock_update = match app.platform().core_rpc.get_best_chain_lock() {
        Ok(latest_chain_lock) => {
            if platform_state.last_committed_block_info().is_none()
                || latest_chain_lock.block_height > last_committed_core_height
            {
                Some(latest_chain_lock)
            } else {
                None
            }
        }
        Err(_) => None,
    };

    // Filter out transactions exceeding max_block_size
    let mut transactions_exceeding_max_block_size = Vec::new();
    {
        let mut total_transactions_size = 0;
        let mut index_to_remove_at = None;
        for (i, raw_transaction) in request.txs.iter().enumerate() {
            total_transactions_size += raw_transaction.len();

            if total_transactions_size as i64 > request.max_tx_bytes {
                index_to_remove_at = Some(i);
                break;
            }
        }

        if let Some(index_to_remove_at) = index_to_remove_at {
            transactions_exceeding_max_block_size.extend(request.txs.drain(index_to_remove_at..));
        }
    }

    let mut block_proposal: BlockProposal = (&request).try_into()?;

    if let Some(core_chain_lock_update) = core_chain_lock_update.as_ref() {
        // We can't add this, as it slows down CI way too much
        // todo: find a way to re-enable this without destroying CI
        tracing::debug!(
            "propose chain lock update to height {} at block {}",
            core_chain_lock_update.block_height,
            request.height
        );
        block_proposal.core_chain_locked_height = core_chain_lock_update.block_height;
    } else {
        block_proposal.core_chain_locked_height = last_committed_core_height;
    }

    // Prepare transaction
    let transaction_guard = if request.height == app.platform().config.abci.genesis_height as i64 {
        // special logic on init chain
        let transaction_guard = app.transaction().read().unwrap();
        if transaction_guard.is_none() {
            Err(Error::Abci(AbciError::BadRequest("received a prepare proposal request for the genesis height before an init chain request".to_string())))?;
        };
        tracing::debug!(
            "rolling back to savepoint to process genesis proposal for round: {}",
            request.round,
        );
        if let Some(tx) = transaction_guard.as_ref() {
            tx.rollback_to_savepoint()
                .map_err(|e| drive::grovedb::error::Error::StorageError(RocksDBError(e)))?;
            tx.set_savepoint();
        }
        transaction_guard
    } else {
        app.start_transaction();
        app.transaction().read().unwrap()
    };

    let transaction = transaction_guard
        .as_ref()
        .expect("transaction must be started");

    // Running the proposal executes all the state transitions for the block
    let mut run_result = app.platform().run_block_proposal(
        block_proposal,
        true,
        &platform_state,
        transaction,
        Some(&timer),
    )?;

    if !run_result.is_valid() {
        // This is a system error, because we are proposing
        return Err(run_result.errors.remove(0));
    }

    let block_execution_outcome::v0::BlockExecutionOutcome {
        app_hash,
        state_transitions_result,
        validator_set_update,
        platform_version,
        mut block_execution_context,
    } = run_result.into_data().map_err(Error::Protocol)?;

    let epoch_info = block_execution_context.epoch_info();

    // We need to let Tenderdash know about the transactions we should remove from execution
    let valid_tx_count = state_transitions_result.valid_count();
    let failed_tx_count = state_transitions_result.failed_count();
    let delayed_tx_count = transactions_exceeding_max_block_size.len();
    let invalid_paid_tx_count = state_transitions_result.invalid_paid_count();
    let invalid_unpaid_tx_count = state_transitions_result.invalid_unpaid_count();

    let storage_fees = state_transitions_result.aggregated_fees().storage_fee;
    let processing_fees = state_transitions_result.aggregated_fees().processing_fee;

    let mut tx_results = Vec::new();
    let mut tx_records = Vec::new();

    for (state_transition_execution_result, raw_state_transition) in state_transitions_result
        .into_execution_results()
        .into_iter()
        .zip(request.txs)
    {
        let tx_action = match &state_transition_execution_result {
            StateTransitionExecutionResult::SuccessfulExecution(..) => TxAction::Unmodified,
            // We have identity to pay for the state transition, so we keep it in the block
            StateTransitionExecutionResult::PaidConsensusError(..) => TxAction::Unmodified,
            // We don't have any associated identity to pay for the state transition,
            // so we remove it from the block to prevent spam attacks.
            // Such state transitions must be invalidated by check tx, but they might
            // still be added to mempool due to inconsistency between check tx and tx processing
            // (fees calculation) or malicious proposer.
            StateTransitionExecutionResult::UnpaidConsensusError(consensus_error) => {
                tracing::trace!(
                    "UnpaidConsensusError at height {}, round {}: {:?}",
                    request.height,
                    request.round,
                    consensus_error
                );
                TxAction::Removed
            }
            // We shouldn't include in the block any state transitions that produced an internal error
            // during execution
            StateTransitionExecutionResult::InternalError(error_message) => {
                tracing::debug!(
                    "InternalError at height {}, round {}: {}",
                    request.height,
                    request.round,
                    error_message
                );
                TxAction::Removed
            }
            // State Transition was not executed as it reached the maximum time limit
            StateTransitionExecutionResult::NotExecuted(..) => TxAction::Delayed,
        };

        let tx_result: Option<ExecTxResult> =
            state_transition_execution_result.try_into_platform_versioned(platform_version)?;

        if let Some(result) = tx_result {
            if tx_action != TxAction::Removed {
                tx_results.push(result);
            }
        }

        tx_records.push(TxRecord {
            action: tx_action.into(),
            tx: raw_state_transition,
        });
    }

    // Add up exceeding transactions to the response
    tx_records.extend(
        transactions_exceeding_max_block_size
            .into_iter()
            .map(|tx| TxRecord {
                action: TxAction::Delayed as i32,
                tx,
            }),
    );

    let response = proto::ResponsePrepareProposal {
        tx_results,
        app_hash: app_hash.to_vec(),
        tx_records,
        core_chain_lock_update: core_chain_lock_update.map(|chain_lock| CoreChainLock {
            core_block_hash: chain_lock.block_hash.to_byte_array().to_vec(),
            core_block_height: chain_lock.block_height,
            signature: chain_lock.signature.to_bytes().to_vec(),
        }),
        validator_set_update,
        consensus_param_updates: consensus_params_update(
            app.platform().config.network,
            starting_platform_version,
            platform_version,
            epoch_info,
        )?,
        app_version: platform_version.protocol_version as u64,
    };

    block_execution_context.set_proposer_results(Some(response.clone()));

    app.block_execution_context()
        .write()
        .unwrap()
        .replace(block_execution_context);

    let elapsed_time_ms = timer.elapsed().as_millis();

    tracing::info!(
        invalid_paid_tx_count,
        invalid_unpaid_tx_count,
        valid_tx_count,
        delayed_tx_count,
        failed_tx_count,
        storage_fees,
        processing_fees,
        "Prepared proposal with {} transition{} for height: {}, round: {} in {} ms",
        valid_tx_count + invalid_paid_tx_count,
        if valid_tx_count + invalid_paid_tx_count > 0 {
            "s"
        } else {
            ""
        },
        request.height,
        request.round,
        elapsed_time_ms,
    );

    Ok(response)
}
