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
use crate::rpc::core::CoreRPCLike;
use dpp::ProtocolError;
use drive::fee::credits::SignedCredits;
use tenderdash_abci::proto::abci::response_verify_vote_extension::VerifyStatus;
use tenderdash_abci::proto::abci::tx_record::TxAction;
use tenderdash_abci::proto::abci::{self as proto, ResponseException};
use tenderdash_abci::proto::abci::{
    ExecTxResult, RequestCheckTx, RequestFinalizeBlock, RequestInitChain, RequestPrepareProposal,
    RequestProcessProposal, RequestQuery, ResponseCheckTx, ResponseFinalizeBlock,
    ResponseInitChain, ResponsePrepareProposal, ResponseProcessProposal, ResponseQuery, TxRecord,
};

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::engine::BlockExecutionOutcome;

use super::withdrawal::WithdrawalTxs;
use super::AbciError;

impl<'a, C> tenderdash_abci::Application for AbciApplication<'a, C>
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
        request: RequestInitChain,
    ) -> Result<ResponseInitChain, ResponseException> {
        self.platform.init_chain(request)?;

        let response = ResponseInitChain {
            ..Default::default()
        };

        tracing::info!(method = "init_chain", "init chain executed");
        Ok(response)
    }

    fn prepare_proposal(
        &self,
        request: RequestPrepareProposal,
    ) -> Result<ResponsePrepareProposal, ResponseException> {
        self.start_transaction();
        let transaction_guard = self.transaction.read().unwrap();
        let transaction = transaction_guard.as_ref().unwrap();
        // Running the proposal executes all the state transitions for the block
        let run_result = self
            .platform
            .run_block_proposal((&request).try_into()?, &transaction)?;

        if !run_result.is_valid() {
            // This is a system error, because we are proposing
            return Err(run_result.errors.first().unwrap().to_string().into());
        }

        //todo: we need to set the block hash

        let BlockExecutionOutcome {
            app_hash,
            tx_results,
        } = run_result.into_data().map_err(Error::Protocol)?;

        // We need to let Tenderdash know about the transactions we should remove from execution
        let (tx_results, tx_records): (Vec<Option<ExecTxResult>>, Vec<TxRecord>) = tx_results
            .into_iter()
            .map(|result| {
                if result.code > 0 {
                    (
                        None,
                        TxRecord {
                            action: TxAction::Removed as i32,
                            tx: result.data,
                        },
                    )
                } else {
                    (
                        Some(result.clone()),
                        TxRecord {
                            action: TxAction::Unmodified as i32,
                            tx: result.data,
                        },
                    )
                }
            })
            .unzip();

        let tx_results = tx_results.into_iter().flatten().collect();

        // We should get the latest CoreChainLock from core
        let latest_chain_lock = self
            .platform
            .core_rpc
            .get_best_chain_lock()
            .map_err(Error::CoreRpc)?;

        let core_chain_lock_update =
            if request.core_chain_locked_height < latest_chain_lock.core_block_height {
                Some(latest_chain_lock)
            } else {
                None
            };

        // Next we should check for validator set updates
        // todo: validator set updates

        // TODO: implement all fields, including tx processing; for now, just leaving bare minimum
        let response = ResponsePrepareProposal {
            tx_results,
            app_hash: app_hash.to_vec(),
            tx_records,
            core_chain_lock_update,
            ..Default::default()
        };

        Ok(response)
    }

    fn process_proposal(
        &self,
        mut request: RequestProcessProposal,
    ) -> Result<ResponseProcessProposal, ResponseException> {
        self.start_transaction();
        let transaction_guard = self.transaction.read().unwrap();
        let transaction = transaction_guard.as_ref().unwrap();

        // We can take the core chain lock update here because it won't be used anywhere else
        if let Some(c) = request.core_chain_lock_update.take() {
            //todo: if there is a core chain lock update we need to validate it
        }

        // Running the proposal executes all the state transitions for the block
        let run_result = self
            .platform
            .run_block_proposal((&request).try_into()?, &transaction)?;

        if !run_result.is_valid() {
            // This was an error running this proposal, tell tenderdash that the block isn't valid
            let response = ResponseProcessProposal {
                status: proto::response_process_proposal::ProposalStatus::Reject.into(),
                ..Default::default()
            };
            Ok(response)
        } else {
            let BlockExecutionOutcome {
                app_hash,
                tx_results,
            } = run_result.into_data().map_err(Error::Protocol)?;

            // TODO: implement all fields, including tx processing; for now, just leaving bare minimum
            let response = ResponseProcessProposal {
                app_hash: app_hash.to_vec(),
                tx_results,
                status: proto::response_process_proposal::ProposalStatus::Accept.into(),
                ..Default::default()
            };
            Ok(response)
        }
    }

    fn extend_vote(
        &self,
        request: proto::RequestExtendVote,
    ) -> Result<proto::ResponseExtendVote, proto::ResponseException> {
        let transaction_guard = self.transaction.read().unwrap();
        let transaction = transaction_guard.as_ref().unwrap();

        self.must_match_vote_info(request.height, request.round, request.hash)?;

        let withdrawals = WithdrawalTxs::load(Some(transaction), &self.platform.drive)?;

        Ok(proto::ResponseExtendVote {
            vote_extensions: withdrawals.to_vec(),
        })
    }

    fn verify_vote_extension(
        &self,
        request: proto::RequestVerifyVoteExtension,
    ) -> Result<proto::ResponseVerifyVoteExtension, proto::ResponseException> {
        let transaction_guard = self.transaction.read().unwrap();
        let transaction = transaction_guard.as_ref().unwrap();

        self.must_match_vote_info(request.height, request.round, request.hash)?;

        let got: WithdrawalTxs = request.vote_extensions.into();
        let expected = WithdrawalTxs::load(Some(transaction), &self.platform.drive)?;

        if let Err(err) = self.platform.check_withdrawals(&got, &expected, None) {
            tracing::error!(
                method = "verify_vote_extension",
                ?got,
                ?expected,
                ?err,
                "vote extension mismatch"
            );
            Ok(proto::ResponseVerifyVoteExtension {
                status: VerifyStatus::Reject.into(),
            })
        } else {
            Ok(proto::ResponseVerifyVoteExtension {
                status: VerifyStatus::Accept.into(),
            })
        }
    }

    fn finalize_block(
        &self,
        request: RequestFinalizeBlock,
    ) -> Result<ResponseFinalizeBlock, ResponseException> {
        let transaction_guard = self.transaction.read().unwrap();

        let transaction = transaction_guard.as_ref().ok_or(Error::Execution(
            ExecutionError::NotInTransaction(
                "trying to finalize block without a current transaction",
            ),
        ))?;

        let block_finalization_outcome = self
            .platform
            .finalize_block_proposal(request, transaction)?;

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

    fn check_tx(&self, request: RequestCheckTx) -> Result<ResponseCheckTx, ResponseException> {
        let RequestCheckTx { tx, r#type } = request;
        let validation_result = self.platform.check_tx(tx)?;

        // If there are no execution errors the code will be 0
        let code = validation_result
            .errors
            .first()
            .map(|error| error.code())
            .unwrap_or_default();
        let gas_wanted = validation_result
            .data
            .map(|fee_result| fee_result.total_base_fee())
            .unwrap_or_default();
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

    fn query(&self, request: RequestQuery) -> Result<ResponseQuery, ResponseException> {
        let RequestQuery {
            data,
            path,
            height,
            prove,
        } = request;

        let data = self
            .platform
            .drive
            .query_serialized(data, path, prove)
            .map_err(Error::Drive)?;

        Ok(ResponseQuery {
            //todo: right now just put GRPC error codes,
            //  later we will use own error codes
            code: 0,
            log: "".to_string(),
            info: "".to_string(),
            index: 0,
            key: vec![],
            value: data,
            proof_ops: None,
            height,
            codespace: "".to_string(),
        })
    }
}
//
// #[cfg(test)]
// mod tests {
//     mod handlers {
//         use crate::config::PlatformConfig;
//         use crate::rpc::core::MockCoreRPCLike;
//         use chrono::{Duration, Utc};
//         use dashcore::hashes::hex::FromHex;
//         use dashcore::BlockHash;
//         use dpp::contracts::withdrawals_contract;
//         use dpp::data_contract::DriveContractExt;
//         use dpp::identity::core_script::CoreScript;
//         use dpp::identity::state_transition::identity_credit_withdrawal_transition::Pooling;
//         use dpp::platform_value::{platform_value, BinaryData};
//         use dpp::prelude::Identifier;
//         use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
//         use dpp::tests::fixtures::get_withdrawal_document_fixture;
//         use dpp::util::hash;
//         use drive::common::helpers::identities::create_test_masternode_identities;
//         use drive::drive::block_info::BlockInfo;
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
//                     .document_type_for_name(withdrawals_contract::document_types::WITHDRAWAL)
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
//                 epoch: Epoch::new(1),
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
impl<'a, C> AbciApplication<'a, C> {
    /// Check if current state (round/height/hash) matches received message.
    fn must_match_vote_info(
        &self,
        height: i64,
        round: i32,
        block_hash: Vec<u8>,
    ) -> Result<(), Error> {
        let guarded_block_execution_context = self.platform.block_execution_context.read().unwrap();
        let block_execution_context =
            guarded_block_execution_context
                .as_ref()
                .ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "block execution context must be set in block begin handler",
                )))?;

        let block_state_info = &block_execution_context.block_state_info;

        if !block_state_info.matches_current_block(height as u64, round as u32, block_hash)? {
            Err(Error::from(AbciError::RequestForWrongBlockReceived(
                format!(
                    "received request for height: {} rount: {}, expected height: {} round: {}",
                    height, round, block_state_info.height, block_state_info.round
                ),
            )))
        } else {
            Ok(())
        }
    }
}
