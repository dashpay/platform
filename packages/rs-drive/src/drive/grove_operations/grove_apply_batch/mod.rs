use crate::drive::batch::GroveDbOpBatch;
use crate::drive::Drive;
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
        self.grove_apply_batch_with_add_costs(
            ops,
            validate,
            transaction,
            &mut vec![],
            drive_version,
        )
    }
}
