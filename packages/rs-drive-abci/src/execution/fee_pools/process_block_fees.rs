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

//! Block Fees Processing.
//!
//! This modules defines functions related to processing block fees upon block and
//! epoch changes.
//!

use crate::abci::messages::FeesAggregate;
use crate::block::BlockInfo;
use crate::error::Error;
use crate::execution::fee_pools::constants::DEFAULT_ORIGINAL_FEE_MULTIPLIER;
use crate::execution::fee_pools::distribute_storage_pool::StorageDistributionLeftoverCredits;
use crate::execution::fee_pools::epoch::EpochInfo;
use crate::execution::fee_pools::fee_distribution::{FeesInPools, ProposersPayouts};
use crate::platform::Platform;
use rs_drive::drive::batch::GroveDbOpBatch;
use rs_drive::drive::fee_pools::epochs::constants::{
    GENESIS_EPOCH_INDEX, PERPETUAL_STORAGE_EPOCHS,
};
use rs_drive::fee_pools::epochs::Epoch;
use rs_drive::grovedb::TransactionArg;
use std::option::Option::None;

/// From the Dash Improvement Proposal:

/// For the purpose of this explanation we can trivialize that the execution of a block comprises
/// the sum of the execution of all state transitions contained within the block. In order to
/// avoid altering participating masternode identity balances every block and distribute fees
/// evenly, the concept of pools is introduced. We will also introduce the concepts of an Epoch
/// and the Epoch Year that are both covered later in this document. As the block executes state
/// transitions, processing and storage fees are accumulated, as well as a list of refunded fees
/// from various Epochs and fee multipliers. When there are no more state transitions to execute
/// we can say the block has ended its state transition execution phase. The system will then add
/// the accumulated fees to their corresponding pools, and in the case of deletion of data, remove
/// storage fees from future Epoch storage pools.

/// Holds info relevant fees and a processed block
pub struct ProcessedBlockFeesResult {
    /// Amount of fees in the storage and processing fee distribution pools
    pub fees_in_pools: FeesInPools,
    /// A struct with the number of proposers to be paid out and the last paid epoch index
    pub payouts: Option<ProposersPayouts>,
}

impl Platform {
    /// Adds operations to the GroveDB batch which initialize the current epoch
    /// as well as the current+1000 epoch, then distributes storage fees accumulated
    /// during the previous epoch.
    ///
    /// `StorageDistributionLeftoverCredits` will be returned, except if we are at Genesis Epoch.
    fn add_process_epoch_change_operations(
        &self,
        block_info: &BlockInfo,
        epoch_info: &EpochInfo,
        transaction: TransactionArg,
        batch: &mut GroveDbOpBatch,
    ) -> Result<Option<StorageDistributionLeftoverCredits>, Error> {
        // init next thousandth empty epochs since last initiated
        let last_initiated_epoch_index = epoch_info
            .previous_epoch_index
            .map_or(GENESIS_EPOCH_INDEX, |i| i + 1);

        for epoch_index in last_initiated_epoch_index..=epoch_info.current_epoch_index {
            let next_thousandth_epoch = Epoch::new(epoch_index + PERPETUAL_STORAGE_EPOCHS);
            next_thousandth_epoch.add_init_empty_without_storage_operations(batch);
        }

        // init current epoch pool for processing
        let current_epoch = Epoch::new(epoch_info.current_epoch_index);

        current_epoch.add_init_current_operations(
            DEFAULT_ORIGINAL_FEE_MULTIPLIER, // TODO use a data contract to choose the fee multiplier
            block_info.block_height,
            block_info.block_time_ms,
            batch,
        );

        // Nothing to distribute on genesis epoch start
        if current_epoch.index == GENESIS_EPOCH_INDEX {
            return Ok(None);
        }

        // Distribute storage fees accumulated during previous epoch
        let storage_distribution_leftover_credits = self
            .add_distribute_storage_fee_distribution_pool_to_epochs_operations(
                current_epoch.index,
                transaction,
                batch,
            )?;

        Ok(Some(storage_distribution_leftover_credits))
    }

