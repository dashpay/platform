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

use crate::abci::messages::{
    AfterFinalizeBlockRequest, AfterFinalizeBlockResponse, BlockBeginRequest, BlockBeginResponse,
    BlockEndRequest, BlockEndResponse, InitChainRequest, InitChainResponse,
};
use crate::block::{BlockExecutionContext, BlockStateInfo};
use crate::execution::fee_pools::epoch::EpochInfo;
use drive::grovedb::TransactionArg;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform::Platform;

/// A trait for handling the Tenderdash ABCI (Application Blockchain Interface).
pub trait TenderdashAbci {
    /// Called with JS drive on init chain
    fn init_chain(
        &self,
        request: InitChainRequest,
        transaction: TransactionArg,
    ) -> Result<InitChainResponse, Error>;

    /// Called with JS Drive on block begin
    fn block_begin(
        &self,
        request: BlockBeginRequest,
        transaction: TransactionArg,
    ) -> Result<BlockBeginResponse, Error>;

    /// Called with JS Drive on block end
    fn block_end(
        &self,
        request: BlockEndRequest,
        transaction: TransactionArg,
    ) -> Result<BlockEndResponse, Error>;

    /// Called with JS Drive after the current block db transaction is committed
    fn after_finalize_block(
        &self,
        request: AfterFinalizeBlockRequest,
    ) -> Result<AfterFinalizeBlockResponse, Error>;
}

impl TenderdashAbci for Platform {
    /// Creates initial state structure and returns response
    fn init_chain(
        &self,
        request: InitChainRequest,
        transaction: TransactionArg,
    ) -> Result<InitChainResponse, Error> {
        self.create_genesis_state(
            request.genesis_time_ms,
            request.system_identity_public_keys,
            transaction,
        )?;

        let response = InitChainResponse {};

        Ok(response)
    }

    /// Set genesis time, block info, and epoch info, and returns response
    fn block_begin(
        &self,
        request: BlockBeginRequest,
        transaction: TransactionArg,
    ) -> Result<BlockBeginResponse, Error> {
        // TODO: If genesis time is not set in genesis config then it set on the first block
        //  which is great but we still need time on init chain. Having two genesis times is not great at all.

        // Set genesis time
        let genesis_time_ms = if request.block_height == 1 {
            self.drive.set_genesis_time(request.block_time_ms);
            request.block_time_ms
        } else {
            //todo: lazy load genesis time
            self.drive
                .get_genesis_time(transaction)
                .map_err(Error::Drive)?
                .ok_or(Error::Execution(ExecutionError::DriveIncoherence(
                    "the genesis time must be set",
                )))?
        };

        // Update versions
        let proposed_app_version = request.proposed_app_version;

        self.drive.update_validator_proposed_app_version(
            request.proposer_pro_tx_hash,
            proposed_app_version,
            transaction,
        )?;

        // Init block execution context
        let block_info = BlockStateInfo::from_block_begin_request(&request);

        let epoch_info = EpochInfo::from_genesis_time_and_block_info(genesis_time_ms, &block_info)?;

        let block_execution_context = BlockExecutionContext {
            block_info,
            epoch_info: epoch_info.clone(),
            hpmn_count: request.total_hpmns,
        };

        // If last synced Core block height is not set instead of scanning
        // number of blocks for asset unlock transactions scan only one
        // on Core chain locked height by setting last_synced_core_height to the same value
        let _last_synced_core_height = if request.last_synced_core_height == 0 {
            block_execution_context.block_info.core_chain_locked_height
        } else {
            request.last_synced_core_height
        };

        self.block_execution_context
            .replace(Some(block_execution_context));

        // TODO: This code is not stable and blocking WASM-DPP integration and v0.24 testing
        //   Must be enabled and accomplished when we come back to withdrawals
        // self.update_broadcasted_withdrawal_transaction_statuses(
        //     last_synced_core_height,
        //     transaction,
        // )?;

        let unsigned_withdrawal_transaction_bytes = self
            .fetch_and_prepare_unsigned_withdrawal_transactions(
                request.validator_set_quorum_hash,
                transaction,
            )?;

        let response = BlockBeginResponse {
            epoch_info,
            unsigned_withdrawal_transactions: unsigned_withdrawal_transaction_bytes,
        };

        Ok(response)
    }

