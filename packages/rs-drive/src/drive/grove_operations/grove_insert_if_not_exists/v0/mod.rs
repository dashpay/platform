use crate::drive::grove_operations::push_drive_operation_result_optional;
use crate::drive::Drive;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use grovedb::{Element, TransactionArg};
use path::SubtreePath;

impl Drive {
    /// Pushes the `OperationCost` of inserting an element in groveDB where the path key does not yet exist
    /// to `drive_operations`.
    pub(super) fn grove_insert_if_not_exists_v0<B: AsRef<[u8]>>(
        &self,
        path: SubtreePath<'_, B>,
        key: &[u8],
        element: Element,
        transaction: TransactionArg,
        drive_operations: Option<&mut Vec<LowLevelDriveOperation>>,
    ) -> Result<bool, Error> {
        let cost_context = self
            .grove
            .insert_if_not_exists(path, key, element, transaction);
        push_drive_operation_result_optional(cost_context, drive_operations)
    }
}
