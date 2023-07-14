mod v0;

use grovedb::{Element, TransactionArg};
use std::ops::Range;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::credits::{Creditable, Credits};
use crate::fee::get_overflow_error;
use dpp::block::epoch::Epoch;
use dpp::fee::Credits;

use crate::fee_pools::epochs::epoch_key_constants;
use crate::fee_pools::epochs::paths::EpochProposers;

use dpp::version::drive_versions::DriveVersion;

impl Drive {
    /// Gets the amount of processing fees to be distributed for the Epoch.
    ///
    /// # Arguments
    ///
    /// * `epoch_tree` - A reference to the Epoch.
    /// * `transaction` - A TransactionArg instance.
    /// * `drive_version` - A DriveVersion instance representing the version of the drive.
    ///
    /// # Returns
    ///
    /// A Result containing either the processing fee for the epoch, if found,
    /// or an Error if something goes wrong.
    pub fn get_epoch_processing_credits_for_distribution(
        &self,
        epoch_tree: &Epoch,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<Credits, Error> {
        match drive_version
            .methods
            .credit_pools
            .epochs
            .get_epoch_processing_credits_for_distribution
        {
            0 => self.get_epoch_processing_credits_for_distribution_v0(epoch_tree, transaction),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "get_epoch_processing_credits_for_distribution".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
