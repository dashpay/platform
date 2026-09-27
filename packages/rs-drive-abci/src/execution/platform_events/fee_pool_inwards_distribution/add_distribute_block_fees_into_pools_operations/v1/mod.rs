use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::block_fees::v0::BlockFeesV0Getters;
use crate::execution::types::block_fees::BlockFees;
use crate::execution::types::fees_in_pools::v0::FeesInPoolsV0;
use crate::platform_types::platform::Platform;
use dpp::block::epoch::Epoch;
use dpp::fee::Credits;
use dpp::version::PlatformVersion;
use drive::drive::credit_pools::operations::update_lifetime_storage_fee_pool_operation;
use drive::grovedb::TransactionArg;
use drive::util::batch::DriveOperation;

impl<C> Platform<C> {
    /// v0, except that the part of the block's storage fees for storage that lives a known
    /// number of epochs (documents with a time to live) goes to the lifetime storage fee pools
    /// of the current epoch, by that number, instead of the storage fee distribution pool; the
    /// next epoch change spreads each pool evenly over its epochs and removes it.
    ///
    /// A block adds only to the pools of its own epoch, and an epoch change spreads and removes
    /// only the pools of earlier epochs, so the two never write the same pool in one batch.
    pub(super) fn add_distribute_block_fees_into_pools_operations_v1(
        &self,
        current_epoch: &Epoch,
        block_fees: &BlockFees,
        cached_aggregated_storage_fees: Option<Credits>,
        transaction: TransactionArg,
        batch: &mut Vec<DriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<FeesInPoolsV0, Error> {
        let block_lifetime_storage_fees = block_fees.lifetime_storage_fees();
        let lifetime_storage_fees_total = block_lifetime_storage_fees
            .values()
            .try_fold(0u64, |total, credits| total.checked_add(*credits))
            .ok_or(ExecutionError::Overflow(
                "overflow adding the lifetime storage fees of a block",
            ))?;
        let perpetual_storage_fee = block_fees
            .storage_fee()
            .checked_sub(lifetime_storage_fees_total)
            .ok_or(ExecutionError::CorruptedCodeExecution(
                "the lifetime storage fees of a block exceed its storage fees",
            ))?;

        // The processing fees and the storage fee distribution pool, as v0 does, with the
        // block's perpetual storage fees only.
        let fees_in_pools = self.add_distribute_block_fees_into_pools_operations_v0_with_storage(
            current_epoch,
            block_fees.processing_fee(),
            perpetual_storage_fee,
            cached_aggregated_storage_fees,
            transaction,
            batch,
            platform_version,
        )?;

        if block_lifetime_storage_fees.is_empty() {
            return Ok(fees_in_pools);
        }
        // What the earlier blocks of this epoch collected. The first block of an epoch finds
        // none: the pools it reads are those of earlier epochs, which its epoch change spreads.
        let current_epoch_pools = self
            .drive
            .fetch_lifetime_storage_fee_pools(transaction, platform_version)?
            .remove(&current_epoch.index)
            .unwrap_or_default();
        for (lifetime_epochs, credits) in block_lifetime_storage_fees {
            let pool_credits = current_epoch_pools
                .get(lifetime_epochs)
                .copied()
                .unwrap_or_default()
                .checked_add(*credits)
                .ok_or(ExecutionError::Overflow(
                    "overflow adding to a lifetime storage fee pool",
                ))?;
            batch.push(DriveOperation::GroveDBOperation(
                update_lifetime_storage_fee_pool_operation(
                    current_epoch.index,
                    *lifetime_epochs,
                    pool_credits,
                )?,
            ));
        }

        Ok(fees_in_pools)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::types::block_fees::v0::BlockFeesV0;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
    use dpp::block::block_info::BlockInfo;
    use dpp::block::epoch::EpochIndex;
    use dpp::fee::fee_result::LifetimeStorageFees;
    use drive::grovedb::Transaction;
    use std::collections::BTreeMap;

    fn block_fees(storage_fee: Credits, lifetime: &[(u16, Credits)]) -> BlockFees {
        BlockFeesV0 {
            storage_fee,
            processing_fee: 1_000,
            lifetime_storage_fees: LifetimeStorageFees::from_iter(lifetime.iter().copied()),
            ..Default::default()
        }
        .into()
    }

    fn distribute(
        platform: &TempPlatform<MockCoreRPCLike>,
        epoch_index: EpochIndex,
        block_fees: &BlockFees,
        transaction: &Transaction,
    ) {
        let platform_version = PlatformVersion::latest();
        let mut batch = vec![];
        platform
            .add_distribute_block_fees_into_pools_operations_v1(
                &Epoch::new(epoch_index).expect("epoch"),
                block_fees,
                None,
                Some(transaction),
                &mut batch,
                platform_version,
            )
            .expect("should distribute the block fees");
        platform
            .drive
            .apply_drive_operations(
                batch,
                true,
                &BlockInfo::default(),
                Some(transaction),
                platform_version,
                None,
            )
            .expect("should apply the batch");
    }

    fn lifetime_pools(
        platform: &TempPlatform<MockCoreRPCLike>,
        transaction: &Transaction,
    ) -> BTreeMap<EpochIndex, LifetimeStorageFees> {
        platform
            .drive
            .fetch_lifetime_storage_fee_pools(Some(transaction), PlatformVersion::latest())
            .expect("should read the lifetime pools")
    }

    #[test]
    fn should_add_lifetime_storage_fees_to_their_pools_and_the_rest_to_the_storage_fee_pool() {
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();
        let transaction = platform.drive.grove.start_transaction();

        distribute(
            &platform,
            1,
            &block_fees(1_000_000, &[(1, 300_000), (40, 400_000)]),
            &transaction,
        );
        assert_eq!(
            platform
                .drive
                .get_storage_fees_from_distribution_pool(
                    Some(&transaction),
                    PlatformVersion::latest()
                )
                .expect("should read the storage fee pool"),
            300_000
        );
        assert_eq!(
            lifetime_pools(&platform, &transaction),
            BTreeMap::from([(1, LifetimeStorageFees::from([(1, 300_000), (40, 400_000)]))])
        );

        // The next block of the epoch adds to its pools.
        distribute(&platform, 1, &block_fees(100, &[(1, 100)]), &transaction);
        assert_eq!(
            lifetime_pools(&platform, &transaction),
            BTreeMap::from([(1, LifetimeStorageFees::from([(1, 300_100), (40, 400_000)]))])
        );
    }

    #[test]
    fn should_add_only_to_the_pools_of_the_blocks_epoch() {
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();
        let transaction = platform.drive.grove.start_transaction();
        distribute(
            &platform,
            1,
            &block_fees(107, &[(2, 100), (5, 7)]),
            &transaction,
        );

        // A block of epoch 2 leaves the pools of epoch 1 to the epoch change, which spreads and
        // removes them, and opens its own.
        distribute(
            &platform,
            2,
            &block_fees(41, &[(2, 40), (9, 1)]),
            &transaction,
        );
        assert_eq!(
            lifetime_pools(&platform, &transaction),
            BTreeMap::from([
                (1, LifetimeStorageFees::from([(2, 100), (5, 7)])),
                (2, LifetimeStorageFees::from([(2, 40), (9, 1)])),
            ])
        );
    }
}