    /// Adds operations to GroveDB op batch related to processing
    /// and distributing the block fees from the previous block and applies the batch.
    ///
    /// Returns `ProcessedBlockFeesResult`.
    pub fn process_block_fees(
        &self,
        block_info: &BlockInfo,
        epoch_info: &EpochInfo,
        block_fees: &FeesAggregate,
        transaction: TransactionArg,
    ) -> Result<ProcessedBlockFeesResult, Error> {
        let current_epoch = Epoch::new(epoch_info.current_epoch_index);

        let mut batch = GroveDbOpBatch::new();

        let storage_distribution_leftover_credits = if epoch_info.is_epoch_change {
            self.add_process_epoch_change_operations(
                block_info,
                epoch_info,
                transaction,
                &mut batch,
            )?
        } else {
            None
        };

        // Since epoch pool tree batched is not committed yet
        // we pass previous block count explicitly
        let cached_previous_block_count = if epoch_info.is_epoch_change {
            Some(0)
        } else {
            None
        };

        batch.push(current_epoch.increment_proposer_block_count_operation(
            &self.drive,
            &block_info.proposer_pro_tx_hash,
            cached_previous_block_count,
            transaction,
        )?);

        // Distribute fees from unpaid epoch pool to proposers

        // Since start_block_height for current epoch is batched and not committed yet
        // we pass it explicitly
        let cached_current_epoch_start_block_height = if epoch_info.is_epoch_change {
            Some(block_info.block_height)
        } else {
            None
        };

        let payouts = self
            .add_distribute_fees_from_oldest_unpaid_epoch_pool_to_proposers_operations(
                epoch_info.current_epoch_index,
                cached_current_epoch_start_block_height,
                transaction,
                &mut batch,
            )?;

        let fees_in_pools = self.add_distribute_block_fees_into_pools_operations(
            &current_epoch,
            block_fees,
            // Add leftovers after storage fee pool distribution to the current block storage fees
            storage_distribution_leftover_credits,
            transaction,
            &mut batch,
        )?;

        self.drive.grove_apply_batch(batch, false, transaction)?;

        Ok(ProcessedBlockFeesResult {
            fees_in_pools,
            payouts,
        })
    }
}

#[cfg(test)]
mod tests {
    mod add_process_epoch_change_operations {
        use crate::common::helpers::setup::setup_platform_with_initial_state_structure;
        use chrono::Utc;
        use rs_drive::drive::fee_pools::epochs::constants::GENESIS_EPOCH_INDEX;
        use rust_decimal::prelude::ToPrimitive;

        mod helpers {
            use crate::abci::messages::FeesAggregate;
            use crate::block::BlockInfo;
            use crate::execution::fee_pools::epoch::{EpochInfo, EPOCH_CHANGE_TIME_MS};
            use crate::platform::Platform;
            use rs_drive::drive::batch::GroveDbOpBatch;
            use rs_drive::drive::fee_pools::epochs::constants::PERPETUAL_STORAGE_EPOCHS;
            use rs_drive::fee_pools::epochs::Epoch;
            use rs_drive::grovedb::TransactionArg;

            /// Process and validate an epoch change
            pub fn process_and_validate_epoch_change(
                platform: &Platform,
                genesis_time_ms: u64,
                epoch_index: u16,
                block_height: u64,
                previous_block_time_ms: Option<u64>,
                should_distribute: bool,
                transaction: TransactionArg,
            ) -> BlockInfo {
                let current_epoch = Epoch::new(epoch_index);

                // Add some storage fees to distribute next time
                if should_distribute {
                    let block_fees = FeesAggregate {
                        processing_fees: 1000,
                        storage_fees: 1000000000,
                    };

                    let mut batch = GroveDbOpBatch::new();

                    platform
                        .add_distribute_block_fees_into_pools_operations(
                            &current_epoch,
                            &block_fees,
                            None,
                            transaction,
                            &mut batch,
                        )
                        .expect("should add distribute block fees into pools operations");

                    platform
                        .drive
                        .grove_apply_batch(batch, false, transaction)
                        .expect("should apply batch");
                }

                let proposer_pro_tx_hash: [u8; 32] = [
                    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
                    1, 1, 1, 1, 1, 1,
                ];

                let block_time_ms = genesis_time_ms + epoch_index as u64 * EPOCH_CHANGE_TIME_MS;

                let block_info = BlockInfo {
                    block_height,
                    block_time_ms,
                    previous_block_time_ms,
                    proposer_pro_tx_hash,
                };

                let epoch_info =
                    EpochInfo::from_genesis_time_and_block_info(genesis_time_ms, &block_info)
                        .expect("should calculate epoch info");

                let mut batch = GroveDbOpBatch::new();

                let distribute_storage_pool_result = platform
                    .add_process_epoch_change_operations(
                        &block_info,
                        &epoch_info,
                        transaction,
                        &mut batch,
                    )
                    .expect("should process epoch");

                platform
                    .drive
                    .grove_apply_batch(batch, false, transaction)
                    .expect("should apply batch");

                // Next thousandth epoch should be created
                let next_thousandth_epoch = Epoch::new(epoch_index + PERPETUAL_STORAGE_EPOCHS);

                let is_epoch_tree_exists = platform
                    .drive
                    .is_epoch_tree_exists(&next_thousandth_epoch, transaction)
                    .expect("should check epoch tree existence");

                assert!(is_epoch_tree_exists);

                // epoch should be initialized as current
                let epoch_start_block_height = platform
                    .drive
                    .get_epoch_start_block_height(&current_epoch, transaction)
                    .expect("should get start block time from start epoch");

                assert_eq!(epoch_start_block_height, block_height);

                // storage fee should be distributed
                assert_eq!(distribute_storage_pool_result.is_some(), should_distribute);

                let thousandth_epoch = Epoch::new(next_thousandth_epoch.index - 1);

                let aggregate_storage_fees = platform
                    .drive
                    .get_epoch_storage_credits_for_distribution(&thousandth_epoch, transaction)
                    .expect("should get epoch storage fees");

                if should_distribute {
                    assert_ne!(aggregate_storage_fees, 0);
                } else {
                    assert_eq!(aggregate_storage_fees, 0);
                }

                block_info
            }
        }

