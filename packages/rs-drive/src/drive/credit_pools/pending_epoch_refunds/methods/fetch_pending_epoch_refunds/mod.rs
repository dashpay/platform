mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::fee::epoch::CreditsPerEpoch;
use dpp::version::drive_versions::DriveVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Fetches all pending epoch refunds based on the drive version
    ///
    /// # Arguments
    ///
    /// * `transaction` - A TransactionArg instance.
    /// * `drive_version` - A DriveVersion instance representing the version of the drive.
    ///
    /// # Returns
    ///
    /// A Result containing CreditsPerEpoch or an Error.
    pub fn fetch_pending_epoch_refunds(
        &self,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<CreditsPerEpoch, Error> {
        match drive_version
            .methods
            .credit_pools
            .pending_epoch_refunds
            .fetch_pending_epoch_refunds
        {
            0 => self.fetch_pending_epoch_refunds_v0(transaction, drive_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_pending_epoch_refunds".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
