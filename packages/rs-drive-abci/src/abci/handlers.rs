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

use crate::abci::server::AbciApplication;
use crate::block::{BlockExecutionContext, BlockStateInfo};
use crate::execution::fee_pools::epoch::EpochInfo;
use crate::{
    abci::messages::{
        AfterFinalizeBlockRequest, AfterFinalizeBlockResponse, BlockBeginRequest,
        BlockBeginResponse, BlockEndRequest, BlockEndResponse, InitChainRequest, InitChainResponse,
    },
    rpc::core::CoreRPCLike,
};
use dpp::identity::TimestampMillis;
use drive::error::drive::DriveError;
use drive::error::Error::GroveDB;
use drive::grovedb::{Transaction, TransactionArg};
use tenderdash_abci::proto::{
    abci::{self as proto, ResponseException},
    serializers::timestamp::ToMilis,
};
use tenderdash_abci::proto::abci::{ExecTxResult, RequestCheckTx, RequestFinalizeBlock, ResponseCheckTx, ResponseFinalizeBlock};
use dpp::state_transition::StateTransition;
use dpp::util::vec::vec_to_array;
use drive::fee::credits::SignedCredits;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::execution_event::ExecutionResult;
use crate::execution::proposal::Proposal;
use crate::platform::Platform;
use crate::validation::state_transition::StateTransitionValidation;

