use std::path::PathBuf;

use crate::abci::app::{PlatformApplication, TransactionalApplication};
use crate::abci::AbciError;
use crate::error::Error;
use crate::execution::types::block_execution_context::v0::BlockExecutionContextV0Setters;
use crate::platform_types::block_execution_outcome;
use crate::platform_types::block_proposal::v0::BlockProposal;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult;
use crate::rpc::core::CoreRPCLike;
use dpp::dashcore::hashes::Hash;
use dpp::version::PlatformVersion;
use dpp::version::TryIntoPlatformVersioned;
use tenderdash_abci::proto::abci as proto;
use tenderdash_abci::proto::abci::tx_record::TxAction;
use tenderdash_abci::proto::abci::{ExecTxResult, TxRecord};
use tenderdash_abci::proto::types::{ConsensusParams, CoreChainLock};

pub fn prepare_proposal<'a, A, C>(
    app: &A,
    mut request: proto::RequestPrepareProposal,
) -> Result<proto::ResponsePrepareProposal, Error>
where
    A: PlatformApplication<C> + TransactionalApplication<'a>,
    C: CoreRPCLike,
{
    let timer = crate::metrics::abci_request_duration("prepare_proposal");

    // We should get the latest CoreChainLock from core
    // It is possible that we will not get a chain lock from core, in this case, just don't
    // propose one
    // This is done before all else

    let state = app.platform().state.read().unwrap();

    let last_committed_core_height = state.last_committed_core_height();

    let core_chain_lock_update = match app.platform().core_rpc.get_best_chain_lock() {
        Ok(latest_chain_lock) => {
            if state.last_committed_block_info().is_none()
                || latest_chain_lock.block_height > last_committed_core_height
            {
                Some(latest_chain_lock)
            } else {
                None
            }
        }
        Err(_) => None,
    };

    drop(state);

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
        let transaction = app.transaction().read().unwrap();
        if transaction.is_none() {
            return Err(Error::Abci(AbciError::BadRequest("received a prepare proposal request for the genesis height before an init chain request".to_string())))?;
        }
        if request.round > 0 {
            transaction.as_ref().map(|tx| tx.rollback_to_savepoint());
        }
        transaction
    } else {
        app.start_transaction();
        app.transaction().read().unwrap()
    };

    let transaction = transaction_guard.as_ref().unwrap();

    // Running the proposal executes all the state transitions for the block
    let mut run_result = app
        .platform()
        .run_block_proposal(block_proposal, true, transaction)?;

    if !run_result.is_valid() {
        // This is a system error, because we are proposing
        return Err(run_result.errors.remove(0).into());
    }

    let block_execution_outcome::v0::BlockExecutionOutcome {
        app_hash,
        state_transitions_result,
        validator_set_update,
        protocol_version,
    } = run_result.into_data().map_err(Error::Protocol)?;

    let platform_version = PlatformVersion::get(protocol_version)
        .expect("must be set in run block proposal from existing protocol version");

    // We need to let Tenderdash know about the transactions we should remove from execution
    let valid_tx_count = state_transitions_result.valid_count();
    let failed_tx_count = state_transitions_result.failed_count();
    let delayed_tx_count = transactions_exceeding_max_block_size.len();
    let invalid_paid_tx_count = state_transitions_result.invalid_paid_count();
    let invalid_unpaid_tx_count = state_transitions_result.invalid_unpaid_count();

    let mut tx_results = Vec::new();
    let mut tx_records = Vec::new();

    for (state_transition_execution_result, raw_state_transition) in state_transitions_result
        .into_execution_results()
        .into_iter()
        .zip(request.txs)
    {
        let tx_action = match &state_transition_execution_result {
            StateTransitionExecutionResult::SuccessfulExecution(_, _) => TxAction::Unmodified,
            // We have identity to pay for the state transition, so we keep it in the block
            StateTransitionExecutionResult::PaidConsensusError(_) => TxAction::Unmodified,
            // We don't have any associated identity to pay for the state transition,
            // so we remove it from the block to prevent spam attacks.
            // Such state transitions must be invalidated by check tx, but they might
            // still be added to mempool due to inconsistency between check tx and tx processing
            // (fees calculation) or malicious proposer.
            StateTransitionExecutionResult::UnpaidConsensusError(_) => TxAction::Removed,
            // We shouldn't include in the block any state transitions that produced an internal error
            // during execution
            StateTransitionExecutionResult::DriveAbciError(_) => TxAction::Removed,
        };

        let tx_result: ExecTxResult =
            state_transition_execution_result.try_into_platform_versioned(platform_version)?;

        if tx_action != TxAction::Removed {
            tx_results.push(tx_result);
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

    // TODO: move to a better place :)
    let consensus_param_updates = get_consensus_params_update(
        &app.platform().config.abci.consensus_params_dir,
        request.height,
    )
    .map_err(|e| Error::Abci(AbciError::ConsensusParams(e.to_string())))?;

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
        consensus_param_updates,
    };

    let mut block_execution_context_guard = app.platform().block_execution_context.write().unwrap();

    let block_execution_context = block_execution_context_guard
        .as_mut()
        .expect("expected that a block execution context was set");
    block_execution_context.set_proposer_results(Some(response.clone()));

    let elapsed_time_ms = timer.elapsed().as_millis();

    tracing::info!(
        invalid_paid_tx_count,
        invalid_unpaid_tx_count,
        valid_tx_count,
        delayed_tx_count,
        failed_tx_count,
        "Prepared proposal with {} transitions for height: {}, round: {} in {} ms",
        valid_tx_count + invalid_paid_tx_count,
        request.height,
        request.round,
        elapsed_time_ms,
    );

    Ok(response)
}

