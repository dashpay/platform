use crate::drive::grove_operations::push_drive_operation_result;
use crate::drive::Drive;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use grovedb::operations::insert::InsertOptions;
use grovedb::{Element, TransactionArg};
use grovedb_path::SubtreePath;

impl Drive {
    /// Pushes the `OperationCost` of inserting an empty sum tree in groveDB to `drive_operations`.
    pub fn grove_insert_empty_sum_tree_v0<B: AsRef<[u8]>>(
        &self,
        path: SubtreePath<'_, B>,
        key: &[u8],
        transaction: TransactionArg,
        options: Option<InsertOptions>,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
    ) -> Result<(), Error> {
        let cost_context =
            self.grove
                .insert(path, key, Element::empty_sum_tree(), options, transaction);
        push_drive_operation_result(cost_context, drive_operations)
    }
}