impl<'a, C> tenderdash_abci::Application for Platform<C>
where
    C: CoreRPCLike,
{
    fn info(&self, request: proto::RequestInfo) -> Result<proto::ResponseInfo, ResponseException> {
        if !tenderdash_abci::check_version(&request.abci_version) {
            return Err(ResponseException::from(format!(
                "tenderdash requires ABCI version {}, our version is {}",
                request.version,
                tenderdash_abci::proto::ABCI_VERSION
            )));
        }

        let response = proto::ResponseInfo {
            app_version: 1,
            version: env!("CARGO_PKG_VERSION").to_string(),
            ..Default::default()
        };

        tracing::info!(method = "info", ?request, ?response, "info executed");
        Ok(response)
    }

    fn init_chain(
        &self,
        request: proto::RequestInitChain,
    ) -> Result<proto::ResponseInitChain, ResponseException> {
        let transaction = self.drive.grove.start_transaction();
        let genesis_time = request
            .time
            .ok_or("genesis time is required in init chain")?
            .to_milis() as TimestampMillis;

        self.create_genesis_state(
            genesis_time,
            self.config.keys.clone().into(),
            Some(&transaction),
        )?;

        self.drive.commit_transaction(transaction)?;

        let response = proto::ResponseInitChain {
            ..Default::default()
        };

        tracing::info!(method = "init_chain", "init chain executed");
        Ok(response)
    }

    fn prepare_proposal(
        &self,
        request: proto::RequestPrepareProposal,
    ) -> Result<proto::ResponsePrepareProposal, ResponseException> {
        let proto::RequestPrepareProposal {
            max_tx_bytes,
            txs,
            local_last_commit,
            misbehavior,
            height,
            time,
            next_validators_hash,
            round,
            core_chain_locked_height,
            proposer_pro_tx_hash,
            proposed_app_version,
            version,
            quorum_hash,
        } = request;

        let transaction = self.drive.grove.start_transaction();
        let time = time.as_ref().ok_or("missing proposal time")?.to_milis();

        let genesis_time_ms = self.get_genesis_time_or_set_if_genesis(height as u64, time, &transaction)?;

        let validator_pro_tx_hash: [u8; 32] = proposer_pro_tx_hash
            .try_into()
            .map_err(|e| format!("invalid proposer protxhash: {}", hex::encode(e)))?;

        self.drive
            .update_validator_proposed_app_version(
                validator_pro_tx_hash,
                proposed_app_version as u32,
                Some(&transaction),
            )
            .map_err(|e| format!("cannot update proposed app version: {}", e))?;

        // Init block execution context
        let block_state_info = BlockStateInfo::from_prepare_proposal_request(&request);

        let epoch_info = EpochInfo::from_genesis_time_and_block_info(genesis_time_ms, &block_state_info)?;

        let block_info = block_state_info.to_block_info(epoch_info.current_epoch_index);
        // FIXME: we need to calculate total hpmns based on masternode list (or remove hpmn_count if not needed)
        let total_hpmns = self.config.quorum_size as u32;
        let block_execution_context = BlockExecutionContext {
            current_transaction: transaction,
            block_info: block_state_info,
            epoch_info: epoch_info.clone(),
            hpmn_count: total_hpmns,
        };

        // If last synced Core block height is not set instead of scanning
        // number of blocks for asset unlock transactions scan only one
        // on Core chain locked height by setting last_synced_core_height to the same value
        // FIXME: re-enable and implement
        // let last_synced_core_height = if request.last_synced_core_height == 0 {
        //     block_execution_context.block_info.core_chain_locked_height
        // } else {
        //     request.last_synced_core_height
        // };
        let last_synced_core_height = block_execution_context.block_info.core_chain_locked_height;

        self.block_execution_context
            .write()
            .unwrap()
            .replace(block_execution_context);

        self.update_broadcasted_withdrawal_transaction_statuses(
            last_synced_core_height,
            &transaction,
        )?;

        self.update_broadcasted_withdrawal_transaction_statuses(
            last_synced_core_height,
            &transaction,
        )?;

        let unsigned_withdrawal_transaction_bytes = self
            .fetch_and_prepare_unsigned_withdrawal_transactions(
                vec_to_array(&request.quorum_hash).expect("invalid quorum hash"),
                &transaction,
            )?;

        let state_transitions = StateTransition::deserialize_many(&txs)?;

        let tx_results = state_transitions
            .into_iter()
            .map(|state_transition| {
                let state_transition_execution_event = state_transition.validate_all(self)?;
                // we map the result to the actual execution
                let execution_result : ExecutionResult = state_transition_execution_event
                    .map_result(|execution_event| {
                        self.execute_event(execution_event, &block_info, &transaction)
                    })?.into();
                execution_result.into()
            })
            .collect::<Result<Vec<ExecTxResult>, Error>>()?;

        let root_hash = self.drive.grove.root_hash(Some(&transaction)).unwrap()?;

        // TODO: implement all fields, including tx processing; for now, just leaving bare minimum
        let response = proto::ResponsePrepareProposal {
            app_hash: root_hash.to_vec(),
            tx_results,
            ..Default::default()
        };

        Ok(response)
    }

    fn finalize_block(&self, request: RequestFinalizeBlock) -> Result<ResponseFinalizeBlock, ResponseException> {
        // Retrieve block execution context
        let mut block_execution_context = self.block_execution_context.write().unwrap();
        let block_execution_context = block_execution_context.take().ok_or(Error::Execution(
            ExecutionError::CorruptedCodeExecution(
                "block execution context must be set in block begin handler",
            ),
        ))?;

        self.pool_withdrawals_into_transactions_queue(&block_execution_context)?;

        let BlockExecutionContext {
            current_transaction,
            block_info,
            epoch_info,
            hpmn_count,
        } = block_execution_context;

        // Process fees
        let process_block_fees_outcome =
            self.process_block_fees(&block_info, &epoch_info, request.fees, &current_transaction)?;

        // Determine a new protocol version if enough proposers voted
        let changed_protocol_version = if epoch_info.is_epoch_change_but_not_genesis() {
            let mut state = self.state.write().unwrap();
            // Set current protocol version to the version from upcoming epoch
            state.current_protocol_version_in_consensus = state.next_epoch_protocol_version;

            // Determine new protocol version based on votes for the next epoch
            let maybe_new_protocol_version =
                self.check_for_desired_protocol_upgrade(hpmn_count, &state, &current_transaction)?;
            if let Some(new_protocol_version) = maybe_new_protocol_version {
                state.next_epoch_protocol_version = new_protocol_version;
            } else {
                state.next_epoch_protocol_version = state.current_protocol_version_in_consensus;
            }

            Some(state.current_protocol_version_in_consensus)
        } else {
            None
        };

        self.drive
            .grove
            .commit_transaction(current_transaction)
            .unwrap()
            .map_err(|e| Error::Drive(GroveDB(e)))?;

        let mut drive_cache = self.drive.cache.write().unwrap();

        drive_cache.cached_contracts.clear_block_cache();

        Ok(AfterFinalizeBlockResponse {})

        Ok(BlockEndResponse::from_outcomes(
            &process_block_fees_outcome,
            changed_protocol_version,
        ))
    }

    fn check_tx(&self, request: RequestCheckTx) -> Result<ResponseCheckTx, ResponseException> {
        let proto::RequestCheckTx {
            tx, r#type
        } = request;
        let state_transition = StateTransition::deserialize(tx.as_slice())?;
        let execution_event = state_transition.validate_all(self)?;

        // We should run the execution event in dry run to see if we would have enough fees for the transaction

        // We do not put the transaction, because this event happens outside of a block
        let validation_result = execution_event
            .and_then_validation(|execution_event| {
                self.validate_fees_of_event(&execution_event, &block_info, None)
            })?;

        // If there are no execution errors the code will be 0
        let code = validation_result.errors.first().map(|error| error.code()).unwrap_or_default();
        let gas_wanted = validation_result.data.map(|fee_result| fee_result.total_base_fee()).unwrap_or_default();
        Ok(ResponseCheckTx {
            code,
            data: vec![],
            info: "".to_string(),
            gas_wanted: gas_wanted as SignedCredits,
            codespace: "".to_string(),
            sender: "".to_string(),
            priority: 0,
        })
    }

    //
    // fn process_proposal(
    //     &self,
    //     _request: RequestProcessProposal,
    // ) -> Result<ResponseProcessProposal, ResponseException> {
    //     let platform = self.platform();
    //     let transaction = self.transaction();
    //     let response = platform.prepare_proposal(&request, transaction)?;
    //
    //     tracing::info!(
    //         method = "prepare_proposal",
    //         height = request.height,
    //         "prepare proposal executed",
    //     );
    //     Ok(response)
    // }
    //
    // fn check_tx(&self, request: RequestCheckTx) -> Result<ResponseCheckTx, ResponseException> {
    //     let RequestCheckTx { tx, .. } = request;
    //     let state_transition = StateTransition::from(tx);
    //
    //     ResponseCheckTx {
    //         code: 0,
    //         data: vec![],
    //         info: "".to_string(),
    //         gas_wanted: 0,
    //         codespace: "".to_string(),
    //         sender: "".to_string(),
    //         priority: 0,
    //     }
    // }
}