/// Determine consensus params that shall be returned at provided height
///
/// If a file `$height.json` is found in `consensus_params_dir` directory, it will be returned.
/// Otherwise, returns None.
///
/// # Arguments
///
/// * `consensus_params_dir` - Directory where consensus params are stored; if empty string, returns None
/// * `height` - Height for which consensus params are requested
///
/// # Returns
///
/// * `Ok(Some(ConsensusParams))` - If file with consensus params for provided height is found
/// * `Ok(None)` - If file with consensus params for provided height is not found
/// * `Err(io::Error)` - If there was an error reading the file
///
// TODO: Move this to correct place
pub(super) fn get_consensus_params_update(
    consensus_params_dir: &str,
    height: i64,
) -> Result<OptionConsensusParams>, std::io::Error> {
    if consensus_params_dir.is_empty() {
        return Ok(None);
    }
    let mut file_path = PathBuf::from(consensus_params_dir);
    file_path.push(format!("{}.json", height));

    // check if file exists
    if !std::path::Path::new(&file_path).exists() {
        return Ok(None);
    }

    let rdr = std::fs::File::open(file_path)?;
    serde_json::from_reader(rdr).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    #[test]
    fn test_get_consensus_params_update() {
        let temp_dir = tempfile::tempdir().unwrap();
        let consensus_params_dir = temp_dir.path().to_str().unwrap().to_string();

        let height = 123456;
        let mut file_path = PathBuf::from(&consensus_params_dir);
        file_path.push(format!("{}.json", height));

        let consensus_params = r#"{
            "block": {
            "max_bytes": "2097152",
            "max_gas": "40000000000"
            },
            "evidence": {
            "max_age_num_blocks": "100000",
            "max_age_duration": "172800000000000",
            "max_bytes": "0"
            },
            "validator": {
            "pub_key_types": [
                "bls12381"
            ]
            },
            "version": {
            "app_version": "1"
            },
            "synchrony": {
            "precision": "500000000",
            "message_delay": "60000000000"
            },
            "timeout": {
            "propose": "40000000000",
            "propose_delta": "5000000000",
            "vote": "40000000000",
            "vote_delta": "5000000000"
            },
            "abci": {
            "recheck_tx": true
            }
        }"#;

        std::fs::write(&file_path, consensus_params).unwrap();

        let result = super::get_consensus_params_update(&consensus_params_dir, height).unwrap();
        println!("{:?}", result);
        assert_eq!(result.unwrap().block.unwrap().max_bytes, 2097152);
    }
}
