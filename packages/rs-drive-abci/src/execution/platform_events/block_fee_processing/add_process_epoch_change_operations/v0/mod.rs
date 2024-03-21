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

use std::option::Option::None;

use dpp::block::epoch::Epoch;
use dpp::fee::epoch::{perpetual_storage_epochs, GENESIS_EPOCH_INDEX};
use dpp::fee::DEFAULT_ORIGINAL_FEE_MULTIPLIER;
use dpp::version::PlatformVersion;
use drive::drive::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
use drive::drive::batch::{DriveOperation, GroveDbOpBatch};
use drive::grovedb::Transaction;

use crate::error::Error;
use crate::execution::types::block_fees::v0::BlockFeesV0Getters;
use crate::execution::types::block_fees::BlockFees;
use crate::execution::types::block_state_info::v0::BlockStateInfoV0Getters;
use crate::execution::types::block_state_info::BlockStateInfo;
use crate::execution::types::storage_fee_distribution_outcome;

use crate::platform_types::epoch_info::v0::EpochInfoV0Getters;
use crate::platform_types::epoch_info::EpochInfo;
use crate::platform_types::platform::Platform;
use drive::fee_pools::epochs::operations_factory::EpochOperations;

/// From the Dash Improvement Proposal:

/// For the purpose of this explanation we can trivialize that the execution of a block comprises
/// the sum of the execution of all state transitions contained within the block. In order to
/// avoid altering participating masternode identity balances every block and distribute fees
/// evenly, the concept of pools is introduced. We will also introduce the concepts of an Epoch
/// and the Epoch Era that are both covered later in this document. As the block executes state
/// transitions, processing and storage fees are accumulated, as well as a list of refunded fees
/// from various Epochs and fee multipliers. When there are no more state transitions to execute
/// we can say the block has ended its state transition execution phase. The system will then add
/// the accumulated fees to their corresponding pools, and in the case of deletion of data, remove
/// storage fees from future Epoch storage pools.