        #[test]
        fn test_processing_epoch_change_for_epoch_0_1_and_4() {
            let platform = setup_platform_with_initial_state_structure();
            let transaction = platform.drive.grove.start_transaction();

            let genesis_time_ms = Utc::now()
                .timestamp_millis()
                .to_u64()
                .expect("block time can not be before 1970");

            /*
            Process genesis

            Storage fees shouldn't be distributed
             */

            let epoch_index = GENESIS_EPOCH_INDEX;
            let block_height = 1;

            let block_info = helpers::process_and_validate_epoch_change(
                &platform,
                genesis_time_ms,
                epoch_index,
                block_height,
                None,
                false,
                Some(&transaction),
            );

            /*
            Process epoch 1

            Storage fees should be distributed
             */

            let epoch_index = 1;
            let block_height = 2;

            let block_info = helpers::process_and_validate_epoch_change(
                &platform,
                genesis_time_ms,
                epoch_index,
                block_height,
                Some(block_info.block_time_ms),
                true,
                Some(&transaction),
            );

            /*
            Process epoch 4

            Multiple next empty epochs must be initialized and fees must be distributed
             */

            let epoch_index = 4;
            let block_height = 3;

            helpers::process_and_validate_epoch_change(
                &platform,
                genesis_time_ms,
                epoch_index,
                block_height,
                Some(block_info.block_time_ms),
                true,
                Some(&transaction),
            );
        }
    }

    mod process_block_fees {
        use crate::common::helpers::setup::setup_platform_with_initial_state_structure;
        use chrono::Utc;
        use rs_drive::common::helpers::identities::create_test_masternode_identities;
        use rs_drive::drive::fee_pools::epochs::constants::GENESIS_EPOCH_INDEX;
        use rust_decimal::prelude::ToPrimitive;

        mod helpers {
            use crate::abci::messages::FeesAggregate;
            use crate::block::BlockInfo;
            use crate::execution::fee_pools::epoch::{EpochInfo, EPOCH_CHANGE_TIME_MS};
            use crate::platform::Platform;
            use rs_drive::drive::fee_pools::epochs::constants::GENESIS_EPOCH_INDEX;
            use rs_drive::fee_pools::epochs::Epoch;
            use rs_drive::grovedb::TransactionArg;

