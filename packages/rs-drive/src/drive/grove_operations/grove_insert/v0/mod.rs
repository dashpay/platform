use grovedb::{Element, TransactionArg};
use grovedb::operations::insert::InsertOptions;
use path::SubtreePath;
use crate::drive::Drive;
use crate::drive::grove_operations::push_drive_operation_result;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;

impl Drive {
    /// Pushes the `OperationCost` of inserting an element in groveDB to `drive_operations`.
    pub(super) fn grove_insert_v0<B: AsRef<[u8]>>(
        &self,
        path: SubtreePath<'_, B>,
        key: &[u8],
        element: Element,
        transaction: TransactionArg,
        options: Option<InsertOptions>,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
    ) -> Result<(), Error> {
        let cost_context = self.grove.insert(path, key, element, options, transaction);
        push_drive_operation_result(cost_context, drive_operations)
    }
}