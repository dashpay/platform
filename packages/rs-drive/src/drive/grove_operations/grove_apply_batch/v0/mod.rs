use crate::drive::batch::GroveDbOpBatch;
use crate::drive::Drive;
use crate::error::Error;
use dpp::version::drive_versions::DriveVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Applies the given groveDB operations batch.
    pub(super) fn grove_apply_batch_v0(
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
