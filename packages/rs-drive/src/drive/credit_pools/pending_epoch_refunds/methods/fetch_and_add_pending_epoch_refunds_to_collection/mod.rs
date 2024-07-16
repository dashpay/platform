mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::fee::epoch::CreditsPerEpoch;
use dpp::version::drive_versions::DriveVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Fetches pending epoch refunds and adds them to specified collection based on the drive version
    ///
    /// # Arguments
    ///
    /// * `refunds_per_epoch` - CreditsPerEpoch instance representing refunds per epoch.
    /// * `transaction` - A TransactionArg instance.
    /// * `drive_version` - A DriveVersion instance representing the version of the drive.
    ///
    /// # Returns
    ///
    /// A Result containing CreditsPerEpoch or an Error.
    pub fn fetch_and_add_pending_epoch_refunds_to_collection(
        &self,
        refunds_per_epoch: CreditsPerEpoch,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<CreditsPerEpoch, Error> {
        match drive_version
            .methods
            .credit_pools
            .pending_epoch_refunds
            .fetch_and_add_pending_epoch_refunds_to_collection
        {
            0 => self.fetch_and_add_pending_epoch_refunds_to_collection_v0(
                refunds_per_epoch,
                transaction,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_and_add_pending_epoch_refunds_to_collection".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
