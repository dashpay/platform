//! The pending grove operations GroveDB reads while it builds a delete.
//!
//! GroveDB's `delete_operation_for_delete_internal` and
//! `delete_operations_for_delete_up_tree_while_empty` take the operations already in the batch
//! being built, so a delete can account for them: a tree whose children the batch deletes counts
//! as empty, and a tree the batch writes into does not. They borrow those operations and ask for
//! the ones at the deleted tree's own path, only when the element is a tree. The batch delete
//! helpers pass the batch's grove operations as they are, where they used to copy the whole
//! batch for every delete.

use crate::fees::op::LowLevelDriveOperation;
use grovedb::batch::QualifiedGroveDbOp;
#[cfg(test)]
use std::cell::Cell;

#[cfg(test)]
thread_local! {
    /// How many pending grove operations this thread copied, counted by
    /// `LowLevelDriveOperation::grovedb_operations_batch`, so tests can keep a copy of the
    /// pending batch out of the per-delete path.
    pub(crate) static COPIED_PENDING_GROVE_OPERATIONS: Cell<usize> = const { Cell::new(0) };
}

/// Adds `copied` to [`COPIED_PENDING_GROVE_OPERATIONS`].
#[cfg(test)]
pub(crate) fn count_copied_pending_grove_operations(copied: usize) {
    COPIED_PENDING_GROVE_OPERATIONS.with(|count| count.set(count.get() + copied));
}

/// The grove operations in `drive_operations`, ephemeral ones included, in batch order: the
/// operations `LowLevelDriveOperation::grovedb_operations_batch` copies, borrowed instead.
pub(crate) fn pending_grove_operations(
    drive_operations: &[LowLevelDriveOperation],
) -> impl Iterator<Item = &QualifiedGroveDbOp> + Clone {
    drive_operations
        .iter()
        .filter_map(|operation| match operation {
            LowLevelDriveOperation::GroveOperation(grovedb_op)
            | LowLevelDriveOperation::EphemeralGroveOperation(grovedb_op, _) => Some(grovedb_op),
            _ => None,
        })
}

#[cfg(test)]
mod tests;
