mod v0;

use crate::drive::batch::GroveDbOpBatch;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use crate::query::GroveError;

use dpp::version::drive_versions::DriveVersion;
use grovedb::batch::{GroveDbOp, OpsByLevelPath};
use grovedb::TransactionArg;
use grovedb_costs::OperationCost;

impl Drive {
    /// Applies the given groveDB operations batch, gets and passes the costs to `push_drive_operation_result`.
    ///
    /// # Parameters
    /// * `ops`: The batch of groveDB operations to retrieve costs for.
    /// * `validate`: Specifies whether to validate that insertions do not override existing entries.
    /// * `transaction`: The groveDB transaction associated with this operation.
    /// * `add_on_operations`: A closure that takes in the operation cost and optional operation by level path
    ///   and returns a result of groveDB operations or a grove error.
    /// * `drive_operations`: A vector to collect the costs of operations for later computation.
    /// * `drive_version`: The drive version to select the correct function version to run.
    ///
    /// # Returns
    /// * `Ok(())` if the operation was successful.
    /// * `Err(DriveError::UnknownVersionMismatch)` if the drive version does not match known versions.
    pub fn grove_apply_partial_batch_with_add_costs(
        &self,
        ops: GroveDbOpBatch,
        validate: bool,
        transaction: TransactionArg,
        add_on_operations: impl FnMut(
            &OperationCost,
            &Option<OpsByLevelPath>,
        ) -> Result<Vec<GroveDbOp>, GroveError>,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        match drive_version.grove_methods.apply.grove_apply_partial_batch {
            0 => self.grove_apply_partial_batch_with_add_costs_v0(
                ops,
                validate,
                transaction,
                add_on_operations,
                drive_operations,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "grove_apply_partial_batch_with_add_costs".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
