use crate::drive::Drive;
use crate::error::Error;
use crate::util::batch::GroveDbOpBatch;
use dpp::version::drive_versions::DriveVersion;
use grovedb::batch::QualifiedGroveDbOp;
use grovedb::TransactionArg;

impl Drive {
    /// Applies the given groveDB operation
    pub(super) fn grove_apply_operation_v0(
        &self,
        operation: QualifiedGroveDbOp,
        validate: bool,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        self.grove_apply_batch_with_add_costs(
            GroveDbOpBatch {
                operations: vec![operation],
            },
            validate,
            transaction,
            &mut vec![],
            drive_version,
        )
    }
}
