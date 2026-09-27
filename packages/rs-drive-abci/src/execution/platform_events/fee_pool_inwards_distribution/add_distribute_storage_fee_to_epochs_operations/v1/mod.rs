use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::storage_fee_distribution_outcome;
use crate::platform_types::platform::Platform;
use dpp::block::epoch::EpochIndex;
use dpp::fee::epoch::distribution::{
    distribute_storage_fee_to_epochs_collection,
    subtract_refunds_priced_in_epoch_from_epoch_credits_collection,
};
use dpp::fee::epoch::SignedCreditsPerEpoch;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;
use drive::util::batch::GroveDbOpBatch;

impl<C> Platform<C> {
    /// Adds operations to the GroveDB op batch which distribute storage fees
    /// from the distribution pool and subtract pending refunds
    /// Returns distribution leftovers
    ///
    /// V1 takes each pending refund back from the epochs it was priced for. A refund is priced
    /// in the epoch the data is removed in and returns the shares of the epochs after it; every
    /// refund pending at an epoch change was priced in `previous_epoch_index`, since the first
    /// block of an epoch keeps only its own refunds pending. V0 restored and subtracted from
    /// the epoch after the current one instead, so the current epoch kept its refunded share
    /// and the later epochs gave back more than theirs.
    pub(super) fn add_distribute_storage_fee_to_epochs_operations_v1(
        &self,
        current_epoch_index: EpochIndex,
        previous_epoch_index: Option<EpochIndex>,
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
            self.config.drive.epochs_per_era,
        )?;

        // Deduct pending refunds from the epochs they were refunded for. Shares of epochs that
        // closed before the current one come out of the current epoch
        // Leftovers are ignored since they already deducted from Identity's refund amount

        let refunds = self
            .drive
            .fetch_pending_epoch_refunds(transaction, &platform_version.drive)?;
        let refunded_epochs_count = refunds.len() as u16;

