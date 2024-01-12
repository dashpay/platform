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

mod error;
mod execution_result;

use crate::abci::server::AbciApplication;
use crate::error::execution::ExecutionError;

use crate::error::Error;
use crate::rpc::core::CoreRPCLike;
use dpp::errors::consensus::codes::ErrorWithCode;
use tenderdash_abci::proto::abci as proto;
use tenderdash_abci::proto::abci::response_verify_vote_extension::VerifyStatus;
use tenderdash_abci::proto::abci::tx_record::TxAction;
use tenderdash_abci::proto::abci::{
    ExecTxResult, RequestCheckTx, RequestFinalizeBlock, RequestInitChain, RequestPrepareProposal,
    RequestProcessProposal, RequestQuery, ResponseCheckTx, ResponseFinalizeBlock,
    ResponseInitChain, ResponsePrepareProposal, ResponseProcessProposal, ResponseQuery, TxRecord,
};
use tenderdash_abci::proto::types::VoteExtensionType;

use super::AbciError;

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
use crate::platform_types::withdrawal::withdrawal_txs;
use dpp::dashcore::hashes::Hash;
use dpp::fee::SignedCredits;
use dpp::version::TryIntoPlatformVersioned;
use dpp::version::{PlatformVersion, PlatformVersionCurrentVersion};
use error::consensus::AbciResponseInfoGetter;
use error::HandlerError;

