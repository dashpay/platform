use grovedb::TransactionArg;
use std::ops::Range;

use crate::drive::Drive;

use dpp::block::epoch::Epoch;

use dpp::version::PlatformVersion;

impl Drive {
    /// Returns a list of storage credits to be distributed to proposers from a range of epochs.
    pub(super) fn get_storage_credits_for_distribution_for_epochs_in_range_v0(
        &self,
        epoch_range: Range<u16>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Vec<u64> {
        epoch_range
            .map(|index| {
                let epoch = Epoch::new(index).unwrap();
                self.get_epoch_storage_credits_for_distribution(
                    &epoch,
                    transaction,
                    platform_version,
                )
                .expect("should get storage fee")
            })
            .collect()
    }
}
