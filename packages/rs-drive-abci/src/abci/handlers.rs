use std::ops::Deref;

use crate::abci::messages::{
    BlockBeginRequest, BlockBeginResponse, BlockEndRequest, BlockEndResponse, InitChainRequest,
    InitChainResponse,
};
use crate::block::{BlockExecutionContext, BlockInfo};
use crate::execution::fee_pools::epoch::EpochInfo;
use rs_drive::grovedb::TransactionArg;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform::Platform;

pub trait TenderdashAbci {
    fn init_chain(
        &self,
        request: InitChainRequest,
        transaction: TransactionArg,
    ) -> Result<InitChainResponse, Error>;

    fn block_begin(
        &self,
        request: BlockBeginRequest,
        transaction: TransactionArg,
    ) -> Result<BlockBeginResponse, Error>;

    fn block_end(
        &self,
        request: BlockEndRequest,
        transaction: TransactionArg,
    ) -> Result<BlockEndResponse, Error>;
}

impl TenderdashAbci for Platform {
    fn init_chain(
        &self,
        _request: InitChainRequest,
        transaction: TransactionArg,
    ) -> Result<InitChainResponse, Error> {
        self.drive
            .create_initial_state_structure(transaction)
            .map_err(Error::Drive)?;

        let response = InitChainResponse {};

        Ok(response)
    }

    fn block_begin(
        &self,
        request: BlockBeginRequest,
        transaction: TransactionArg,
    ) -> Result<BlockBeginResponse, Error> {
        // Set genesis time
        let genesis_time_ms = if request.block_height == 1 {
            self.drive
                .init_genesis_time(request.block_time_ms, transaction)?;
            request.block_time_ms
        } else {
            self.drive
                .get_genesis_time(transaction)
                .map_err(Error::Drive)?
                .ok_or(Error::Execution(ExecutionError::DriveIncoherence(
                    "the genesis time must be set",
                )))?
        };

        // Init block execution context
        let block_info = BlockInfo::from_block_begin_request(&request);

        let epoch_info = EpochInfo::from_genesis_time_and_block_info(genesis_time_ms, &block_info)?;

        let block_execution_context = BlockExecutionContext {
            block_info,
            epoch_info,
        };

        self.block_execution_context
            .replace(Some(block_execution_context));

        let response = BlockBeginResponse {};

        Ok(response)
    }

    fn block_end(
        &self,
        request: BlockEndRequest,
        transaction: TransactionArg,
    ) -> Result<BlockEndResponse, Error> {
        // Retrieve block execution context
        let block_execution_context = self.block_execution_context.borrow();
        let block_execution_context = match block_execution_context.deref() {
            Some(block_execution_context) => block_execution_context,
            None => {
                return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "block execution context must be set in block begin handler",
                )))
            }
        };

        // Process fees
        let process_block_fees_result = self.process_block_fees(
            &block_execution_context.block_info,
            &block_execution_context.epoch_info,
            &request.fees,
            transaction,
        )?;

        Ok(
            BlockEndResponse::from_epoch_info_and_process_block_fees_result(
                &block_execution_context.epoch_info,
                &process_block_fees_result,
            ),
        )
    }
}

#[cfg(test)]
mod tests {
    mod handlers {
        use crate::abci::handlers::TenderdashAbci;
        use crate::common::helpers::fee_pools::create_test_masternode_share_identities_and_documents;
        use chrono::{Duration, Utc};
        use rs_drive::common::helpers::identities::create_test_masternode_identities;
        use rust_decimal::prelude::ToPrimitive;
        use std::ops::Div;

        use crate::abci::messages::{
            BlockBeginRequest, BlockEndRequest, FeesAggregate, InitChainRequest,
        };
        use crate::common::helpers::setup::setup_platform;

        #[test]
        fn test_abci_flow() {
            let platform = setup_platform();
            let transaction = platform.drive.grove.start_transaction();

            // init chain
            let init_chain_request = InitChainRequest {};

            platform
                .init_chain(init_chain_request, Some(&transaction))
                .expect("should init chain");

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
                Some(&transaction),
            );

            create_test_masternode_share_identities_and_documents(
                &platform.drive,
                &contract,
                &proposers,
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
                        proposer_pro_tx_hash: proposers
                            [block_height as usize % (proposers_count as usize)],
                    };

                    platform
                        .block_begin(block_begin_request, Some(&transaction))
                        .expect(
                            format!(
                                "should begin process block #{} for day #{}",
                                block_height, day
                            )
                            .as_str(),
                        );

                    let block_end_request = BlockEndRequest {
                        fees: FeesAggregate {
                            processing_fees: 1600,
                            storage_fees: storage_fees_per_block,
                        },
                    };

                    let block_end_response = platform
                        .block_end(block_end_request, Some(&transaction))
                        .expect(
                            format!(
                                "should end process block #{} for day #{}",
                                block_height, day
                            )
                            .as_str(),
                        );

                    // Set previous block time
                    previous_block_time_ms = Some(block_time_ms);

                    // Should calculate correct current epochs
                    let (epoch_index, epoch_change) = if day > epoch_1_start_day {
                        (1, false)
                    } else if day == epoch_1_start_day {
                        if block_num < epoch_1_start_block {
                            (0, false)
                        } else if block_num == epoch_1_start_block {
                            (1, true)
                        } else {
                            (1, false)
                        }
                    } else if day == 0 && block_num == 0 {
                        (0, true)
                    } else {
                        (0, false)
                    };

                    assert_eq!(block_end_response.current_epoch_index, epoch_index);

                    assert_eq!(block_end_response.is_epoch_change, epoch_change);

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

            let platform = setup_platform();
            let transaction = platform.drive.grove.start_transaction();

            // init chain
            let init_chain_request = InitChainRequest {};

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
                Some(&transaction),
            );

            create_test_masternode_share_identities_and_documents(
                &platform.drive,
                &contract,
                &proposers,
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
                        proposer_pro_tx_hash: proposers
                            [block_height as usize % (proposers_count as usize)],
                    };

                    platform
                        .block_begin(block_begin_request, Some(&transaction))
                        .expect(
                            format!(
                                "should begin process block #{} for day #{}",
                                block_height, day
                            )
                            .as_str(),
                        );

                    let block_end_request = BlockEndRequest {
                        fees: FeesAggregate {
                            processing_fees: 1600,
                            storage_fees: storage_fees_per_block,
                        },
                    };

                    let block_end_response = platform
                        .block_end(block_end_request, Some(&transaction))
                        .expect(
                            format!(
                                "should end process block #{} for day #{}",
                                block_height, day
                            )
                            .as_str(),
                        );

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

                    assert_eq!(block_end_response.current_epoch_index, epoch_index);

                    assert_eq!(block_end_response.is_epoch_change, epoch_change);

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