impl<'a, C> tenderdash_abci::Application for AbciApplication<'a, C>
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
            .last_block_app_hash()
            .map(|app_hash| app_hash.to_vec())
            .unwrap_or_default();

        let latest_platform_version = PlatformVersion::latest();

        let response = proto::ResponseInfo {
            data: "".to_string(),
            app_version: latest_platform_version.protocol_version as u64,
            last_block_height: state_guard.last_block_height() as i64,
            version: env!("CARGO_PKG_VERSION").to_string(),
            last_block_app_hash: state_app_hash.clone(),
        };

        tracing::debug!(
            protocol_version = latest_platform_version.protocol_version,
            software_version = env!("CARGO_PKG_VERSION"),
            block_version = request.block_version,
            p2p_version = request.p2p_version,
            app_hash = hex::encode(state_app_hash),
            height = state_guard.last_block_height(),
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
        request: RequestInitChain,
    ) -> Result<ResponseInitChain, proto::ResponseException> {
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

    fn prepare_proposal(
        &self,
        mut request: RequestPrepareProposal,
    ) -> Result<ResponsePrepareProposal, proto::ResponseException> {
        let timer = crate::metrics::abci_request_duration("prepare_proposal");

        // We should get the latest CoreChainLock from core
        // It is possible that we will not get a chain lock from core, in this case, just don't
        // propose one
        // This is done before all else

        let core_chain_lock_update = match self.platform.core_rpc.get_best_chain_lock() {
            Ok(latest_chain_lock) => {
                if request.core_chain_locked_height < latest_chain_lock.core_block_height {
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
                core_chain_lock_update.core_block_height,
                request.height
            );
            block_proposal.core_chain_locked_height = core_chain_lock_update.core_block_height;
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
            .run_block_proposal(block_proposal, transaction)?;

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
        let invalid_all_tx_count = state_transitions_result.invalid_count();
        let failed_tx_count = state_transitions_result.failed_count();
        let delayed_tx_count = transactions_exceeding_max_block_size.len();
        let mut invalid_paid_tx_count = state_transitions_result.invalid_count();
        let mut invalid_unpaid_tx_count = state_transitions_result.invalid_count();

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
                StateTransitionExecutionResult::PaidConsensusError(_) => {
                    invalid_paid_tx_count += 1;

                    TxAction::Unmodified
                }
                // We don't have any associated identity to pay for the state transition,
                // so we remove it from the block to prevent spam attacks.
                // Such state transitions must be invalidated by check tx, but they might
                // still be added to mempool due to inconsistency between check tx and tx processing
                // (fees calculation) or malicious proposer.
                StateTransitionExecutionResult::UnpaidConsensusError(_) => {
                    invalid_unpaid_tx_count += 1;

                    TxAction::Removed
                }
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

        let response = ResponsePrepareProposal {
            tx_results,
            app_hash: app_hash.to_vec(),
            tx_records,
            core_chain_lock_update,
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
            invalid_unpaid_tx_count,
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
        mut request: RequestProcessProposal,
    ) -> Result<ResponseProcessProposal, proto::ResponseException> {
        let timer = crate::metrics::abci_request_duration("process_proposal");

        let mut block_execution_context_guard =
            self.platform.block_execution_context.write().unwrap();

        let mut drop_block_execution_context = false;
        if let Some(block_execution_context) = block_execution_context_guard.as_mut() {
            // We are already in a block
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
                    return Ok(ResponseProcessProposal {
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
                    return Ok(ResponseProcessProposal {
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

        // We can take the core chain lock update here because it won't be used anywhere else
        if let Some(_c) = request.core_chain_lock_update.take() {
            //todo: if there is a core chain lock update we need to validate it
        }

        // Running the proposal executes all the state transitions for the block
        let run_result = self
            .platform
            .run_block_proposal((&request).try_into()?, transaction)?;

        if !run_result.is_valid() {
            // This was an error running this proposal, tell tenderdash that the block isn't valid
            let response = ResponseProcessProposal {
                status: proto::response_process_proposal::ProposalStatus::Reject.into(),
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

            let invalid_tx_count = state_transition_results.invalid_count();
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

            let response = ResponseProcessProposal {
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
            Err(Error::from(AbciError::RequestForWrongBlockReceived(format!(
                "received extend vote request for height: {} round: {}, block: {};  expected height: {} round: {}, block: {}",
                height, round, hex::encode(block_hash),
                block_state_info.height(), block_state_info.round(), block_state_info.block_hash().map(hex::encode).unwrap_or("None".to_string())
            )))
                .into())
        } else {
            // we only want to sign the hash of the transaction
            let extensions = block_execution_context
                .withdrawal_transactions()
                .keys()
                .map(|tx_id| proto::ExtendVoteExtension {
                    r#type: VoteExtensionType::ThresholdRecover as i32,
                    extension: tx_id.to_byte_array().to_vec(),
                })
                .collect();
            Ok(proto::ResponseExtendVote {
                vote_extensions: extensions,
            })
        }
    }

    /// Todo: Verify vote extension not really needed because extend vote is deterministic
    fn verify_vote_extension(
        &self,
        request: proto::RequestVerifyVoteExtension,
    ) -> Result<proto::ResponseVerifyVoteExtension, proto::ResponseException> {
        let _timer = crate::metrics::abci_request_duration("verify_vote_extension");

        let proto::RequestVerifyVoteExtension {
            height,
            round,
            vote_extensions,
            ..
        } = request;

        let height: u64 = height as u64;
        let round: u32 = round as u32;

        let guarded_block_execution_context = self.platform.block_execution_context.read().unwrap();
        let Some(block_execution_context) = guarded_block_execution_context.as_ref() else {
            tracing::warn!(
                "vote extension for height: {}, round: {} is rejected because we are not in a block execution phase",
                height,
                round,
            );

            return Ok(proto::ResponseVerifyVoteExtension {
                status: VerifyStatus::Reject.into(),
            });
        };

        let platform_version = block_execution_context
            .block_platform_state()
            .current_platform_version()?;

        let got: withdrawal_txs::v0::WithdrawalTxs = vote_extensions.into();
        let expected = block_execution_context
            .withdrawal_transactions()
            .keys()
            .map(|tx_id| proto::ExtendVoteExtension {
                r#type: VoteExtensionType::ThresholdRecover as i32,
                extension: tx_id.to_byte_array().to_vec(),
            })
            .collect::<Vec<_>>()
            .into();

        // let state = self.platform.state.read().unwrap();
        //
        // let quorum = state.current_validator_set()?;

        // let validator_pro_tx_hash = ProTxHash::from_slice(validator_pro_tx_hash.as_slice())
        //     .map_err(|_| {
        //         Error::Abci(AbciError::BadRequestDataSize(format!(
        //             "invalid vote extension protxhash: {}",
        //             hex::encode(validator_pro_tx_hash.as_slice())
        //         )))
        //     })?;
        //
        // let Some(validator) = quorum.validator_set.get(&validator_pro_tx_hash) else {
        //     return Ok(proto::ResponseVerifyVoteExtension {
        //         status: VerifyStatus::Unknown.into(),
        //     });
        // };

        let block_state_info = block_execution_context.block_state_info();

        //// Verification that vote extension is for our current executed block
        // When receiving the vote extension, we need to make sure that info matches our current block

        if block_state_info.height() != height || block_state_info.round() != round {
            tracing::warn!(
                "vote extension for height: {}, round: {} is rejected because we are at height: {} round {}",
                height,
                round,
                block_state_info.height(),
                block_state_info.round()
            );

            return Ok(proto::ResponseVerifyVoteExtension {
                status: VerifyStatus::Reject.into(),
            });
        }

        let validation_result = self.platform.check_withdrawals(
            &got,
            &expected,
            height as u64,
            round as u32,
            None,
            None,
            platform_version,
        )?;

        if validation_result.is_valid() {
            tracing::debug!(
                "vote extension for height: {}, round: {} is successfully verified",
                height,
                round,
            );

            Ok(proto::ResponseVerifyVoteExtension {
                status: VerifyStatus::Accept.into(),
            })
        } else {
            tracing::error!(
                ?got,
                ?expected,
                ?validation_result.errors,
                "vote extension for height: {}, round: {} mismatch",
                height, round
            );

            Ok(proto::ResponseVerifyVoteExtension {
                status: VerifyStatus::Reject.into(),
            })
        }
    }

    fn finalize_block(
        &self,
        request: RequestFinalizeBlock,
    ) -> Result<ResponseFinalizeBlock, proto::ResponseException> {
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

        Ok(ResponseFinalizeBlock {
            events: vec![],
            retain_height: 0,
        })
    }

    fn check_tx(
        &self,
        request: RequestCheckTx,
    ) -> Result<ResponseCheckTx, proto::ResponseException> {
        let _timer = crate::metrics::abci_request_duration("check_tx");

        let RequestCheckTx { tx, r#type } = request;
        match self.platform.check_tx(tx.as_slice(), r#type.try_into()?) {
            Ok(validation_result) => {
                let platform_state = self.platform.state.read().unwrap();
                let platform_version = platform_state.current_platform_version()?;
                let first_consensus_error = validation_result.errors.first();

                let (code, info) = if let Some(consensus_error) = first_consensus_error {
                    (
                        consensus_error.code(),
                        consensus_error
                            .response_info_for_version(platform_version)
                            .map_err(proto::ResponseException::from)?,
                    )
                } else {
                    // If there are no execution errors the code will be 0
                    (0, "".to_string())
                };

                let gas_wanted = validation_result
                    .data
                    .map(|fee_result| {
                        fee_result
                            .map(|fee_result| fee_result.total_base_fee())
                            .unwrap_or_default()
                    })
                    .unwrap_or_default();

                Ok(ResponseCheckTx {
                    code,
                    data: vec![],
                    info,
                    gas_wanted: gas_wanted as SignedCredits,
                    codespace: "".to_string(),
                    sender: "".to_string(),
                    priority: 0,
                })
            }
            Err(error) => {
                let handler_error = HandlerError::Internal(error.to_string());

                tracing::error!(?error, "check_tx failed");

                Ok(ResponseCheckTx {
                    code: handler_error.code(),
                    data: vec![],
                    info: handler_error.response_info()?,
                    gas_wanted: 0 as SignedCredits,
                    codespace: "".to_string(),
                    sender: "".to_string(),
                    priority: 0,
                })
            }
        }
    }

    fn query(&self, request: RequestQuery) -> Result<ResponseQuery, proto::ResponseException> {
        let _timer = crate::metrics::abci_request_duration("query");

        let RequestQuery { data, path, .. } = &request;

        // TODO: It must be proto::ResponseException
        let Some(platform_version) = PlatformVersion::get_maybe_current() else {
            let handler_error =
                HandlerError::Unavailable("platform is not initialized".to_string());

            let response = ResponseQuery {
                code: handler_error.code(),
                log: "".to_string(),
                info: handler_error.response_info()?,
                index: 0,
                key: vec![],
                value: vec![],
                proof_ops: None,
                height: self.platform.state.read().unwrap().height() as i64,
                codespace: "".to_string(),
            };

            tracing::error!(?response, "platform version not initialized");

            return Ok(response);
        };

        let result = self
            .platform
            .query(path.as_str(), data.as_slice(), platform_version)?;

        let (code, data, info) = if result.is_valid() {
            (0, result.data.unwrap_or_default(), "success".to_string())
        } else {
            let error = result
                .errors
                .first()
                .expect("validation result should have at least one error");

            let handler_error = HandlerError::from(error);

            (handler_error.code(), vec![], handler_error.response_info()?)
        };

        let response = ResponseQuery {
            //todo: right now just put GRPC error codes,
            //  later we will use own error codes
            code,
            log: "".to_string(),
            info,
            index: 0,
            key: vec![],
            value: data,
            proof_ops: None,
            height: self.platform.state.read().unwrap().height() as i64,
            codespace: "".to_string(),
        };

        Ok(response)
    }
}
//
// #[cfg(test)]
// mod tests {
//     mod handlers {
//         use crate::config::PlatformConfig;
//         use crate::rpc::core::MockCoreRPCLike;
//         use chrono::{Duration, Utc};
//         use dashcore_rpc::dashcore::hashes::hex::FromHex;
//         use dashcore_rpc::dashcore::BlockHash;
//         use dpp::contracts::withdrawals_contract;
//
//         use dpp::identity::core_script::CoreScript;
//         use dpp::identity::state_transition::identity_credit_withdrawal_transition::Pooling;
//         use dpp::platform_value::{platform_value, BinaryData};
//         use dpp::prelude::Identifier;
//         use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
//         use dpp::tests::fixtures::get_withdrawal_document_fixture;
//         use dpp::util::hash;
//         use drive::common::helpers::identities::create_test_masternode_identities;
//         use dpp::block::block_info::BlockInfo;
//         use drive::drive::identity::withdrawals::WithdrawalTransactionIdAndBytes;
//         use drive::fee::epoch::CreditsPerEpoch;
//         use drive::fee_pools::epochs::Epoch;
//         use drive::tests::helpers::setup::setup_document;
//         use rust_decimal::prelude::ToPrimitive;
//         use serde_json::json;
//         use std::cmp::Ordering;
//         use std::ops::Div;
//         use tenderdash_abci::Application;
//         use tenderdash_abci::proto::abci::{RequestPrepareProposal, RequestProcessProposal};
//         use tenderdash_abci::proto::google::protobuf::Timestamp;
//
//         use crate::abci::messages::{
//             AfterFinalizeBlockRequest, BlockBeginRequest, BlockEndRequest, BlockFees,
//         };
//         use crate::platform::Platform;
//         use crate::test::fixture::abci::static_init_chain_request;
//         use crate::test::helpers::fee_pools::create_test_masternode_share_identities_and_documents;
//         use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
//
//
//         fn prepare_withdrawal_test(platform: &TempPlatform<MockCoreRPCLike>) {
//             let transaction = platform.drive.grove.start_transaction();
//             //this should happen after
//             let data_contract = load_system_data_contract(SystemDataContract::Withdrawals)
//                 .expect("to load system data contract");
//
//             // Init withdrawal requests
//             let withdrawals: Vec<WithdrawalTransactionIdAndBytes> = (0..16)
//                 .map(|index: u64| (index.to_be_bytes().to_vec(), vec![index as u8; 32]))
//                 .collect();
//
//             let owner_id = Identifier::new([1u8; 32]);
//
//             for (_, tx_bytes) in withdrawals.iter() {
//                 let tx_id = hash::hash(tx_bytes);
//
//                 let document = get_withdrawal_document_fixture(
//                     &data_contract,
//                     owner_id,
//                     platform_value!({
//                         "amount": 1000u64,
//                         "coreFeePerByte": 1u32,
//                         "pooling": Pooling::Never as u8,
//                         "outputScript": CoreScript::from_bytes((0..23).collect::<Vec<u8>>()),
//                         "status": withdrawals_contract::WithdrawalStatus::POOLED as u8,
//                         "transactionIndex": 1u64,
//                         "transactionSignHeight": 93u64,
//                         "transactionId": BinaryData::new(tx_id),
//                     }),
//                     None,
//                 )
//                     .expect("expected withdrawal document");
//
//                 let document_type = data_contract
//                     .document_type(withdrawals_contract::document_types::WITHDRAWAL)
//                     .expect("expected to get document type");
//
//                 setup_document(
//                     &platform.drive,
//                     &document,
//                     &data_contract,
//                     document_type,
//                     Some(&transaction),
//                 );
//             }
//
//             let block_info = BlockInfo {
//                 time_ms: 1,
//                 height: 1,
//                 epoch: Epoch::new(1).unwrap(),
//             };
//
//             let mut drive_operations = vec![];
//
//             platform
//                 .drive
//                 .add_enqueue_withdrawal_transaction_operations(&withdrawals, &mut drive_operations);
//
//             platform
//                 .drive
//                 .apply_drive_operations(drive_operations, true, &block_info, Some(&transaction))
//                 .expect("to apply drive operations");
//
//             platform.drive.grove.commit_transaction(transaction).unwrap().expect("expected to commit transaction")
//         }
//
//         #[test]
//         fn test_abci_flow_with_withdrawals() {
//             let mut platform = TestPlatformBuilder::new()
//                 .with_config(PlatformConfig {
//                     verify_sum_trees: false,
//                     ..Default::default()
//                 })
//                 .build_with_mock_rpc();
//
//             let mut core_rpc_mock = MockCoreRPCLike::new();
//
//             core_rpc_mock
//                 .expect_get_block_hash()
//                 // .times(total_days)
//                 .returning(|_| {
//                     Ok(BlockHash::from_hex(
//                         "0000000000000000000000000000000000000000000000000000000000000000",
//                     )
//                     .unwrap())
//                 });
//
//             core_rpc_mock
//                 .expect_get_block_json()
//                 // .times(total_days)
//                 .returning(|_| Ok(json!({})));
//
//             platform.core_rpc = core_rpc_mock;
//
//             // init chain
//             let init_chain_request = static_init_chain_request();
//
//             platform
//                 .init_chain(init_chain_request)
//                 .expect("should init chain");
//
//             prepare_withdrawal_test(&platform);
//
//             let transaction = platform.drive.grove.start_transaction();
//
//             // setup the contract
//             let contract = platform.create_mn_shares_contract(Some(&transaction));
//
//             let genesis_time = Utc::now();
//
//             let total_days = 29;
//
//             let epoch_1_start_day = 18;
//
//             let blocks_per_day = 50i64;
//
//             let epoch_1_start_block = 13;
//
//             let proposers_count = 50u16;
//
//             let storage_fees_per_block = 42000;
//
//             // and create masternode identities
//             let proposers = create_test_masternode_identities(
//                 &platform.drive,
//                 proposers_count,
//                 Some(51),
//                 Some(&transaction),
//             );
//
//             create_test_masternode_share_identities_and_documents(
//                 &platform.drive,
//                 &contract,
//                 &proposers,
//                 Some(53),
//                 Some(&transaction),
//             );
//
//             platform.drive.grove.commit_transaction(transaction).unwrap().expect("expected to commit transaction");
//
//             let block_interval = 86400i64.div(blocks_per_day);
//
//             let mut previous_block_time_ms: Option<u64> = None;
//
//             // process blocks
//             for day in 0..total_days {
//                 for block_num in 0..blocks_per_day {
//                     let block_time = if day == 0 && block_num == 0 {
//                         genesis_time
//                     } else {
//                         genesis_time
//                             + Duration::days(day as i64)
//                             + Duration::seconds(block_interval * block_num)
//                     };
//
//                     let block_height = 1 + (blocks_per_day as u64 * day as u64) + block_num as u64;
//
//                     let block_time_ms = block_time
//                         .timestamp_millis()
//                         .to_u64()
//                         .expect("block time can not be before 1970");
//
//                     //todo: before we had total_hpmns, where should we put it
//                     let request_process_proposal = RequestPrepareProposal {
//                         max_tx_bytes: 0,
//                         txs: vec![],
//                         local_last_commit: None,
//                         misbehavior: vec![],
//                         height: block_height as i64,
//                         round: 0,
//                         time: Some(Timestamp {
//                             seconds: (block_time_ms / 1000) as i64,
//                             nanos: ((block_time_ms % 1000) * 1000) as i32,
//                         }),
//                         next_validators_hash: [0u8;32].to_vec(),
//                         core_chain_locked_height: 1,
//                         proposer_pro_tx_hash: proposers
//                             .get(block_height as usize % (proposers_count as usize))
//                             .unwrap().to_vec(),
//                         proposed_app_version: 1,
//                         version: None,
//                         quorum_hash: [0u8;32].to_vec(),
//                     };
//
//                     // We are going to process the proposal, during processing we expect internal
//                     // subroutines to take place, these subroutines will create the transactions
//                     let process_proposal_response = platform
//                         .process_proposal(block_begin_request)
//                         .unwrap_or_else(|e| {
//                             panic!(
//                                 "should begin process block #{} for day #{} : {:?}",
//                                 block_height, day, e
//                             )
//                         });
//
//                     // Set previous block time
//                     previous_block_time_ms = Some(block_time_ms);
//
//                     // Should calculate correct current epochs
//                     let (epoch_index, epoch_change) = if day > epoch_1_start_day {
//                         (1, false)
//                     } else if day == epoch_1_start_day {
//                         match block_num.cmp(&epoch_1_start_block) {
//                             Ordering::Less => (0, false),
//                             Ordering::Equal => (1, true),
//                             Ordering::Greater => (1, false),
//                         }
//                     } else if day == 0 && block_num == 0 {
//                         (0, true)
//                     } else {
//                         (0, false)
//                     };
//
//                     assert_eq!(
//                         block_begin_response.epoch_info.current_epoch_index,
//                         epoch_index
//                     );
//
//                     assert_eq!(
//                         block_begin_response.epoch_info.is_epoch_change,
//                         epoch_change
//                     );
//
//                     if day == 0 && block_num == 0 {
//                         let unsigned_withdrawal_hexes = block_begin_response
//                             .unsigned_withdrawal_transactions
//                             .iter()
//                             .map(hex::encode)
//                             .collect::<Vec<String>>();
//
//                         assert_eq!(unsigned_withdrawal_hexes, vec![
//               "200000000000000000000000000000000000000000000000000000000000000000010000002b32db6c2c0a6235fb1397e8225ea85e0f0e6e8c7b126d0016ccbde0e667151e",
//               "200101010101010101010101010101010101010101010101010101010101010101010000002b32db6c2c0a6235fb1397e8225ea85e0f0e6e8c7b126d0016ccbde0e667151e",
//               "200202020202020202020202020202020202020202020202020202020202020202010000002b32db6c2c0a6235fb1397e8225ea85e0f0e6e8c7b126d0016ccbde0e667151e",
//               "200303030303030303030303030303030303030303030303030303030303030303010000002b32db6c2c0a6235fb1397e8225ea85e0f0e6e8c7b126d0016ccbde0e667151e",
//               "200404040404040404040404040404040404040404040404040404040404040404010000002b32db6c2c0a6235fb1397e8225ea85e0f0e6e8c7b126d0016ccbde0e667151e",
//               "200505050505050505050505050505050505050505050505050505050505050505010000002b32db6c2c0a6235fb1397e8225ea85e0f0e6e8c7b126d0016ccbde0e667151e",
//               "200606060606060606060606060606060606060606060606060606060606060606010000002b32db6c2c0a6235fb1397e8225ea85e0f0e6e8c7b126d0016ccbde0e667151e",
//               "200707070707070707070707070707070707070707070707070707070707070707010000002b32db6c2c0a6235fb1397e8225ea85e0f0e6e8c7b126d0016ccbde0e667151e",
//               "200808080808080808080808080808080808080808080808080808080808080808010000002b32db6c2c0a6235fb1397e8225ea85e0f0e6e8c7b126d0016ccbde0e667151e",
//               "200909090909090909090909090909090909090909090909090909090909090909010000002b32db6c2c0a6235fb1397e8225ea85e0f0e6e8c7b126d0016ccbde0e667151e",
//               "200a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a010000002b32db6c2c0a6235fb1397e8225ea85e0f0e6e8c7b126d0016ccbde0e667151e",
//               "200b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b010000002b32db6c2c0a6235fb1397e8225ea85e0f0e6e8c7b126d0016ccbde0e667151e",
//               "200c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c010000002b32db6c2c0a6235fb1397e8225ea85e0f0e6e8c7b126d0016ccbde0e667151e",
//               "200d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d010000002b32db6c2c0a6235fb1397e8225ea85e0f0e6e8c7b126d0016ccbde0e667151e",
//               "200e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e010000002b32db6c2c0a6235fb1397e8225ea85e0f0e6e8c7b126d0016ccbde0e667151e",
//               "200f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f010000002b32db6c2c0a6235fb1397e8225ea85e0f0e6e8c7b126d0016ccbde0e667151e",
//             ]);
//                     } else {
//                         assert_eq!(
//                             block_begin_response.unsigned_withdrawal_transactions.len(),
//                             0
//                         );
//                     }
//
//                     let block_end_request = BlockEndRequest {
//                         fees: BlockFees {
//                             storage_fee: storage_fees_per_block,
//                             processing_fee: 1600,
//                             refunds_per_epoch: CreditsPerEpoch::from_iter([(0, 100)]),
//                         },
//                     };
//
//                     let block_end_response = platform
//                         .block_end(block_end_request, Some(&transaction))
//                         .unwrap_or_else(|_| {
//                             panic!(
//                                 "should end process block #{} for day #{}",
//                                 block_height, day
//                             )
//                         });
//
//                     let after_finalize_block_request = AfterFinalizeBlockRequest {
//                         updated_data_contract_ids: Vec::new(),
//                     };
//
//                     platform
//                         .after_finalize_block(after_finalize_block_request)
//                         .unwrap_or_else(|_| {
//                             panic!(
//                                 "should begin process block #{} for day #{}",
//                                 block_height, day
//                             )
//                         });
//
//                     // Should pay to all proposers for epoch 0, when epochs 1 started
//                     if epoch_index != 0 && epoch_change {
//                         assert!(block_end_response.proposers_paid_count.is_some());
//                         assert!(block_end_response.paid_epoch_index.is_some());
//
//                         assert_eq!(
//                             block_end_response.proposers_paid_count.unwrap(),
//                             proposers_count
//                         );
//                         assert_eq!(block_end_response.paid_epoch_index.unwrap(), 0);
//                     } else {
//                         assert!(block_end_response.proposers_paid_count.is_none());
//                         assert!(block_end_response.paid_epoch_index.is_none());
//                     };
//                 }
//             }
//         }
//
//         #[test]
//         fn test_chain_halt_for_36_days() {
//             // TODO refactor to remove code duplication
//
//             let mut platform = TestPlatformBuilder::new()
//                 .with_config(PlatformConfig {
//                     verify_sum_trees: false,
//                     ..Default::default()
//                 })
//                 .build_with_mock_rpc();
//
//             let mut core_rpc_mock = MockCoreRPCLike::new();
//
//             core_rpc_mock
//                 .expect_get_block_hash()
//                 // .times(1) // TODO: investigate why it always n + 1
//                 .returning(|_| {
//                     Ok(BlockHash::from_hex(
//                         "0000000000000000000000000000000000000000000000000000000000000000",
//                     )
//                     .unwrap())
//                 });
//
//             core_rpc_mock
//                 .expect_get_block_json()
//                 // .times(1) // TODO: investigate why it always n + 1
//                 .returning(|_| Ok(json!({})));
//
//             platform.core_rpc = core_rpc_mock;
//
//             let transaction = platform.drive.grove.start_transaction();
//
//             // init chain
//             let init_chain_request = static_init_chain_request();
//
//             platform
//                 .init_chain(init_chain_request, Some(&transaction))
//                 .expect("should init chain");
//
//             // setup the contract
//             let contract = platform.create_mn_shares_contract(Some(&transaction));
//
//             let genesis_time = Utc::now();
//
//             let epoch_2_start_day = 37;
//
//             let blocks_per_day = 50i64;
//
//             let proposers_count = 50u16;
//
//             let storage_fees_per_block = 42000;
//
//             // and create masternode identities
//             let proposers = create_test_masternode_identities(
//                 &platform.drive,
//                 proposers_count,
//                 Some(52),
//                 Some(&transaction),
//             );
//
//             create_test_masternode_share_identities_and_documents(
//                 &platform.drive,
//                 &contract,
//                 &proposers,
//                 Some(54),
//                 Some(&transaction),
//             );
//
//             let block_interval = 86400i64.div(blocks_per_day);
//
//             let mut previous_block_time_ms: Option<u64> = None;
//
//             // process blocks
//             for day in [0, 1, 2, 3, 37] {
//                 for block_num in 0..blocks_per_day {
//                     let block_time = if day == 0 && block_num == 0 {
//                         genesis_time
//                     } else {
//                         genesis_time
//                             + Duration::days(day as i64)
//                             + Duration::seconds(block_interval * block_num)
//                     };
//
//                     let block_height = 1 + (blocks_per_day as u64 * day as u64) + block_num as u64;
//
//                     let block_time_ms = block_time
//                         .timestamp_millis()
//                         .to_u64()
//                         .expect("block time can not be before 1970");
//
//                     // Processing block
//                     let block_begin_request = BlockBeginRequest {
//                         block_height,
//                         block_time_ms,
//                         previous_block_time_ms,
//                         proposer_pro_tx_hash: *proposers
//                             .get(block_height as usize % (proposers_count as usize))
//                             .unwrap(),
//                         proposed_app_version: 1,
//                         validator_set_quorum_hash: Default::default(),
//                         last_synced_core_height: 1,
//                         core_chain_locked_height: 1,
//                         total_hpmns: proposers_count as u32,
//                     };
//
//                     let block_begin_response = platform
//                         .block_begin(block_begin_request, Some(&transaction))
//                         .unwrap_or_else(|_| {
//                             panic!(
//                                 "should begin process block #{} for day #{}",
//                                 block_height, day
//                             )
//                         });
//
//                     // Set previous block time
//                     previous_block_time_ms = Some(block_time_ms);
//
//                     // Should calculate correct current epochs
//                     let (epoch_index, epoch_change) = if day == epoch_2_start_day {
//                         if block_num == 0 {
//                             (2, true)
//                         } else {
//                             (2, false)
//                         }
//                     } else if day == 0 && block_num == 0 {
//                         (0, true)
//                     } else {
//                         (0, false)
//                     };
//
//                     assert_eq!(
//                         block_begin_response.epoch_info.current_epoch_index,
//                         epoch_index
//                     );
//
//                     assert_eq!(
//                         block_begin_response.epoch_info.is_epoch_change,
//                         epoch_change
//                     );
//
//                     let block_end_request = BlockEndRequest {
//                         fees: BlockFees {
//                             storage_fee: storage_fees_per_block,
//                             processing_fee: 1600,
//                             refunds_per_epoch: CreditsPerEpoch::from_iter([(0, 100)]),
//                         },
//                     };
//
//                     let block_end_response = platform
//                         .block_end(block_end_request, Some(&transaction))
//                         .unwrap_or_else(|_| {
//                             panic!(
//                                 "should end process block #{} for day #{}",
//                                 block_height, day
//                             )
//                         });
//
//                     let after_finalize_block_request = AfterFinalizeBlockRequest {
//                         updated_data_contract_ids: Vec::new(),
//                     };
//
//                     platform
//                         .after_finalize_block(after_finalize_block_request)
//                         .unwrap_or_else(|_| {
//                             panic!(
//                                 "should begin process block #{} for day #{}",
//                                 block_height, day
//                             )
//                         });
//
//                     // Should pay to all proposers for epoch 0, when epochs 1 started
//                     if epoch_index != 0 && epoch_change {
//                         assert!(block_end_response.proposers_paid_count.is_some());
//                         assert!(block_end_response.paid_epoch_index.is_some());
//
//                         assert_eq!(
//                             block_end_response.proposers_paid_count.unwrap(),
//                             blocks_per_day as u16,
//                         );
//                         assert_eq!(block_end_response.paid_epoch_index.unwrap(), 0);
//                     } else {
//                         assert!(block_end_response.proposers_paid_count.is_none());
//                         assert!(block_end_response.paid_epoch_index.is_none());
//                     };
//                 }
//             }
//         }
//     }
// }
