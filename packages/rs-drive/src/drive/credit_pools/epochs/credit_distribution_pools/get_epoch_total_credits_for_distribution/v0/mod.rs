
use std::ops::Range;
use grovedb::{Element, TransactionArg};

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::credits::{Creditable, Credits};
use crate::fee::get_overflow_error;
use dpp::block::epoch::Epoch;
use dpp::fee::Credits;

use crate::fee_pools::epochs::epoch_key_constants;
use crate::fee_pools::epochs::paths::EpochProposers;


impl Drive {

    /// Gets the total credits to be distributed for the Epoch.
    pub(super) fn get_epoch_total_credits_for_distribution_v0(
        &self,
        epoch_tree: &Epoch,
        transaction: TransactionArg,
    ) -> Result<Credits, Error> {
        let storage_pool_credits =
            self.get_epoch_storage_credits_for_distribution(epoch_tree, transaction)?;

        let processing_pool_credits =
            self.get_epoch_processing_credits_for_distribution(epoch_tree, transaction)?;

        storage_pool_credits
            .checked_add(processing_pool_credits)
            .ok_or_else(|| get_overflow_error("overflow getting total credits for distribution"))
    }
}


#[cfg(test)]
mod tests {
    use dpp::block::epoch::Epoch;
    use dpp::fee::Credits;
    use dpp::version::drive_versions::DriveVersion;
    use crate::drive::batch::GroveDbOpBatch;
    use crate::fee_pools::epochs::operations_factory::EpochOperations;
    use crate::tests::helpers::setup::setup_drive_with_initial_state_structure;

    #[test]
    fn test_get_epoch_total_credits_for_distribution_v0() {
        let drive = setup_drive_with_initial_state_structure();
        let transaction = drive.grove.start_transaction();
        let drive_version = DriveVersion::latest();

        let processing_fee: Credits = 42;
        let storage_fee: Credits = 1000;

        let epoch = Epoch::new(0).unwrap();

        let mut batch = GroveDbOpBatch::new();

        batch.push(
            epoch
                .update_processing_fee_pool_operation(processing_fee)
                .expect("should add operation"),
        );

        batch.push(
            epoch
                .update_storage_fee_pool_operation(storage_fee)
                .expect("should add operation"),
        );

        drive
            .grove_apply_batch(batch, false, Some(&transaction), &drive_version)
            .expect("should apply batch");

        let retrieved_combined_fee = drive
            .get_epoch_total_credits_for_distribution(&epoch, Some(&transaction))
            .expect("should get combined fee");

        assert_eq!(retrieved_combined_fee, processing_fee + storage_fee);
    }
}