impl<CoreRPCLike> Platform<CoreRPCLike> {
    /// Adds operations to the GroveDB batch which initialize the current epoch
    /// as well as the current+total epochs (40*50 for mainnet) epoch, then distributes storage fees
    /// accumulated during the previous epoch.
    ///
    /// `DistributionLeftoverCredits` will be returned, except if we are at Genesis Epoch.
    #[inline(always)]
    pub(super) fn add_process_epoch_change_operations_v0(
        &self,
        block_info: &BlockStateInfo,
        epoch_info: &EpochInfo,
        block_fees: &BlockFees,
        transaction: &Transaction,
        batch: &mut Vec<DriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<storage_fee_distribution_outcome::v0::StorageFeeDistributionOutcome>, Error>
    {
        let mut inner_batch = GroveDbOpBatch::new();

        // init next thousandth empty epochs since last initiated
        let last_initiated_epoch_index = epoch_info
            .previous_epoch_index()
            .map_or(GENESIS_EPOCH_INDEX, |i| i + 1);

        for epoch_index in last_initiated_epoch_index..=epoch_info.current_epoch_index() {
            let next_thousandth_epoch = Epoch::new(
                epoch_index + perpetual_storage_epochs(self.config.drive.epochs_per_era),
            )?;
            next_thousandth_epoch.add_init_empty_without_storage_operations(&mut inner_batch);
        }

        // init current epoch pool for processing
        let current_epoch = Epoch::new(epoch_info.current_epoch_index())?;

        //todo: version
        current_epoch.add_init_current_operations(
            DEFAULT_ORIGINAL_FEE_MULTIPLIER, // TODO use a data contract to choose the fee multiplier
            block_info.height(),
            block_info.core_chain_locked_height(),
            block_info.block_time_ms(),
            &mut inner_batch,
        );

        // Nothing to distribute on genesis epoch start
        if current_epoch.index == GENESIS_EPOCH_INDEX {
            batch.push(DriveOperation::GroveDBOpBatch(inner_batch));
            return Ok(None);
        }

        // Distribute storage fees accumulated during previous epoch
        let storage_fee_distribution_outcome = self
            .add_distribute_storage_fee_to_epochs_operations(
                current_epoch.index,
                Some(transaction),
                &mut inner_batch,
                platform_version,
            )?;

        self.drive
            .add_delete_pending_epoch_refunds_except_specified_operations(
                &mut inner_batch,
                block_fees.refunds_per_epoch(),
                Some(transaction),
                &platform_version.drive,
            )?;

        batch.push(DriveOperation::GroveDBOpBatch(inner_batch));

        Ok(Some(storage_fee_distribution_outcome))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use chrono::Utc;
    use rust_decimal::prelude::ToPrimitive;

    use crate::test::helpers::setup::TestPlatformBuilder;

    mod helpers {
        use super::*;
        use crate::execution::types::block_fees::v0::{BlockFeesV0, BlockFeesV0Methods};
        use crate::execution::types::block_state_info::v0::BlockStateInfoV0;
        use crate::platform_types::epoch_info::v0::EpochInfoV0;
        use dpp::block::block_info::BlockInfo;
        use dpp::fee::epoch::CreditsPerEpoch;

        /// Process and validate an epoch change
        pub fn process_and_validate_epoch_change<C>(
            platform: &Platform<C>,
            genesis_time_ms: u64,
            epoch_index: u16,
            block_height: u64,
            previous_block_time_ms: Option<u64>,
            should_distribute: bool,
            transaction: &Transaction,
            platform_version: &PlatformVersion,
        ) -> BlockStateInfoV0 {
            let current_epoch = Epoch::new(epoch_index).expect("expected valid epoch index");

            // Add some storage fees to distribute next time
            if should_distribute {
                let block_fees = BlockFeesV0::from_fees(1000000000, 1000).into();

                let mut batch = vec![];

                platform
                    .add_distribute_block_fees_into_pools_operations(
                        &current_epoch,
                        &block_fees,
                        None,
                        Some(transaction),
                        &mut batch,
                        platform_version,
                    )
                    .expect("should add distribute block fees into pools operations");

                platform
                    .drive
                    .apply_drive_operations(
                        batch,
                        true,
                        &BlockInfo::default(),
                        Some(transaction),
                        platform_version,
                    )
                    .expect("should apply batch");
            }

            let proposer_pro_tx_hash: [u8; 32] = [
                1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
                1, 1, 1, 1,
            ];

            let block_time_ms = genesis_time_ms
                + epoch_index as u64 * platform.config.execution.epoch_time_length_s * 1000;

            let block_info = BlockStateInfoV0 {
                height: block_height,
                round: 0,
                block_time_ms,
                previous_block_time_ms,
                proposer_pro_tx_hash,
                core_chain_locked_height: 1,
                block_hash: None,
                app_hash: None,
            };

            let epoch_info = EpochInfoV0::from_genesis_time_and_block_info(
                genesis_time_ms,
                &block_info,
                platform.config.execution.epoch_time_length_s,
            )
            .expect("should calculate epoch info")
            .into();

            let block_fees = BlockFeesV0 {
                storage_fee: 1000000000,
                processing_fee: 10000,
                refunds_per_epoch: CreditsPerEpoch::from_iter([(0, 10000)]),
            }
            .into();

            let mut batch = vec![];

            let storage_fee_distribution_outcome = platform
                .add_process_epoch_change_operations_v0(
                    &block_info.clone().into(),
                    &epoch_info,
                    &block_fees,
                    transaction,
                    &mut batch,
                    platform_version,
                )
                .expect("should process epoch");

            platform
                .drive
                .apply_drive_operations(
                    batch,
                    true,
                    &BlockInfo::default(),
                    Some(transaction),
                    platform_version,
                )
                .expect("should apply batch");

            // Next thousandth epoch should be created
            let next_thousandth_epoch = Epoch::new(
                epoch_index + perpetual_storage_epochs(platform.config.drive.epochs_per_era),
            )
            .unwrap();

            let has_epoch_tree_exists = platform
                .drive
                .has_epoch_tree_exists(&next_thousandth_epoch, Some(transaction))
                .expect("should check epoch tree existence");

            assert!(has_epoch_tree_exists);

            // epoch should be initialized as current
            let epoch_start_block_height = platform
                .drive
                .get_epoch_start_block_height(&current_epoch, Some(transaction), platform_version)
                .expect("should get start block time from start epoch");

            assert_eq!(epoch_start_block_height, block_height);

            // storage fee should be distributed
            assert_eq!(
                storage_fee_distribution_outcome.is_some(),
                should_distribute
            );

            let thousandth_epoch = Epoch::new(next_thousandth_epoch.index - 1).unwrap();

            let aggregate_storage_fees = platform
                .drive
                .get_epoch_storage_credits_for_distribution(
                    &thousandth_epoch,
                    Some(transaction),
                    platform_version,
                )
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
        let platform_version = PlatformVersion::first();

        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();
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
            &transaction,
            platform_version,
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
            &transaction,
            platform_version,
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
            &transaction,
            platform_version,
        );
    }
}
