use crate::error::Error;
use crate::execution::types::block_fees::v0::BlockFeesV0Getters;
use crate::execution::types::block_fees::BlockFees;
use crate::execution::types::fees_in_pools::v0::FeesInPoolsV0;
use crate::platform_types::platform::Platform;
use dpp::block::epoch::Epoch;
use dpp::fee::Credits;
use dpp::version::PlatformVersion;
use drive::util::batch::DriveOperation;

use drive::drive::credit_pools::epochs::operations_factory::EpochOperations;
use drive::drive::credit_pools::operations::update_storage_fee_distribution_pool_operation;
use drive::grovedb::TransactionArg;
use drive::{error, grovedb};

impl<C> Platform<C> {
    /// Adds operations to an op batch which update total storage fees
    /// for the epoch considering fees from a new block.
    ///
    /// Returns `FeesInPools`
    pub(super) fn add_distribute_block_fees_into_pools_operations_v0(
        &self,
        current_epoch: &Epoch,
        block_fees: &BlockFees,
        cached_aggregated_storage_fees: Option<Credits>,
        transaction: TransactionArg,
        batch: &mut Vec<DriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<FeesInPoolsV0, Error> {
        // update epochs pool processing fees
        let epoch_processing_fees = self
            .drive
            .get_epoch_processing_credits_for_distribution(
                current_epoch,
                transaction,
                platform_version,
            )
            .or_else(|e| match e {
                // Handle epoch change when storage fees are not set yet
                error::Error::GroveDB(grovedb::Error::PathKeyNotFound(_)) => Ok(0u64),
                _ => Err(e),
            })?;

        let total_processing_fees = epoch_processing_fees + block_fees.processing_fee();

        batch.push(DriveOperation::GroveDBOperation(
            current_epoch.update_processing_fee_pool_operation(total_processing_fees)?,
        ));

        // update storage fee pool
        let storage_distribution_credits_in_fee_pool = match cached_aggregated_storage_fees {
            None => self
                .drive
                .get_storage_fees_from_distribution_pool(transaction, platform_version)?,
            Some(storage_fees) => storage_fees,
        };

        let total_storage_fees =
            storage_distribution_credits_in_fee_pool + block_fees.storage_fee();

        batch.push(DriveOperation::GroveDBOperation(
            update_storage_fee_distribution_pool_operation(
                storage_distribution_credits_in_fee_pool + block_fees.storage_fee(),
            )?,
        ));

        Ok(FeesInPoolsV0 {
            processing_fees: total_processing_fees,
            storage_fees: total_storage_fees,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::block::block_info::BlockInfo;
    use drive::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;

    use crate::test::helpers::setup::TestPlatformBuilder;

    use crate::execution::types::block_fees;
    use crate::execution::types::block_fees::v0::BlockFeesV0Methods;
    use drive::util::batch::GroveDbOpBatch;

    #[test]
    fn test_distribute_block_fees_into_uncommitted_epoch_on_epoch_change() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();
        let transaction = platform.drive.grove.start_transaction();

        let current_epoch_tree = Epoch::new(1).unwrap();

        let mut batch = vec![];

        let mut inner_batch = GroveDbOpBatch::new();

        current_epoch_tree.add_init_current_operations(
            platform_version
                .fee_version
                .uses_version_fee_multiplier_permille
                .expect("expected a fee multiplier"),
            1,
            1,
            1,
            &mut inner_batch,
        );

        batch.push(DriveOperation::GroveDBOpBatch(inner_batch));

        let processing_fees = 1000000;
        let storage_fees = 2000000;

        let block_fees =
            block_fees::v0::BlockFeesV0::from_fees(storage_fees, processing_fees).into();

        platform
            .add_distribute_block_fees_into_pools_operations_v0(
                &current_epoch_tree,
                &block_fees,
                None,
                Some(&transaction),
                &mut batch,
                platform_version,
            )
            .expect("should distribute fees into pools");

        platform
            .drive
            .apply_drive_operations(
                batch,
                true,
                &BlockInfo::default(),
                Some(&transaction),
                platform_version,
                None,
            )
            .expect("should apply batch");

        let stored_processing_fee_credits = platform
            .drive
            .get_epoch_processing_credits_for_distribution(
                &current_epoch_tree,
                Some(&transaction),
                platform_version,
            )
            .expect("should get processing fees");

        let stored_storage_fee_credits = platform
            .drive
            .get_storage_fees_from_distribution_pool(Some(&transaction), platform_version)
            .expect("should get storage fee pool");

        assert_eq!(stored_processing_fee_credits, processing_fees);
        assert_eq!(stored_storage_fee_credits, storage_fees);
    }

    #[test]
    fn test_distribute_block_fees_into_pools() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();
        let transaction = platform.drive.grove.start_transaction();

        let current_epoch_tree = Epoch::new(1).unwrap();

        let mut batch = GroveDbOpBatch::new();

        current_epoch_tree.add_init_current_operations(
            platform_version
                .fee_version
                .uses_version_fee_multiplier_permille
                .expect("expected a fee multiplier"),
            1,
            1,
            1,
            &mut batch,
        );

        // Apply new pool structure
        platform
            .drive
            .grove_apply_batch(batch, false, Some(&transaction), &platform_version.drive)
            .expect("should apply batch");

        let mut batch = vec![];

        let processing_fees = 1000000;
        let storage_fees = 2000000;

        let block_fees =
            block_fees::v0::BlockFeesV0::from_fees(storage_fees, processing_fees).into();

        platform
            .add_distribute_block_fees_into_pools_operations_v0(
                &current_epoch_tree,
                &block_fees,
                None,
                Some(&transaction),
                &mut batch,
                platform_version,
            )
            .expect("should distribute fees into pools");

        platform
            .drive
            .apply_drive_operations(
                batch,
                true,
                &BlockInfo::default(),
                Some(&transaction),
                platform_version,
                None,
            )
            .expect("should apply batch");

        let stored_processing_fee_credits = platform
            .drive
            .get_epoch_processing_credits_for_distribution(
                &current_epoch_tree,
                Some(&transaction),
                platform_version,
            )
            .expect("should get processing fees");

        let stored_storage_fee_credits = platform
            .drive
            .get_storage_fees_from_distribution_pool(Some(&transaction), platform_version)
            .expect("should get storage fee pool");

        assert_eq!(stored_processing_fee_credits, processing_fees);
        assert_eq!(stored_storage_fee_credits, storage_fees);
    }
}
