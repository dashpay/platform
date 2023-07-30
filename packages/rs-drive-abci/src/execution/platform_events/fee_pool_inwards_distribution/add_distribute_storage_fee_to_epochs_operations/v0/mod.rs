use crate::error::Error;
use crate::execution::types::storage_fee_distribution_outcome;
use crate::platform_types::platform::Platform;
use dpp::block::epoch::EpochIndex;
use dpp::fee::epoch::distribution::{
    distribute_storage_fee_to_epochs_collection, subtract_refunds_from_epoch_credits_collection,
};
use dpp::fee::epoch::SignedCreditsPerEpoch;
use dpp::version::PlatformVersion;
use drive::drive::batch::GroveDbOpBatch;
use drive::fee::epoch::distribution::{
    distribute_storage_fee_to_epochs_collection, subtract_refunds_from_epoch_credits_collection,
};
use drive::fee::epoch::{EpochIndex, SignedCreditsPerEpoch};
use drive::grovedb::TransactionArg;

impl<C> Platform<C> {
    /// Adds operations to the GroveDB op batch which distribute storage fees
    /// from the distribution pool and subtract pending refunds
    /// Returns distribution leftovers
    pub(super) fn add_distribute_storage_fee_to_epochs_operations_v0(
        &self,
        current_epoch_index: EpochIndex,
        transaction: TransactionArg,
        batch: &mut GroveDbOpBatch,
        platform_version: &PlatformVersion,
    ) -> Result<storage_fee_distribution_outcome::v0::StorageFeeDistributionOutcome, Error> {
        let storage_distribution_fees = self
            .drive
            .get_storage_fees_from_distribution_pool(transaction, platform_version)?;

        let mut credits_per_epochs = SignedCreditsPerEpoch::default();

        // Distribute from storage distribution pool
        let leftovers = distribute_storage_fee_to_epochs_collection(
            &mut credits_per_epochs,
            storage_distribution_fees,
            current_epoch_index,
        )?;

        // Deduct pending refunds since epoch where data was removed skipping previous
        // (already paid or pay-in-progress) epochs. We want people to pay for the current epoch
        // Leftovers are ignored since they already deducted from Identity's refund amount

        let refunds = self
            .drive
            .fetch_pending_epoch_refunds(transaction, &platform_version.drive)?;
        let refunded_epochs_count = refunds.len() as u16;

        for (epoch_index, credits) in refunds {
            subtract_refunds_from_epoch_credits_collection(
                &mut credits_per_epochs,
                credits,
                epoch_index,
                current_epoch_index,
            )?;
        }

        self.drive
            .add_update_epoch_storage_fee_pools_sequence_operations(
                batch,
                credits_per_epochs,
                transaction,
            )?;

        Ok(
            storage_fee_distribution_outcome::v0::StorageFeeDistributionOutcome {
                leftovers,
                refunded_epochs_count,
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use drive::common::helpers::epoch::get_storage_credits_for_distribution_for_epochs_in_range;

    mod add_distribute_storage_fee_to_epochs_operations {
        use dpp::balances::credits::Creditable;
        use dpp::block::epoch::Epoch;
        use dpp::block::block_info::BlockInfo;
        use dpp::fee::epoch::distribution::subtract_refunds_from_epoch_credits_collection;
        use dpp::fee::epoch::{
            CreditsPerEpoch, SignedCreditsPerEpoch, GENESIS_EPOCH_INDEX, PERPETUAL_STORAGE_EPOCHS,
        };
        use dpp::fee::Credits;
        use drive::drive::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
        use drive::drive::batch::DriveOperation;
        use drive::drive::credit_pools::pending_epoch_refunds::add_update_pending_epoch_refunds_operations;
        use drive::fee_pools::epochs::operations_factory::EpochOperations;
        use drive::fee_pools::update_storage_fee_distribution_pool_operation;

        use crate::test::helpers::setup::TestPlatformBuilder;

        use super::*;

        #[test]
        fn should_add_operations_to_distribute_distribution_storage_pool_and_refunds() {
            let platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure();
            let transaction = platform.drive.grove.start_transaction();

            let platform_version = PlatformVersion::latest();

            /*
            Initial distribution
            */

            let storage_pool = 1000000;
            let current_epoch_index = 0;

            let mut batch = GroveDbOpBatch::new();

            // Store distribution storage fees
            batch.push(
                update_storage_fee_distribution_pool_operation(storage_pool)
                    .expect("should return operation"),
            );

            platform
                .drive
                .grove_apply_batch(batch, false, Some(&transaction), &platform_version.drive)
                .expect("should apply batch");

            let mut batch = GroveDbOpBatch::new();

            platform
                .add_distribute_storage_fee_to_epochs_operations(
                    current_epoch_index,
                    Some(&transaction),
                    &mut batch,
                    platform_version,
                )
                .expect("should distribute storage fee pool");

            platform
                .drive
                .grove_apply_batch(batch, false, Some(&transaction), &platform_version.drive)
                .expect("should apply batch");

            /*
            Distribute since epoch 2 with refunds
            */

            let current_epoch_index = 3;

            let mut batch = vec![];

            let mut inner_batch = GroveDbOpBatch::new();

            // init additional epochs pools as it will be done in epoch_change
            for i in PERPETUAL_STORAGE_EPOCHS..=PERPETUAL_STORAGE_EPOCHS + current_epoch_index {
                let epoch = Epoch::new(i).unwrap();
                epoch
                    .add_init_empty_operations(&mut inner_batch)
                    .expect("should add init operations");
            }

            // Store distribution storage fees
            inner_batch.push(
                update_storage_fee_distribution_pool_operation(storage_pool)
                    .expect("should add operation"),
            );

            // Add pending refunds

            let refunds =
                CreditsPerEpoch::from_iter([(0, 10000), (1, 15000), (2, 20000), (3, 25000)]);

            add_update_pending_epoch_refunds_operations(&mut batch, refunds.clone())
                .expect("should update pending epoch refunds");

            batch.push(DriveOperation::GroveDBOpBatch(inner_batch));

            platform
                .drive
                .apply_drive_operations(
                    batch,
                    true,
                    &BlockInfo::default(),
                    Some(&transaction),
                    platform_version,
                )
                .expect("should apply batch");

            let mut batch = GroveDbOpBatch::new();

            let outcome = platform
                .add_distribute_storage_fee_to_epochs_operations(
                    current_epoch_index,
                    Some(&transaction),
                    &mut batch,
                    platform_version,
                )
                .expect("should distribute storage fee pool");

            platform
                .drive
                .grove_apply_batch(batch, false, Some(&transaction), &platform_version.drive)
                .expect("should apply batch");

            // check leftover
            assert_eq!(outcome.leftovers, 180);
            assert_eq!(outcome.refunded_epochs_count, refunds.len() as u16);

            // collect all the storage fee values of the 1000 epochs pools
            let storage_fees = drive.get_storage_credits_for_distribution_for_epochs_in_range(
                &platform.drive,
                GENESIS_EPOCH_INDEX..current_epoch_index + PERPETUAL_STORAGE_EPOCHS,
                Some(&transaction),
                platform_version,
            );

            // Assert total distributed fees

            let total_storage_pool_distribution = (storage_pool - outcome.leftovers) * 2;

            let total_refunds: Credits = refunds
                .into_iter()
                .map(|(epoch_index, credits)| {
                    let mut credits_per_epochs = SignedCreditsPerEpoch::default();

                    subtract_refunds_from_epoch_credits_collection(
                        &mut credits_per_epochs,
                        credits,
                        epoch_index,
                        current_epoch_index,
                    )
                    .expect("should subtract refunds");

                    credits_per_epochs
                        .into_values()
                        .map(|credits| credits.to_unsigned())
                        .sum::<Credits>()
                })
                .sum();

            let total_distributed = storage_fees.into_iter().sum::<Credits>();

            assert_eq!(
                total_distributed,
                total_storage_pool_distribution - total_refunds
            );
        }
    }
}
