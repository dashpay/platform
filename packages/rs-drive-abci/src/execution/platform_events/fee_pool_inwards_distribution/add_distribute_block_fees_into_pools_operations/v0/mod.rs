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

use crate::abci::messages::BlockFees;
use crate::error::Error;
use crate::execution::types::fees_in_pools::v0::FeesInPools;
use crate::platform_types::platform::Platform;
use dpp::block::epoch::Epoch;
use drive::drive::batch::DriveOperation;
use drive::fee::credits::Credits;
use drive::fee_pools::epochs::operations_factory::EpochOperations;
use drive::fee_pools::update_storage_fee_distribution_pool_operation;
use drive::grovedb::TransactionArg;
use drive::{error, grovedb};

impl<CoreRPCLike> Platform<CoreRPCLike> {
    /// Adds operations to an op batch which update total storage fees
    /// for the epoch considering fees from a new block.
    ///
    /// Returns `FeesInPools`
    pub fn add_distribute_block_fees_into_pools_operations_v0(
        &self,
        current_epoch: &Epoch,
        block_fees: &BlockFees,
        cached_aggregated_storage_fees: Option<Credits>,
        transaction: TransactionArg,
        batch: &mut Vec<DriveOperation>,
    ) -> Result<FeesInPools, Error> {
        // update epochs pool processing fees
        let epoch_processing_fees = self
            .drive
            .get_epoch_processing_credits_for_distribution(current_epoch, transaction)
            .or_else(|e| match e {
                // Handle epoch change when storage fees are not set yet
                error::Error::GroveDB(grovedb::Error::PathKeyNotFound(_)) => Ok(0u64),
                _ => Err(e),
            })?;

        let total_processing_fees = epoch_processing_fees + block_fees.processing_fee;

        batch.push(DriveOperation::GroveDBOperation(
            current_epoch.update_processing_fee_pool_operation(total_processing_fees)?,
        ));

        // update storage fee pool
        let storage_distribution_credits_in_fee_pool = match cached_aggregated_storage_fees {
            None => self
                .drive
                .get_storage_fees_from_distribution_pool(transaction)?,
            Some(storage_fees) => storage_fees,
        };

        let total_storage_fees = storage_distribution_credits_in_fee_pool + block_fees.storage_fee;

        batch.push(DriveOperation::GroveDBOperation(
            update_storage_fee_distribution_pool_operation(
                storage_distribution_credits_in_fee_pool + block_fees.storage_fee,
            )?,
        ));

        Ok(FeesInPools {
            processing_fees: total_processing_fees,
            storage_fees: total_storage_fees,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::block::block_info::BlockInfo;

    use crate::test::helpers::setup::TestPlatformBuilder;

    use drive::drive::batch::GroveDbOpBatch;

    #[test]
    fn test_distribute_block_fees_into_uncommitted_epoch_on_epoch_change() {
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();
        let transaction = platform.drive.grove.start_transaction();

        let current_epoch_tree = Epoch::new(1).unwrap();

        let mut batch = vec![];

        let mut inner_batch = GroveDbOpBatch::new();

        current_epoch_tree.add_init_current_operations(1.0, 1, 1, 1, &mut inner_batch);

        batch.push(DriveOperation::GroveDBOpBatch(inner_batch));

        let processing_fees = 1000000;
        let storage_fees = 2000000;

        platform
            .add_distribute_block_fees_into_pools_operations_v0(
                &current_epoch_tree,
                &BlockFees::from_fees(storage_fees, processing_fees),
                None,
                Some(&transaction),
                &mut batch,
            )
            .expect("should distribute fees into pools");

        platform
            .drive
            .apply_drive_operations(batch, true, &BlockInfo::default(), Some(&transaction))
            .expect("should apply batch");

        let stored_processing_fee_credits = platform
            .drive
            .get_epoch_processing_credits_for_distribution(&current_epoch_tree, Some(&transaction))
            .expect("should get processing fees");

        let stored_storage_fee_credits = platform
            .drive
            .get_storage_fees_from_distribution_pool(Some(&transaction))
            .expect("should get storage fee pool");

        assert_eq!(stored_processing_fee_credits, processing_fees);
        assert_eq!(stored_storage_fee_credits, storage_fees);
    }

    #[test]
    fn test_distribute_block_fees_into_pools() {
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();
        let transaction = platform.drive.grove.start_transaction();

        let current_epoch_tree = Epoch::new(1).unwrap();

        let mut batch = GroveDbOpBatch::new();

        current_epoch_tree.add_init_current_operations(1.0, 1, 1, 1, &mut batch);

        // Apply new pool structure
        platform
            .drive
            .grove_apply_batch(batch, false, Some(&transaction))
            .expect("should apply batch");

        let mut batch = vec![];

        let processing_fees = 1000000;
        let storage_fees = 2000000;

        platform
            .add_distribute_block_fees_into_pools_operations_v0(
                &current_epoch_tree,
                &BlockFees::from_fees(storage_fees, processing_fees),
                None,
                Some(&transaction),
                &mut batch,
            )
            .expect("should distribute fees into pools");

        platform
            .drive
            .apply_drive_operations(batch, true, &BlockInfo::default(), Some(&transaction))
            .expect("should apply batch");

        let stored_processing_fee_credits = platform
            .drive
            .get_epoch_processing_credits_for_distribution(&current_epoch_tree, Some(&transaction))
            .expect("should get processing fees");

        let stored_storage_fee_credits = platform
            .drive
            .get_storage_fees_from_distribution_pool(Some(&transaction))
            .expect("should get storage fee pool");

        assert_eq!(stored_processing_fee_credits, processing_fees);
        assert_eq!(stored_storage_fee_credits, storage_fees);
    }
}
