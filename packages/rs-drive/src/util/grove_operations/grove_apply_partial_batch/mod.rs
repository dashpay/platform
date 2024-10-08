use crate::drive::Drive;
use crate::error::Error;
use crate::query::GroveError;
use crate::util::batch::GroveDbOpBatch;
use dpp::version::drive_versions::DriveVersion;
use grovedb::batch::{OpsByLevelPath, QualifiedGroveDbOp};
use grovedb::TransactionArg;
use grovedb_costs::OperationCost;

impl Drive {
    /// Applies the given groveDB operations batch.
    ///
    /// # Parameters
    /// * `ops`: The batch of groveDB operations to apply.
    /// * `validate`: Specifies whether to validate that insertions do not override existing entries.
    /// * `transaction`: The groveDB transaction associated with this operation.
    /// * `add_on_operations`: A closure that takes in the operation cost and optional operation by level path
    ///   and returns a result of groveDB operations or a grove error.
    /// * `drive_version`: The drive version to select the correct function version to run.
    ///
    /// # Returns
    /// * `Ok(())` if the operation was successful.
    /// * `Err(DriveError::UnknownVersionMismatch)` if the drive version does not match known versions.
    pub fn grove_apply_partial_batch(
        &self,
        ops: GroveDbOpBatch,
        validate: bool,
        transaction: TransactionArg,
        add_on_operations: impl FnMut(
            &OperationCost,
            &Option<OpsByLevelPath>,
        ) -> Result<Vec<QualifiedGroveDbOp>, GroveError>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        self.grove_apply_partial_batch_with_add_costs(
            ops,
            validate,
            transaction,
            add_on_operations,
            &mut vec![],
            drive_version,
        )
    }
}