    /// Processes block fees and returns response
    fn block_end(
        &self,
        request: BlockEndRequest,
        transaction: TransactionArg,
    ) -> Result<BlockEndResponse, Error> {
        // Retrieve block execution context
        let block_execution_context = self.block_execution_context.borrow();
        let block_execution_context = block_execution_context.as_ref().ok_or(Error::Execution(
            ExecutionError::CorruptedCodeExecution(
                "block execution context must be set in block begin handler",
            ),
        ))?;

        self.pool_withdrawals_into_transactions_queue(transaction)?;

        // Process fees
        let process_block_fees_outcome = self.process_block_fees(
            &block_execution_context.block_info,
            &block_execution_context.epoch_info,
            request.fees,
            transaction,
        )?;

        // Determine a new protocol version if enough proposers voted
        let changed_protocol_version = if block_execution_context
            .epoch_info
            .is_epoch_change_but_not_genesis()
        {
            // Set current protocol version to the version from upcoming epoch
            self.state.replace_with(|state| {
                state.current_protocol_version_in_consensus = state.next_epoch_protocol_version;

                state.clone()
            });

            // Determine new protocol version based on votes for the next epoch
            let maybe_new_protocol_version = self.check_for_desired_protocol_upgrade(
                block_execution_context.hpmn_count,
                transaction,
            )?;

            self.state.replace_with(|state| {
                if let Some(new_protocol_version) = maybe_new_protocol_version {
                    state.next_epoch_protocol_version = new_protocol_version;
                } else {
                    state.next_epoch_protocol_version = state.current_protocol_version_in_consensus;
                }
                state.clone()
            });

            Some(self.state.borrow().current_protocol_version_in_consensus)
        } else {
            None
        };

        Ok(BlockEndResponse::from_outcomes(
            &process_block_fees_outcome,
            changed_protocol_version,
        ))
    }

    fn after_finalize_block(
        &self,
        _: AfterFinalizeBlockRequest,
    ) -> Result<AfterFinalizeBlockResponse, Error> {
        let mut drive_cache = self.drive.cache.borrow_mut();

        drive_cache.cached_contracts.clear_block_cache();

        Ok(AfterFinalizeBlockResponse {})
    }
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
        use dpp::identity::state_transition::identity_credit_withdrawal_transition::Pooling;
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
        use crate::test::helpers::setup::setup_platform_raw;

        // TODO: Should we remove this test in favor of strategy tests?

        #[test]
        fn test_abci_flow() {
            let mut platform = setup_platform_raw(Some(PlatformConfig {
                verify_sum_trees: false,
                ..Default::default()
            }));

            let mut core_rpc_mock = MockCoreRPCLike::new();

            let transaction = platform.drive.grove.start_transaction();

            // init chain
            let init_chain_request = static_init_chain_request();

            platform
                .init_chain(init_chain_request, Some(&transaction))
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
                    json!({
                        "amount": 1000,
                        "coreFeePerByte": 1,
                        "pooling": Pooling::Never,
                        "outputScript": (0..23).collect::<Vec<u8>>(),
                        "status": withdrawals_contract::WithdrawalStatus::POOLED,
                        "transactionIndex": 1,
                        "transactionSignHeight": 93,
                        "transactionId": tx_id,
                    }),
                );

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

            platform.core_rpc = Box::new(core_rpc_mock);

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
                        .block_begin(block_begin_request, Some(&transaction))
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

            let mut platform = setup_platform_raw(Some(PlatformConfig {
                verify_sum_trees: false,
                ..Default::default()
            }));

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

            platform.core_rpc = Box::new(core_rpc_mock);

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
