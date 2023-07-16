use grovedb::{Element, TransactionArg};
use std::ops::Range;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;

use dpp::block::epoch::Epoch;
use dpp::fee::Credits;

use crate::fee_pools::epochs::epoch_key_constants;
use crate::fee_pools::epochs::paths::EpochProposers;

impl Drive {
    /// Returns a list of storage credits to be distributed to proposers from a range of epochs.
    pub(super) fn get_storage_credits_for_distribution_for_epochs_in_range_v0(
        &self,
        epoch_range: Range<u16>,
        transaction: TransactionArg,
    ) -> Vec<u64> {
        epoch_range
            .map(|index| {
                let epoch = Epoch::new(index).unwrap();
                self.get_epoch_storage_credits_for_distribution(&epoch, transaction)
                    .expect("should get storage fee")
            })
            .collect()
    }
}
