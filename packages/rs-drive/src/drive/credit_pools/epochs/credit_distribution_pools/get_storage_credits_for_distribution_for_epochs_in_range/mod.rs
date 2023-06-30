mod v0;

use std::ops::Range;
use grovedb::{Element, TransactionArg};

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::credits::{Creditable, Credits};
use crate::fee::get_overflow_error;
use dpp::block::epoch::Epoch;
use dpp::state_transition::fee::Credits;

use crate::fee_pools::epochs::epoch_key_constants;
use crate::fee_pools::epochs::paths::EpochProposers;

use dpp::version::drive_versions::DriveVersion;

impl Drive {
    /// Returns a list of storage credits to be distributed to proposers from a range of epochs.
    ///
    /// # Arguments
    ///
    /// * `epoch_range` - A Range<u16> specifying the epochs for which to get the storage credits.
    /// * `transaction` - A TransactionArg instance.
    /// * `drive_version` - A DriveVersion instance representing the version of the drive.
    ///
    /// # Returns
    ///
    /// A Vec<u64> containing the storage credits for each epoch in the specified range.
    pub fn get_storage_credits_for_distribution_for_epochs_in_range(
        &self,
        epoch_range: Range<u16>,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<Vec<u64>, Error> {
        match drive_version.methods.credit_pools.epochs.get_storage_credits_for_distribution_for_epochs_in_range {
            0 => Ok(self.get_storage_credits_for_distribution_for_epochs_in_range_v0(epoch_range, transaction)),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "get_storage_credits_for_distribution_for_epochs_in_range".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}