#[cfg(test)]
mod tests {
    mod handlers {
        use crate::abci::handlers::TenderdashAbci;
        use crate::config::PlatformConfig;
        use crate::rpc::core::MockCoreRPCLike;
        use chrono::{Duration, Utc};
        use dashcore::hashes::hex::FromHex;
        use dashcore::BlockHash;
        use dpp::contracts::withdrawals_contract;
        use dpp::data_contract::DriveContractExt;
        use dpp::identity::core_script::CoreScript;
        use dpp::identity::state_transition::identity_credit_withdrawal_transition::Pooling;
        use dpp::platform_value::{platform_value, BinaryData};
        use dpp::prelude::Identifier;
        use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
        use dpp::tests::fixtures::get_withdrawal_document_fixture;
        use dpp::util::hash;
        use drive::common::helpers::identities::create_test_masternode_identities;
        use drive::drive::block_info::BlockInfo;
        use drive::drive::identity::withdrawals::WithdrawalTransactionIdAndBytes;
        use drive::fee::epoch::CreditsPerEpoch;
        use drive::fee_pools::epochs::Epoch;
        use drive::tests::helpers::setup::setup_document;
        use rust_decimal::prelude::ToPrimitive;
        use serde_json::json;
        use std::cmp::Ordering;
        use std::ops::Div;