        if !refunds.is_empty() {
            let Some(pricing_epoch_index) = previous_epoch_index else {
                return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "pending storage refunds at an epoch change without a previous epoch",
                )));
            };

            for (epoch_index, credits) in refunds {
                subtract_refunds_priced_in_epoch_from_epoch_credits_collection(
                    &mut credits_per_epochs,
                    credits,
                    epoch_index,
                    pricing_epoch_index,
                    current_epoch_index,
                    self.config.drive.epochs_per_era,
                )?;
            }
        }

        self.drive
            .add_update_epoch_storage_fee_pools_sequence_operations(
                batch,
                credits_per_epochs,
                transaction,
                platform_version,
            )?;

        Ok(
            storage_fee_distribution_outcome::v0::StorageFeeDistributionOutcome {
                total_distributed_storage_fees: storage_distribution_fees,
                leftovers,
                refunded_epochs_count,
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::PlatformConfig;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
    use dpp::block::block_info::BlockInfo;
    use dpp::block::epoch::Epoch;
    use dpp::fee::epoch::distribution::calculate_storage_fee_refund_amount_and_leftovers;
    use dpp::fee::epoch::{perpetual_storage_epochs, CreditsPerEpoch};
    use dpp::fee::Credits;
    use drive::config::DriveConfig;
    use drive::drive::credit_pools::epochs::operations_factory::EpochOperations;
    use drive::drive::credit_pools::operations::update_storage_fee_distribution_pool_operation;
    use drive::drive::Drive;
    use drive::grovedb::Transaction;
    use drive::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
    use drive::util::batch::DriveOperation;
    use std::ops::Range;

    const EPOCHS_PER_ERA: u16 = 20;

    fn setup_platform() -> TempPlatform<MockCoreRPCLike> {
        TestPlatformBuilder::new()
            .with_config(PlatformConfig {
                drive: DriveConfig {
                    epochs_per_era: EPOCHS_PER_ERA,
                    ..Default::default()
                },
                ..Default::default()
            })
            .build_with_mock_rpc()
            .set_initial_state_structure()
    }

    /// Sets up the epoch change into `current_epoch_index` with `refunds` pending and nothing
    /// new to distribute, over epoch pools filled from the current epoch on
    fn prepare_epoch_change(
        platform: &TempPlatform<MockCoreRPCLike>,
        current_epoch_index: EpochIndex,
        refunds: CreditsPerEpoch,
        transaction: &Transaction,
    ) {
        let platform_version = PlatformVersion::latest();

        let mut batch = GroveDbOpBatch::new();

        // init additional epochs pools as it will be done in epoch_change
        let perpetual_storage_epochs = perpetual_storage_epochs(EPOCHS_PER_ERA);
        for epoch_index in perpetual_storage_epochs..=perpetual_storage_epochs + current_epoch_index
        {
            Epoch::new(epoch_index)
                .expect("expected a valid epoch")
                .add_init_empty_operations(&mut batch)
                .expect("should add init operations");
        }

        batch.push(
            update_storage_fee_distribution_pool_operation(100_000_000_000)
                .expect("should return operation"),
        );
        platform
            .drive
            .grove_apply_batch(batch, false, Some(transaction), &platform_version.drive)
            .expect("should apply batch");

        let mut batch = GroveDbOpBatch::new();
        platform
            .add_distribute_storage_fee_to_epochs_operations(
                current_epoch_index,
                None,
                Some(transaction),
                &mut batch,
                platform_version,
            )
            .expect("should distribute storage fee pool");
        batch
            .push(update_storage_fee_distribution_pool_operation(0).expect("should add operation"));

        let mut operations = vec![];
        Drive::add_update_pending_epoch_refunds_operations(
            &mut operations,
            refunds,
            &platform_version.drive,
        )
        .expect("should update pending epoch refunds");
        operations.push(DriveOperation::GroveDBOpBatch(batch));

        platform
            .drive
            .apply_drive_operations(
                operations,
                true,
                &BlockInfo::default(),
                Some(transaction),
                platform_version,
                None,
            )
            .expect("should apply batch");
    }

    fn epoch_storage_pools(
        platform: &TempPlatform<MockCoreRPCLike>,
        epochs: Range<EpochIndex>,
        transaction: &Transaction,
    ) -> Vec<Credits> {
        platform
            .drive
            .get_storage_credits_for_distribution_for_epochs_in_range(
                epochs,
                Some(transaction),
                PlatformVersion::latest(),
            )
            .expect("should get storage fees")
    }

    #[test]
    fn should_claw_back_each_pending_refund_from_the_epochs_it_was_priced_for() {
        let platform = setup_platform();
        let transaction = platform.drive.grove.start_transaction();
        let platform_version = PlatformVersion::latest();

        // Stored in epoch 1 and removed in epoch 2, clawed back at the change into epoch 3.
        // Every share of this fee is a whole number of credits
        let storage_fee: Credits = 10_000_000_000;
        let (refund, _) =
            calculate_storage_fee_refund_amount_and_leftovers(storage_fee, 1, 2, EPOCHS_PER_ERA)
                .expect("should price the refund");

        let current_epoch_index = 3;
        prepare_epoch_change(
            &platform,
            current_epoch_index,
            CreditsPerEpoch::from_iter([(1, refund)]),
            &transaction,
        );

        let pools_range = 0..current_epoch_index + perpetual_storage_epochs(EPOCHS_PER_ERA);
        let pools_before = epoch_storage_pools(&platform, pools_range.clone(), &transaction);

        let mut batch = GroveDbOpBatch::new();
        let outcome = platform
            .add_distribute_storage_fee_to_epochs_operations(
                current_epoch_index,
                Some(2),
                Some(&transaction),
                &mut batch,
                platform_version,
            )
            .expect("should distribute storage fee pool");
        platform
            .drive
            .grove_apply_batch(batch, false, Some(&transaction), &platform_version.drive)
            .expect("should apply batch");

        assert_eq!(outcome.refunded_epochs_count, 1);

        let pools_after = epoch_storage_pools(&platform, pools_range.clone(), &transaction);

        let mut shares = SignedCreditsPerEpoch::default();
        distribute_storage_fee_to_epochs_collection(&mut shares, storage_fee, 1, EPOCHS_PER_ERA)
            .expect("should distribute storage fee");

        for (epoch_index, (before, after)) in pools_range.zip(pools_before.iter().zip(&pools_after))
        {
            // The refund returned the shares of the epochs after epoch 2
            let refunded_share = if epoch_index > 2 {
                shares.get(&epoch_index).copied().unwrap_or(0) as Credits
            } else {
                0
            };
            assert_eq!(
                before - after,
                refunded_share,
                "epoch {epoch_index} should give back its refunded share"
            );
        }

        // Epoch 3 gives back its own share, not just leftovers
        assert_eq!(pools_before[3] - pools_after[3], 25_000_000);
        assert_eq!(
            pools_before.iter().sum::<Credits>() - pools_after.iter().sum::<Credits>(),
            refund
        );
    }

    #[test]
    fn should_refuse_pending_refunds_at_an_epoch_change_without_a_previous_epoch() {
        let platform = setup_platform();
        let transaction = platform.drive.grove.start_transaction();

        prepare_epoch_change(
            &platform,
            3,
            CreditsPerEpoch::from_iter([(1, 1_000_000)]),
            &transaction,
        );

        let result = platform.add_distribute_storage_fee_to_epochs_operations(
            3,
            None,
            Some(&transaction),
            &mut GroveDbOpBatch::new(),
            PlatformVersion::latest(),
        );

        assert!(matches!(
            result,
            Err(Error::Execution(ExecutionError::CorruptedCodeExecution(_)))
        ));
    }
}
