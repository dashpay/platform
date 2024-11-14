mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::drive_versions::DriveVersion;
use grovedb::batch::QualifiedGroveDbOp;
use grovedb::TransactionArg;

impl Drive {
    /// Applies the given groveDB operation.
    ///
    /// # Parameters
    /// * `operation`: The groveDB operation to apply.
    /// * `validate`: Specifies whether to validate that insertions do not override existing entries.
    /// * `transaction`: The groveDB transaction associated with this operation.
    /// * `drive_version`: The drive version to select the correct function version to run.
    ///
    /// # Returns
    /// * `Ok(())` if the operation was successful.
    /// * `Err(DriveError::UnknownVersionMismatch)` if the drive version does not match known versions.
    pub fn grove_apply_operation(
        &self,
        operation: QualifiedGroveDbOp,
        validate: bool,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        match drive_version.grove_methods.apply.grove_apply_operation {
            0 => self.grove_apply_operation_v0(operation, validate, transaction, drive_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "grove_apply_operation".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
