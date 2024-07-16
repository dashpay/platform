mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::util::batch::GroveDbOpBatch;
use dpp::fee::epoch::CreditsPerEpoch;
use dpp::version::drive_versions::DriveVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Adds operations to delete pending epoch refunds except epochs from provided collection based on the drive version
    ///
    /// # Arguments
    ///
    /// * `batch` - A GroveDbOpBatch instance.
    /// * `refunds_per_epoch` - A CreditsPerEpoch instance.
    /// * `transaction` - A TransactionArg instance.
    /// * `drive_version` - A DriveVersion instance representing the version of the drive.
    ///
    /// # Returns
    ///
    /// A Result indicating success or an Error.
    pub fn add_delete_pending_epoch_refunds_except_specified_operations(
        &self,
        batch: &mut GroveDbOpBatch,
        refunds_per_epoch: &CreditsPerEpoch,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        match drive_version
            .methods
            .credit_pools
            .pending_epoch_refunds
            .add_delete_pending_epoch_refunds_except_specified
        {
            0 => self.add_delete_pending_epoch_refunds_except_specified_operations_v0(
                batch,
                refunds_per_epoch,
                transaction,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_delete_pending_epoch_refunds_except_specified_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