        use crate::abci::messages::{
            AfterFinalizeBlockRequest, BlockBeginRequest, BlockEndRequest, BlockFees,
        };
        use crate::test::fixture::abci::static_init_chain_request;
        use crate::test::helpers::fee_pools::create_test_masternode_share_identities_and_documents;
        use crate::test::helpers::setup::TestPlatformBuilder;

        // TODO: Should we remove this test in favor of strategy tests?

        #[test]
        fn test_abci_flow() {
            let mut platform = TestPlatformBuilder::new()
                .with_config(PlatformConfig {
                    verify_sum_trees: false,
                    ..Default::default()
                })
                .build_with_mock_rpc();

            let mut core_rpc_mock = MockCoreRPCLike::new();

            core_rpc_mock
                .expect_get_block_hash()
                // .times(total_days)
                .returning(|_| {
                    Ok(BlockHash::from_hex(
                        "0000000000000000000000000000000000000000000000000000000000000000",
                    )
                    .unwrap())
                });

            core_rpc_mock
                .expect_get_block_json()
                // .times(total_days)
                .returning(|_| Ok(json!({})));

            platform.core_rpc = core_rpc_mock;

            // init chain
            let init_chain_request = static_init_chain_request();

            platform
                .init_chain(init_chain_request)
                .expect("should init chain");

            let data_contract = load_system_data_contract(SystemDataContract::Withdrawals)
                .expect("to load system data contract");

            // Init withdrawal requests
            let withdrawals: Vec<WithdrawalTransactionIdAndBytes> = (0..16)
                .map(|index: u64| (index.to_be_bytes().to_vec(), vec![index as u8; 32]))
                .collect();

            let owner_id = Identifier::new([1u8; 32]);

            for (_, tx_bytes) in withdrawals.iter() {
                let tx_id = hash::hash(tx_bytes);

                let document = get_withdrawal_document_fixture(
                    &data_contract,
                    owner_id,
                    platform_value!({
                        "amount": 1000u64,
                        "coreFeePerByte": 1u32,
                        "pooling": Pooling::Never as u8,
                        "outputScript": CoreScript::from_bytes((0..23).collect::<Vec<u8>>()),
                        "status": withdrawals_contract::WithdrawalStatus::POOLED as u8,
                        "transactionIndex": 1u64,
                        "transactionSignHeight": 93u64,
                        "transactionId": BinaryData::new(tx_id),
                    }),
                    None,
                )
                .expect("expected withdrawal document");

                let document_type = data_contract
                    .document_type_for_name(withdrawals_contract::document_types::WITHDRAWAL)
                    .expect("expected to get document type");

                setup_document(
                    &platform.drive,
                    &document,
                    &data_contract,
                    document_type,
                    Some(&transaction),
                );
            }

            let block_info = BlockInfo {
                time_ms: 1,
                height: 1,
                epoch: Epoch::new(1),
            };

            let mut drive_operations = vec![];

            platform
                .drive
                .add_enqueue_withdrawal_transaction_operations(&withdrawals, &mut drive_operations);

            platform
                .drive
                .apply_drive_operations(drive_operations, true, &block_info, Some(&transaction))
                .expect("to apply drive operations");

            // setup the contract
            let contract = platform.create_mn_shares_contract(Some(&transaction));

            let genesis_time = Utc::now();

            let total_days = 29;

            let epoch_1_start_day = 18;

            let blocks_per_day = 50i64;

            let epoch_1_start_block = 13;

            let proposers_count = 50u16;

            let storage_fees_per_block = 42000;

            // and create masternode identities
            let proposers = create_test_masternode_identities(
                &platform.drive,
                proposers_count,
                Some(51),
                Some(&transaction),
            );

            create_test_masternode_share_identities_and_documents(
                &platform.drive,
                &contract,
                &proposers,
                Some(53),
                Some(&transaction),
            );

            let block_interval = 86400i64.div(blocks_per_day);

            let mut previous_block_time_ms: Option<u64> = None;

            // process blocks
            for day in 0..total_days {
                for block_num in 0..blocks_per_day {
                    let block_time = if day == 0 && block_num == 0 {
                        genesis_time
                    } else {
                        genesis_time
                            + Duration::days(day as i64)
                            + Duration::seconds(block_interval * block_num)
                    };

                    let block_height = 1 + (blocks_per_day as u64 * day as u64) + block_num as u64;

                    let block_time_ms = block_time
                        .timestamp_millis()
                        .to_u64()
                        .expect("block time can not be before 1970");

                    // Processing block
                    let block_begin_request = BlockBeginRequest {
                        block_height,
                        block_time_ms,
                        previous_block_time_ms,
                        proposer_pro_tx_hash: *proposers
                            .get(block_height as usize % (proposers_count as usize))
                            .unwrap(),
                        proposed_app_version: 1,
                        validator_set_quorum_hash: Default::default(),
                        last_synced_core_height: 1,
                        core_chain_locked_height: 1,
                        total_hpmns: proposers_count as u32,
                    };

                    let block_begin_response = platform
                        .block_begin(block_begin_request)
                        .unwrap_or_else(|e| {
                            panic!(
                                "should begin process block #{} for day #{} : {}",
                                block_height, day, e
                            )
                        });

                    // Set previous block time
                    previous_block_time_ms = Some(block_time_ms);

                    // Should calculate correct current epochs
                    let (epoch_index, epoch_change) = if day > epoch_1_start_day {
                        (1, false)
                    } else if day == epoch_1_start_day {
                        match block_num.cmp(&epoch_1_start_block) {
                            Ordering::Less => (0, false),
                            Ordering::Equal => (1, true),
                            Ordering::Greater => (1, false),
                        }
                    } else if day == 0 && block_num == 0 {
                        (0, true)
                    } else {
                        (0, false)
                    };

                    assert_eq!(
                        block_begin_response.epoch_info.current_epoch_index,
                        epoch_index
                    );

                    assert_eq!(
                        block_begin_response.epoch_info.is_epoch_change,
                        epoch_change
                    );

                    if day == 0 && block_num == 0 {
                        let unsigned_withdrawal_hexes = block_begin_response
                            .unsigned_withdrawal_transactions
                            .iter()
                            .map(hex::encode)
                            .collect::<Vec<String>>();

                        assert_eq!(unsigned_withdrawal_hexes, vec![
              "200000000000000000000000000000000000000000000000000000000000000000010000002b32db6c2c0a6235fb1397e8225ea85e0f0e6e8c7b126d0016ccbde0e667151e",
              "200101010101010101010101010101010101010101010101010101010101010101010000002b32db6c2c0a6235fb1397e8225ea85e0f0e6e8c7b126d0016ccbde0e667151e",
              "200202020202020202020202020202020202020202020202020202020202020202010000002b32db6c2c0a6235fb1397e8225ea85e0f0e6e8c7b126d0016ccbde0e667151e",
              "200303030303030303030303030303030303030303030303030303030303030303010000002b32db6c2c0a6235fb1397e8225ea85e0f0e6e8c7b126d0016ccbde0e667151e",
              "200404040404040404040404040404040404040404040404040404040404040404010000002b32db6c2c0a6235fb1397e8225ea85e0f0e6e8c7b126d0016ccbde0e667151e",
              "200505050505050505050505050505050505050505050505050505050505050505010000002b32db6c2c0a6235fb1397e8225ea85e0f0e6e8c7b126d0016ccbde0e667151e",
              "200606060606060606060606060606060606060606060606060606060606060606010000002b32db6c2c0a6235fb1397e8225ea85e0f0e6e8c7b126d0016ccbde0e667151e",
              "200707070707070707070707070707070707070707070707070707070707070707010000002b32db6c2c0a6235fb1397e8225ea85e0f0e6e8c7b126d0016ccbde0e667151e",
              "200808080808080808080808080808080808080808080808080808080808080808010000002b32db6c2c0a6235fb1397e8225ea85e0f0e6e8c7b126d0016ccbde0e667151e",
              "200909090909090909090909090909090909090909090909090909090909090909010000002b32db6c2c0a6235fb1397e8225ea85e0f0e6e8c7b126d0016ccbde0e667151e",
              "200a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a010000002b32db6c2c0a6235fb1397e8225ea85e0f0e6e8c7b126d0016ccbde0e667151e",
              "200b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b010000002b32db6c2c0a6235fb1397e8225ea85e0f0e6e8c7b126d0016ccbde0e667151e",
              "200c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c010000002b32db6c2c0a6235fb1397e8225ea85e0f0e6e8c7b126d0016ccbde0e667151e",
              "200d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d010000002b32db6c2c0a6235fb1397e8225ea85e0f0e6e8c7b126d0016ccbde0e667151e",
              "200e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e010000002b32db6c2c0a6235fb1397e8225ea85e0f0e6e8c7b126d0016ccbde0e667151e",
              "200f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f010000002b32db6c2c0a6235fb1397e8225ea85e0f0e6e8c7b126d0016ccbde0e667151e",
            ]);
                    } else {
                        assert_eq!(
                            block_begin_response.unsigned_withdrawal_transactions.len(),
                            0
                        );
                    }

                    let block_end_request = BlockEndRequest {
                        fees: BlockFees {
                            storage_fee: storage_fees_per_block,
                            processing_fee: 1600,
                            refunds_per_epoch: CreditsPerEpoch::from_iter([(0, 100)]),
                        },
                    };

                    let block_end_response = platform
                        .block_end(block_end_request, Some(&transaction))
                        .unwrap_or_else(|_| {
                            panic!(
                                "should end process block #{} for day #{}",
                                block_height, day
                            )
                        });

                    let after_finalize_block_request = AfterFinalizeBlockRequest {
                        updated_data_contract_ids: Vec::new(),
                    };

                    platform
                        .after_finalize_block(after_finalize_block_request)
                        .unwrap_or_else(|_| {
                            panic!(
                                "should begin process block #{} for day #{}",
                                block_height, day
                            )
                        });

                    // Should pay to all proposers for epoch 0, when epochs 1 started
                    if epoch_index != 0 && epoch_change {
                        assert!(block_end_response.proposers_paid_count.is_some());
                        assert!(block_end_response.paid_epoch_index.is_some());

                        assert_eq!(
                            block_end_response.proposers_paid_count.unwrap(),
                            proposers_count
                        );
                        assert_eq!(block_end_response.paid_epoch_index.unwrap(), 0);
                    } else {
                        assert!(block_end_response.proposers_paid_count.is_none());
                        assert!(block_end_response.paid_epoch_index.is_none());
                    };
                }
            }
        }

