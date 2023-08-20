mod v0;

use crate::drive::batch::GroveDbOpBatch;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::drive_versions::DriveVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Applies the given groveDB operations batch.
    ///
    /// # Parameters
    /// * `ops`: The groveDB operations batch.
    /// * `validate`: Specifies whether to validate that insertions do not override existing entries.
    /// * `transaction`: The groveDB transaction associated with this operation.
    /// * `drive_version`: The drive version to select the correct function version to run.
    ///
    /// # Returns
    /// * `Ok(())` if the operation was successful.
    /// * `Err(DriveError::UnknownVersionMismatch)` if the drive version does not match known versions.
    pub fn grove_apply_batch(
        &self,
        ops: GroveDbOpBatch,
        validate: bool,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        match drive_version.grove_methods.apply.grove_apply_batch {
            0 => self.grove_apply_batch_v0(ops, validate, transaction, drive_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "grove_apply_batch".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
