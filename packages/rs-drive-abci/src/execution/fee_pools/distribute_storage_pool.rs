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

//! Fee Distribution to Epoch Pools.
//!
//! This module defines and implements in the Platform trait functions to add up and distribute
//! storage fees from the distribution pool to the epoch pools.
//!

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform::Platform;
use drive::drive::batch::GroveDbOpBatch;
use drive::fee::credits::{Credits, SignedCredits};
use drive::fee::epoch::distribution::{
    distribute_refunds_to_epochs_collection, distribute_storage_fee_to_epochs_collection,
};
use drive::fee::epoch::{EpochIndex, SignedCreditsPerEpoch};
use drive::grovedb::TransactionArg;

/// Result of storage fee distribution
pub struct StorageFeeDistributionOutcome {
    /// Leftovers in result of divisions and rounding after storage fee distribution to epochs
    pub leftovers: Credits,
    /// A number of epochs which had refunded
    pub refunded_epochs_count: u16,
}

impl Platform {
    /// Adds operations to the GroveDB op batch which calculate and distribute storage fees
    /// from the distribution pool and pending updates to the epoch pools and returns the leftovers.
    pub fn add_distribute_storage_fee_to_epochs_operations(
        &self,
        current_epoch_index: EpochIndex,
        transaction: TransactionArg,
        batch: &mut GroveDbOpBatch,
    ) -> Result<StorageFeeDistributionOutcome, Error> {
        let storage_distribution_fees = self
            .drive
            .get_aggregate_storage_fees_from_distribution_pool(transaction)?;

        let mut credits_per_epochs = SignedCreditsPerEpoch::default();

        // Distribute from storage distribution pool
        let leftovers = distribute_storage_fee_to_epochs_collection(
            &mut credits_per_epochs,
            storage_distribution_fees,
            current_epoch_index,
        )?;

        // Deduct refunds since epoch where data was removed skipping previous (already paid or pay-in-progress) epochs.
        // We want people to pay for the current epoch
        // Leftovers are ignored since they already deducted from Identity's refund amount

        let refunds = self.drive.fetch_pending_updates(transaction)?;
        let refunded_epochs_count = refunds.len() as u16;

        for (epoch_index, credits) in refunds {
            subtract_refunds_from_epoch_credits_collection(
                &mut credits_per_epochs,
                credits,
                epoch_index,
                current_epoch_index + 1,
            )?;
        }

        dbg!(&credits_per_epochs);

        self.drive
            .add_update_epoch_storage_fee_pools_sequence_operations(
                batch,
                credits_per_epochs,
                transaction,
            )?;

        Ok(StorageFeeDistributionOutcome {
            leftovers,
            refunded_epochs_count,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::common::helpers::setup::setup_platform_with_initial_state_structure;
    use drive::common::helpers::epoch::get_storage_credits_for_distribution_for_epochs_in_range;

    mod add_distribute_storage_fee_to_epochs_operations {
        use drive::drive::fee_pools::pending_epoch_updates::add_update_pending_epoch_storage_pool_update_operations;
        use drive::fee::credits::Creditable;
        use drive::fee::epoch::{CreditsPerEpoch, GENESIS_EPOCH_INDEX, PERPETUAL_STORAGE_EPOCHS};
        use drive::fee_pools::epochs::Epoch;
        use drive::fee_pools::update_storage_fee_distribution_pool_operation;

        use super::*;

        #[test]
        fn should_add_operations_to_distribute_distribution_storage_pool_and_refunds() {
            let platform = setup_platform_with_initial_state_structure(None);
            let transaction = platform.drive.grove.start_transaction();

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
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            let mut batch = GroveDbOpBatch::new();

            platform
                .add_distribute_storage_fee_to_epochs_operations(
                    current_epoch_index,
                    Some(&transaction),
                    &mut batch,
                )
                .expect("should distribute storage fee pool");

            platform
                .drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            /*
            Distribute since epoch 2 with refunds
            */

            let storage_pool = 1000000;
            let current_epoch_index = 3;

            let mut batch = GroveDbOpBatch::new();

            // init additional epochs pools as it will be done in epoch_change
            for i in PERPETUAL_STORAGE_EPOCHS..=PERPETUAL_STORAGE_EPOCHS + current_epoch_index {
                let epoch = Epoch::new(i);
                epoch
                    .add_init_empty_operations(&mut batch)
                    .expect("should add init operations");
            }

            // Store distribution storage fees
            batch.push(
                update_storage_fee_distribution_pool_operation(storage_pool)
                    .expect("should add operation"),
            );

            // Add pending refunds

            let refunds =
                CreditsPerEpoch::from_iter([(0, 10000), (1, 15000), (2, 20000), (3, 25000)]);

            add_update_pending_epoch_storage_pool_update_operations(&mut batch, refunds.clone())
                .expect("should update pending updates");

            platform
                .drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            let mut batch = GroveDbOpBatch::new();

            let outcome = platform
                .add_distribute_storage_fee_to_epochs_operations(
                    current_epoch_index,
                    Some(&transaction),
                    &mut batch,
                )
                .expect("should distribute storage fee pool");

            platform
                .drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("should apply batch");

            // check leftover
            assert_eq!(outcome.leftovers, 180);
            assert_eq!(outcome.refunded_epochs_count, refunds.len() as u16);

            // collect all the storage fee values of the 1000 epochs pools
            let storage_fees = get_storage_credits_for_distribution_for_epochs_in_range(
                &platform.drive,
                GENESIS_EPOCH_INDEX..current_epoch_index + PERPETUAL_STORAGE_EPOCHS,
                Some(&transaction),
            );

            // Assert total distributed fees

            let total_storage_pool_distribution = (storage_pool - outcome.leftovers) * 2;

            let total_refunds: Credits = refunds
                .into_iter()
                .map(|(epoch_index, credits)| {
                    let mut credits_per_epochs = SignedCreditsPerEpoch::default();

                    let leftovers = distribute_storage_fee_to_epochs_collection(
                        &mut credits_per_epochs,
                        credits,
                        epoch_index,
                    )
                    .expect("should distribute refunds");

                    let already_paid_epochs = current_epoch_index as u64 - epoch_index as u64 + 1;

                    let already_paid_credits = if already_paid_epochs > 0 {
                        credits_per_epochs
                            .into_iter()
                            .take(already_paid_epochs as usize)
                            .map(|(_, credits)| credits.to_unsigned())
                            .sum()
                    } else {
                        0
                    };

                    credits - leftovers - already_paid_credits
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
