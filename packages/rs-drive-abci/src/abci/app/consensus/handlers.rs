// MIT LICENSE
//
// Copyright (c) 2021 Dash Core Group
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.
//

//! Tenderdash ABCI Handlers.
//!
//! This module defines the `TenderdashAbci` trait and implements it for type `Platform`.
//!
//! Handlers in this function MUST be version agnostic, meaning that for all future versions, we
//! can only make changes that are backwards compatible. Otherwise new calls must be made instead.
//!

use crate::abci::app::consensus::ConsensusAbciApplication;
use crate::abci::AbciError;
use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::block_execution_context::v0::{
    BlockExecutionContextV0Getters, BlockExecutionContextV0MutableGetters,
    BlockExecutionContextV0Setters,
};
use crate::execution::types::block_state_info::v0::{
    BlockStateInfoV0Getters, BlockStateInfoV0Methods, BlockStateInfoV0Setters,
};
use crate::platform_types::block_execution_outcome;
use crate::platform_types::block_proposal::v0::BlockProposal;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::platform_state::PlatformState;
use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult;
use crate::rpc::core::CoreRPCLike;
use dpp::dashcore::hashes::Hash;
use dpp::version::PlatformVersion;
use dpp::version::TryIntoPlatformVersioned;
use tenderdash_abci::proto::abci as proto;
use tenderdash_abci::proto::abci::response_verify_vote_extension::VerifyStatus;
use tenderdash_abci::proto::abci::tx_record::TxAction;
use tenderdash_abci::proto::abci::{ExecTxResult, ExtendVoteExtension, TxRecord};
use tenderdash_abci::proto::types::CoreChainLock;