        #[test]
        fn test_chain_halt_for_36_days() {
            // TODO refactor to remove code duplication

            let mut platform = TestPlatformBuilder::new()
                .with_config(PlatformConfig {
                    verify_sum_trees: false,
                    ..Default::default()
                })
                .build_with_mock_rpc();

            let mut core_rpc_mock = MockCoreRPCLike::new();

            core_rpc_mock
                .expect_get_block_hash()
                // .times(1) // TODO: investigate why it always n + 1
                .returning(|_| {
                    Ok(BlockHash::from_hex(
                        "0000000000000000000000000000000000000000000000000000000000000000",
                    )
                    .unwrap())
                });

            core_rpc_mock
                .expect_get_block_json()
                // .times(1) // TODO: investigate why it always n + 1
                .returning(|_| Ok(json!({})));

            platform.core_rpc = core_rpc_mock;

            let transaction = platform.drive.grove.start_transaction();

            // init chain
            let init_chain_request = static_init_chain_request();

            platform
                .init_chain(init_chain_request, Some(&transaction))
                .expect("should init chain");

            // setup the contract
            let contract = platform.create_mn_shares_contract(Some(&transaction));

            let genesis_time = Utc::now();

            let epoch_2_start_day = 37;

            let blocks_per_day = 50i64;

            let proposers_count = 50u16;

            let storage_fees_per_block = 42000;

            // and create masternode identities
            let proposers = create_test_masternode_identities(
                &platform.drive,
                proposers_count,
                Some(52),
                Some(&transaction),
            );

            create_test_masternode_share_identities_and_documents(
                &platform.drive,
                &contract,
                &proposers,
                Some(54),
                Some(&transaction),
            );

            let block_interval = 86400i64.div(blocks_per_day);

            let mut previous_block_time_ms: Option<u64> = None;

            // process blocks
            for day in [0, 1, 2, 3, 37] {
                for block_num in 0..blocks_per_day {
                    let block_time = if day == 0 && block_num == 0 {
                        genesis_time
                    } else {
                        genesis_time
                            + Duration::days(day as i64)
                            + Duration::seconds(block_interval * block_num)
                    };

                    let block_height = 1 + (blocks_per_day as u64 * day as u64) + block_num as u64;

                    let block_time_ms = block_time
                        .timestamp_millis()
                        .to_u64()
                        .expect("block time can not be before 1970");

                    // Processing block
                    let block_begin_request = BlockBeginRequest {
                        block_height,
                        block_time_ms,
                        previous_block_time_ms,
                        proposer_pro_tx_hash: *proposers
                            .get(block_height as usize % (proposers_count as usize))
                            .unwrap(),
                        proposed_app_version: 1,
                        validator_set_quorum_hash: Default::default(),
                        last_synced_core_height: 1,
                        core_chain_locked_height: 1,
                        total_hpmns: proposers_count as u32,
                    };

                    let block_begin_response = platform
                        .block_begin(block_begin_request, Some(&transaction))
                        .unwrap_or_else(|_| {
                            panic!(
                                "should begin process block #{} for day #{}",
                                block_height, day
                            )
                        });

                    // Set previous block time
                    previous_block_time_ms = Some(block_time_ms);

                    // Should calculate correct current epochs
                    let (epoch_index, epoch_change) = if day == epoch_2_start_day {
                        if block_num == 0 {
                            (2, true)
                        } else {
                            (2, false)
                        }
                    } else if day == 0 && block_num == 0 {
                        (0, true)
                    } else {
                        (0, false)
                    };

                    assert_eq!(
                        block_begin_response.epoch_info.current_epoch_index,
                        epoch_index
                    );

                    assert_eq!(
                        block_begin_response.epoch_info.is_epoch_change,
                        epoch_change
                    );

                    let block_end_request = BlockEndRequest {
                        fees: BlockFees {
                            storage_fee: storage_fees_per_block,
                            processing_fee: 1600,
                            refunds_per_epoch: CreditsPerEpoch::from_iter([(0, 100)]),
                        },
                    };

                    let block_end_response = platform
                        .block_end(block_end_request, Some(&transaction))
                        .unwrap_or_else(|_| {
                            panic!(
                                "should end process block #{} for day #{}",
                                block_height, day
                            )
                        });

                    let after_finalize_block_request = AfterFinalizeBlockRequest {
                        updated_data_contract_ids: Vec::new(),
                    };

                    platform
                        .after_finalize_block(after_finalize_block_request)
                        .unwrap_or_else(|_| {
                            panic!(
                                "should begin process block #{} for day #{}",
                                block_height, day
                            )
                        });

                    // Should pay to all proposers for epoch 0, when epochs 1 started
                    if epoch_index != 0 && epoch_change {
                        assert!(block_end_response.proposers_paid_count.is_some());
                        assert!(block_end_response.paid_epoch_index.is_some());

                        assert_eq!(
                            block_end_response.proposers_paid_count.unwrap(),
                            blocks_per_day as u16,
                        );
                        assert_eq!(block_end_response.paid_epoch_index.unwrap(), 0);
                    } else {
                        assert!(block_end_response.proposers_paid_count.is_none());
                        assert!(block_end_response.paid_epoch_index.is_none());
                    };
                }
            }
        }
    }
}
