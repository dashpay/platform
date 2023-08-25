use crate::drive::batch::GroveDbOpBatch;
use crate::drive::Drive;
use crate::error::Error;
use crate::query::GroveError;
use dpp::version::drive_versions::DriveVersion;
use grovedb::batch::{GroveDbOp, OpsByLevelPath};
use grovedb::TransactionArg;
use grovedb_costs::OperationCost;

impl Drive {
    /// Applies the given groveDB operations batch.
    pub(super) fn grove_apply_partial_batch_v0(
        &self,
        ops: GroveDbOpBatch,
        validate: bool,
        add_on_operations: impl FnMut(
            &OperationCost,
            &Option<OpsByLevelPath>,
        ) -> Result<Vec<GroveDbOp>, GroveError>,
        transaction: TransactionArg,
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
