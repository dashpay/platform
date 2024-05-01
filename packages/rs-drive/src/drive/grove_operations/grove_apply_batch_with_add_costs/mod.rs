mod v0;

use crate::drive::batch::GroveDbOpBatch;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;

use dpp::version::drive_versions::DriveVersion;

use grovedb::TransactionArg;

impl Drive {
    /// Applies the given groveDB operations batch and gets and passes the costs to `push_drive_operation_result`.
    ///
    /// # Parameters
    /// * `ops`: The groveDB operations batch.
    /// * `validate`: Specifies whether to validate that insertions do not override existing entries.
    /// * `transaction`: The groveDB transaction associated with this operation.
    /// * `drive_operations`: A vector of operations on the drive.
    /// * `drive_version`: The drive version to select the correct function version to run.
    ///
    /// # Returns
    /// * `Ok(())` if the operation was successful.
    /// * `Err(DriveError::UnknownVersionMismatch)` if the drive version does not match known versions.
    pub fn grove_apply_batch_with_add_costs(
        &self,
        ops: GroveDbOpBatch,
        validate: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        match drive_version.grove_methods.apply.grove_apply_batch {
            0 => self.grove_apply_batch_with_add_costs_v0(
                ops,
                validate,
                transaction,
                drive_operations,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "grove_apply_batch_with_add_costs".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
