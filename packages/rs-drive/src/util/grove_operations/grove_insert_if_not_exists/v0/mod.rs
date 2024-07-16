use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::push_drive_operation_result_optional;
use grovedb::{Element, TransactionArg};
use grovedb_path::SubtreePath;
use platform_version::version::drive_versions::DriveVersion;

impl Drive {
    /// Pushes the `OperationCost` of inserting an element in groveDB where the path key does not yet exist
    /// to `drive_operations`.
    pub(crate) fn grove_insert_if_not_exists_v0<B: AsRef<[u8]>>(
        &self,
        path: SubtreePath<'_, B>,
        key: &[u8],
        element: Element,
        transaction: TransactionArg,
        drive_operations: Option<&mut Vec<LowLevelDriveOperation>>,
        drive_version: &DriveVersion,
    ) -> Result<bool, Error> {
        let cost_context = self.grove.insert_if_not_exists(
            path,
            key,
            element,
            transaction,
            &drive_version.grove_version,
        );
        push_drive_operation_result_optional(cost_context, drive_operations)
    }
}