            /// Process and validate block fees
            pub fn process_and_validate_block_fees(
                platform: &Platform,
                genesis_time_ms: u64,
                epoch_index: u16,
                block_height: u64,
                previous_block_time_ms: Option<u64>,
                proposer_pro_tx_hash: [u8; 32],
                transaction: TransactionArg,
            ) -> BlockInfo {
                let current_epoch = Epoch::new(epoch_index);

                let block_time_ms =
                    genesis_time_ms + epoch_index as u64 * EPOCH_CHANGE_TIME_MS + block_height;

                let block_info = BlockInfo {
                    block_height,
                    block_time_ms,
                    previous_block_time_ms,
                    proposer_pro_tx_hash,
                };

                let epoch_info =
                    EpochInfo::from_genesis_time_and_block_info(genesis_time_ms, &block_info)
                        .expect("should calculate epoch info");

                let block_fees = FeesAggregate {
                    processing_fees: 1000,
                    storage_fees: 10000,
                };

                let distribute_storage_pool_result = platform
                    .process_block_fees(&block_info, &epoch_info, &block_fees, transaction)
                    .expect("should process block fees");

                // Should process epoch change
                // and distribute aggregated storage fees into pools on epoch > 0

                let aggregated_storage_fees = platform
                    .drive
                    .get_aggregate_storage_fees_from_distribution_pool(transaction)
                    .expect("should get storage fees from distribution pool");

                if epoch_info.is_epoch_change {
                    if epoch_info.current_epoch_index == GENESIS_EPOCH_INDEX {
                        assert_eq!(aggregated_storage_fees, block_fees.storage_fees);
                    } else {
                        // Assuming leftovers
                        assert!(
                            block_fees.storage_fees <= aggregated_storage_fees
                                && aggregated_storage_fees < block_fees.storage_fees + 1000
                        );
                    };
                } else {
                    assert!(aggregated_storage_fees > block_fees.storage_fees);
                }

                // Should increment proposer block count

                let proposers_block_count = platform
                    .drive
                    .get_epochs_proposer_block_count(
                        &current_epoch,
                        &proposer_pro_tx_hash,
                        transaction,
                    )
                    .expect("should get proposers");

                assert_ne!(proposers_block_count, 0);

                // Should pay for previous epoch

                if epoch_info.is_epoch_change && epoch_index > GENESIS_EPOCH_INDEX {
                    assert!(distribute_storage_pool_result.payouts.is_some());
                } else {
                    assert!(distribute_storage_pool_result.payouts.is_none());
                }

                // Should distribute block fees into pools

                let processing_fees = platform
                    .drive
                    .get_epoch_processing_credits_for_distribution(&current_epoch, transaction)
                    .expect("should get processing credits");

                assert_ne!(processing_fees, 0);

                block_info
            }
        }

        #[test]
        fn test_process_3_block_fees_from_different_epochs() {
            let platform = setup_platform_with_initial_state_structure();
            let transaction = platform.drive.grove.start_transaction();

            platform.create_mn_shares_contract(Some(&transaction));

            let proposers =
                create_test_masternode_identities(&platform.drive, 6, Some(&transaction));

            let genesis_time_ms = Utc::now()
                .timestamp_millis()
                .to_u64()
                .expect("block time can not be before 1970");

            /*
            Process first block of epoch 0 (genesis epoch)

            Should change epoch to 0
            Should not pay to proposers
             */

            let epoch_index = GENESIS_EPOCH_INDEX;
            let block_height = 1;

            let block_info = helpers::process_and_validate_block_fees(
                &platform,
                genesis_time_ms,
                epoch_index,
                block_height,
                None,
                proposers[0],
                Some(&transaction),
            );

            /*
            Process second block of epoch 0

            Should not change epoch
            Should not pay to proposers
             */

            let epoch_index = GENESIS_EPOCH_INDEX;
            let block_height = 2;

            let block_info = helpers::process_and_validate_block_fees(
                &platform,
                genesis_time_ms,
                epoch_index,
                block_height,
                Some(block_info.block_time_ms),
                proposers[1],
                Some(&transaction),
            );

            /*
            Process first block of epoch 1

            Should change epoch to 1
            Should pay to proposers from epoch 0
             */

            let epoch_index = 1;
            let block_height = 3;

            let block_info = helpers::process_and_validate_block_fees(
                &platform,
                genesis_time_ms,
                epoch_index,
                block_height,
                Some(block_info.block_time_ms),
                proposers[2],
                Some(&transaction),
            );

            /*
            Process second block of epoch 1

            Should change epoch to 1
            Should not pay to proposers 0
             */

            let epoch_index = 1;
            let block_height = 4;

            let block_info = helpers::process_and_validate_block_fees(
                &platform,
                genesis_time_ms,
                epoch_index,
                block_height,
                Some(block_info.block_time_ms),
                proposers[3],
                Some(&transaction),
            );

            /*
            Process first block of epoch 3, skipping epoch 2 (i.e. chain halt)

            Should change epoch to
            Should pay to proposers for epoch 1
             */

            let epoch_index = 3;
            let block_height = 5;

            let block_info = helpers::process_and_validate_block_fees(
                &platform,
                genesis_time_ms,
                epoch_index,
                block_height,
                Some(block_info.block_time_ms),
                proposers[3],
                Some(&transaction),
            );

            /*
            Process second block of epoch 3

            Should not change epoch to
            Should not pay to proposers
             */

            let epoch_index = 3;
            let block_height = 6;

            helpers::process_and_validate_block_fees(
                &platform,
                genesis_time_ms,
                epoch_index,
                block_height,
                Some(block_info.block_time_ms),
                proposers[4],
                Some(&transaction),
            );
        }
    }
}