impl<'a, C> tenderdash_abci::Application for ConsensusAbciApplication<'a, C>
where
    C: CoreRPCLike,
{
    fn info(
        &self,
        request: proto::RequestInfo,
    ) -> Result<proto::ResponseInfo, proto::ResponseException> {
        let state_guard = self.platform.state.read().unwrap();

        if !tenderdash_abci::check_version(&request.abci_version) {
            return Err(proto::ResponseException::from(format!(
                "tenderdash requires ABCI version {}, our version is {}",
                request.abci_version,
                tenderdash_abci::proto::ABCI_VERSION
            )));
        }

        let state_app_hash = state_guard
            .last_committed_block_app_hash()
            .map(|app_hash| app_hash.to_vec())
            .unwrap_or_default();

        let latest_platform_version = PlatformVersion::latest();

        let response = proto::ResponseInfo {
            data: "".to_string(),
            app_version: latest_platform_version.protocol_version as u64,
            last_block_height: state_guard.last_committed_block_height() as i64,
            version: env!("CARGO_PKG_VERSION").to_string(),
            last_block_app_hash: state_app_hash.clone(),
        };

        tracing::debug!(
            protocol_version = latest_platform_version.protocol_version,
            software_version = env!("CARGO_PKG_VERSION"),
            block_version = request.block_version,
            p2p_version = request.p2p_version,
            app_hash = hex::encode(state_app_hash),
            height = state_guard.last_committed_block_height(),
            "Handshake with consensus engine",
        );

        if tracing::enabled!(tracing::Level::TRACE) {
            tracing::trace!(
                platform_state_fingerprint = hex::encode(state_guard.fingerprint()),
                "platform runtime state",
            );
        }

        Ok(response)
    }

    fn init_chain(
        &self,
        request: proto::RequestInitChain,
    ) -> Result<proto::ResponseInitChain, proto::ResponseException> {
        self.start_transaction();
        let chain_id = request.chain_id.to_string();

        // We need to drop the block execution context just in case init chain had already been called
        let mut block_execution_context = self.platform.block_execution_context.write().unwrap();
        let block_context = block_execution_context.take(); //drop the block execution context
        if block_context.is_some() {
            tracing::warn!("block context was present during init chain, restarting");
            let protocol_version_in_consensus = self.platform.config.initial_protocol_version;
            let mut platform_state_write_guard = self.platform.state.write().unwrap();
            *platform_state_write_guard = PlatformState::default_with_protocol_versions(
                protocol_version_in_consensus,
                protocol_version_in_consensus,
            );
            drop(platform_state_write_guard);
        }
        drop(block_execution_context);

        let transaction_guard = self.transaction.read().unwrap();
        let transaction = transaction_guard.as_ref().unwrap();
        let response = self.platform.init_chain(request, transaction)?;

        transaction.set_savepoint();

        let app_hash = hex::encode(&response.app_hash);

        tracing::info!(
            app_hash,
            chain_id,
            "Platform chain initialized, initial state is created"
        );

        Ok(response)
    }

    fn query(
        &self,
        _request: proto::RequestQuery,
    ) -> Result<proto::ResponseQuery, proto::ResponseException> {
        unreachable!("query is not implemented for consensus ABCI application")
    }

    fn check_tx(
        &self,
        _request: proto::RequestCheckTx,
    ) -> Result<proto::ResponseCheckTx, proto::ResponseException> {
        unreachable!("check_tx is not implemented for consensus ABCI application")
    }

    fn extend_vote(
        &self,
        request: proto::RequestExtendVote,
    ) -> Result<proto::ResponseExtendVote, proto::ResponseException> {
        let _timer = crate::metrics::abci_request_duration("extend_vote");

        let proto::RequestExtendVote {
            hash: block_hash,
            height,
            round,
        } = request;
        let guarded_block_execution_context = self.platform.block_execution_context.read().unwrap();
        let block_execution_context =
            guarded_block_execution_context
                .as_ref()
                .ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "block execution context must be set in block begin handler for extend vote",
                )))?;

        // Verify Tenderdash that it called this handler correctly
        let block_state_info = &block_execution_context.block_state_info();

        if !block_state_info.matches_current_block(
            height as u64,
            round as u32,
            block_hash.clone(),
        )? {
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

    fn finalize_block(
        &self,
        request: proto::RequestFinalizeBlock,
    ) -> Result<proto::ResponseFinalizeBlock, proto::ResponseException> {
        let _timer = crate::metrics::abci_request_duration("finalize_block");

        let transaction_guard = self.transaction.read().unwrap();

        let transaction = transaction_guard.as_ref().ok_or(Error::Execution(
            ExecutionError::NotInTransaction(
                "trying to finalize block without a current transaction",
            ),
        ))?;

        let block_finalization_outcome = self
            .platform
            .finalize_block_proposal(request.try_into()?, transaction)?;

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
            )
            .into());
        }

        drop(transaction_guard);

        self.commit_transaction()?;

        Ok(proto::ResponseFinalizeBlock {
            events: vec![],
            retain_height: 0,
        })
    }

    fn prepare_proposal(
        &self,
        mut request: proto::RequestPrepareProposal,
    ) -> Result<proto::ResponsePrepareProposal, proto::ResponseException> {
        let timer = crate::metrics::abci_request_duration("prepare_proposal");

        // We should get the latest CoreChainLock from core
        // It is possible that we will not get a chain lock from core, in this case, just don't
        // propose one
        // This is done before all else

        let state = self.platform.state.read().unwrap();

        let last_committed_core_height = state.last_committed_core_height();

        let core_chain_lock_update = match self.platform.core_rpc.get_best_chain_lock() {
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
                transactions_exceeding_max_block_size
                    .extend(request.txs.drain(index_to_remove_at..));
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
        let transaction_guard = if request.height == self.platform.config.abci.genesis_height as i64
        {
            // special logic on init chain
            let transaction = self.transaction.read().unwrap();
            if transaction.is_none() {
                return Err(Error::Abci(AbciError::BadRequest("received a prepare proposal request for the genesis height before an init chain request".to_string())))?;
            }
            if request.round > 0 {
                transaction.as_ref().map(|tx| tx.rollback_to_savepoint());
            }
            transaction
        } else {
            self.start_transaction();
            self.transaction.read().unwrap()
        };

        let transaction = transaction_guard.as_ref().unwrap();

        // Running the proposal executes all the state transitions for the block
        let run_result = self
            .platform
            .run_block_proposal(block_proposal, true, transaction)?;

        if !run_result.is_valid() {
            // This is a system error, because we are proposing
            return Err(run_result.errors.first().unwrap().to_string().into());
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
            // TODO: implement consensus param updates
            consensus_param_updates: None,
        };

        let mut block_execution_context_guard =
            self.platform.block_execution_context.write().unwrap();

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

    fn process_proposal(
        &self,
        mut request: proto::RequestProcessProposal,
    ) -> Result<proto::ResponseProcessProposal, proto::ResponseException> {
        let timer = crate::metrics::abci_request_duration("process_proposal");

        let mut block_execution_context_guard =
            self.platform.block_execution_context.write().unwrap();

        let mut drop_block_execution_context = false;
        if let Some(block_execution_context) = block_execution_context_guard.as_mut() {
            // We are already in a block, or in init chain.
            // This only makes sense if we were the proposer unless we are at a future round
            if block_execution_context.block_state_info().round() != (request.round as u32) {
                // We were not the proposer, and we should process something new
                drop_block_execution_context = true;
            } else if let Some(current_block_hash) =
                block_execution_context.block_state_info().block_hash()
            {
                // There is also the possibility that this block already came in, but tenderdash crashed
                // Now tenderdash is sending it again
                if let Some(proposal_info) = block_execution_context.proposer_results() {
                    tracing::debug!(
                        method = "process_proposal",
                        ?proposal_info, // TODO: It might be too big for debug
                        "we knew block hash, block execution context already had a proposer result",
                    );
                    // We were the proposer as well, so we have the result in cache
                    return Ok(proto::ResponseProcessProposal {
                        status: proto::response_process_proposal::ProposalStatus::Accept.into(),
                        app_hash: proposal_info.app_hash.clone(),
                        tx_results: proposal_info.tx_results.clone(),
                        consensus_param_updates: proposal_info.consensus_param_updates.clone(),
                        validator_set_update: proposal_info.validator_set_update.clone(),
                    });
                }

                if current_block_hash.as_slice() == request.hash {
                    // We were not the proposer, just drop the execution context
                    tracing::warn!(
                        method = "process_proposal",
                        ?request, // Shumkov, lklimek: this structure might be very big and we already logged it such as all other ABCI requests and responses
                        "block execution context already existed, but we are running it again for same height {}/round {}",
                        request.height,
                        request.round,
                    );
                    drop_block_execution_context = true;
                } else {
                    // We are getting a different block hash for a block of the same round
                    // This is a terrible issue
                    return Err(Error::Abci(AbciError::BadRequest(
                        "received a process proposal request twice with different hash".to_string(),
                    )))?;
                }
            } else {
                let Some(proposal_info) = block_execution_context.proposer_results() else {
                    return Err(Error::Abci(AbciError::BadRequest(
                        "received a process proposal request twice".to_string(),
                    )))?;
                };

                let expected_transactions = proposal_info
                    .tx_records
                    .iter()
                    .filter_map(|record| {
                        if record.action == TxAction::Removed as i32
                            || record.action == TxAction::Delayed as i32
                        {
                            None
                        } else {
                            Some(&record.tx)
                        }
                    })
                    .collect::<Vec<_>>();

                // While it is true that the length could be same, seeing how this is such a rare situation
                // It does not seem worth to deal with situations where the length is the same but the transactions have changed
                if expected_transactions.len() == request.txs.len()
                    && proposal_info.core_chain_lock_update == request.core_chain_lock_update
                {
                    let (app_hash, tx_results, consensus_param_updates, validator_set_update) = {
                        tracing::debug!(
                            method = "process_proposal",
                            "we didn't know block hash (we were most likely proposer), block execution context already had a proposer result {:?}",
                            proposal_info,
                        );

                        // Cloning all required properties from proposal_info and then dropping it
                        let app_hash = proposal_info.app_hash.clone();
                        let tx_results = proposal_info.tx_results.clone();
                        let consensus_param_updates = proposal_info.consensus_param_updates.clone();
                        let validator_set_update = proposal_info.validator_set_update.clone();
                        (
                            app_hash,
                            tx_results,
                            consensus_param_updates,
                            validator_set_update,
                        )
                    };

                    // We need to set the block hash
                    block_execution_context
                        .block_state_info_mut()
                        .set_block_hash(Some(request.hash.clone().try_into().map_err(|_| {
                            Error::Abci(AbciError::BadRequestDataSize(
                                "block hash is not 32 bytes in process proposal".to_string(),
                            ))
                        })?));
                    return Ok(proto::ResponseProcessProposal {
                        status: proto::response_process_proposal::ProposalStatus::Accept.into(),
                        app_hash,
                        tx_results,
                        consensus_param_updates,
                        validator_set_update,
                    });
                } else {
                    tracing::warn!(
                            method = "process_proposal",
                            "we didn't know block hash (we were most likely proposer), block execution context already had a proposer result {:?}, but we are requesting a different amount of transactions, dropping the cache",
                            proposal_info,
                        );

                    drop_block_execution_context = true;
                };
            }
        }

        if drop_block_execution_context {
            *block_execution_context_guard = None;
        }
        drop(block_execution_context_guard);

        // Get transaction
        let transaction_guard = if request.height == self.platform.config.abci.genesis_height as i64
        {
            // special logic on init chain
            let transaction = self.transaction.read().unwrap();
            if transaction.is_none() {
                return Err(Error::Abci(AbciError::BadRequest("received a process proposal request for the genesis height before an init chain request".to_string())))?;
            }
            if request.round > 0 {
                transaction.as_ref().map(|tx| tx.rollback_to_savepoint());
            }
            transaction
        } else {
            self.start_transaction();
            self.transaction.read().unwrap()
        };
        let transaction = transaction_guard.as_ref().unwrap();

        // Running the proposal executes all the state transitions for the block
        let run_result =
            self.platform
                .run_block_proposal((&request).try_into()?, false, transaction)?;

        if !run_result.is_valid() {
            // This was an error running this proposal, tell tenderdash that the block isn't valid
            let response = proto::ResponseProcessProposal {
                status: proto::response_process_proposal::ProposalStatus::Reject.into(),
                app_hash: [0; 32].to_vec(), // we must send 32 bytes
                ..Default::default()
            };

            tracing::warn!(
                errors = ?run_result.errors,
                "Rejected invalid proposal for height: {}, round: {}",
                request.height,
                request.round,
            );

            Ok(response)
        } else {
            let block_execution_outcome::v0::BlockExecutionOutcome {
                app_hash,
                state_transitions_result: state_transition_results,
                validator_set_update,
                protocol_version,
            } = run_result.into_data().map_err(Error::Protocol)?;

            let platform_version = PlatformVersion::get(protocol_version)
                .expect("must be set in run block proposer from existing platform version");

            let invalid_tx_count = state_transition_results.invalid_paid_count();
            let valid_tx_count = state_transition_results.valid_count();

            let tx_results = state_transition_results
                .into_execution_results()
                .into_iter()
                // To prevent spam attacks we add to the block state transitions covered with fees only
                .filter(|execution_result| {
                    matches!(
                        execution_result,
                        StateTransitionExecutionResult::SuccessfulExecution(_, _)
                            | StateTransitionExecutionResult::PaidConsensusError(_)
                    )
                })
                .map(|execution_result| {
                    execution_result.try_into_platform_versioned(platform_version)
                })
                .collect::<Result<_, _>>()?;

            let response = proto::ResponseProcessProposal {
                app_hash: app_hash.to_vec(),
                tx_results,
                status: proto::response_process_proposal::ProposalStatus::Accept.into(),
                validator_set_update,
                // TODO: Implement consensus param updates
                consensus_param_updates: None,
            };

            let elapsed_time_ms = timer.elapsed().as_millis();

            tracing::info!(
                invalid_tx_count,
                valid_tx_count,
                elapsed_time_ms,
                "Processed proposal with {} transactions for height: {}, round: {} in {} ms",
                valid_tx_count + invalid_tx_count,
                request.height,
                request.round,
                elapsed_time_ms,
            );

            Ok(response)
        }
    }

    /// Todo: Verify vote extension not really needed because extend vote is deterministic
    fn verify_vote_extension(
        &self,
        request: proto::RequestVerifyVoteExtension,
    ) -> Result<proto::ResponseVerifyVoteExtension, proto::ResponseException> {
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
        let guarded_block_execution_context = self.platform.block_execution_context.read().unwrap();
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
}
