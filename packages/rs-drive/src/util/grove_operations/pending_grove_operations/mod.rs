//! The pending grove operations GroveDB reads while it builds a delete.
//!
//! GroveDB's `delete_operation_for_delete_internal` takes the operations already in the batch
//! being built so it can account for them when the element it deletes is a tree: it treats the
//! tree as empty if the batch deletes all of its children, and as not empty if the batch writes
//! anything else into it. It reads only the operations whose path is that tree's own path (the
//! delete's path followed by its key), and none at all when the element is not a tree. Each
//! operation's result and cost depend on nothing else in the batch, so the batch delete helpers
//! pass GroveDB those operations alone instead of a copy of the whole batch per delete.

use crate::fees::op::LowLevelDriveOperation;
use grovedb::batch::{KeyInfoPath, QualifiedGroveDbOp};
use grovedb::MaybeTree;
use grovedb_path::SubtreePath;
#[cfg(test)]
use std::cell::Cell;

#[cfg(test)]
thread_local! {
    /// How many pending grove operations this thread copied to hand to GroveDB, counted by
    /// every copy here and by `LowLevelDriveOperation::grovedb_operations_batch`, so tests can
    /// keep a copy of the pending batch out of the per-delete path.
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
) -> impl Iterator<Item = &QualifiedGroveDbOp> {
    drive_operations
        .iter()
        .filter_map(|operation| match operation {
            LowLevelDriveOperation::GroveOperation(grovedb_op)
            | LowLevelDriveOperation::EphemeralGroveOperation(grovedb_op) => Some(grovedb_op),
            _ => None,
        })
}

fn copy_for_grovedb<'a>(
    operations: impl Iterator<Item = &'a QualifiedGroveDbOp>,
) -> Vec<QualifiedGroveDbOp> {
    let copied: Vec<QualifiedGroveDbOp> = operations.cloned().collect();
    #[cfg(test)]
    count_copied_pending_grove_operations(copied.len());
    copied
}

/// The pending grove operations `delete_operation_for_delete_internal` reads when it deletes
/// `key` under `path` with the `is_known_to_be_subtree` hint.
///
/// These are the operations at `path` followed by `key`, the path GroveDB compares each pending
/// operation's path against when the element is a tree, and none when the hint says the element
/// is not a tree, since GroveDB then takes the hint without reading the element or the batch.
pub(crate) fn pending_grove_operations_for_delete<B: AsRef<[u8]>>(
    drive_operations: &[LowLevelDriveOperation],
    path: &SubtreePath<'_, B>,
    key: &[u8],
    is_known_to_be_subtree: Option<MaybeTree>,
) -> Vec<QualifiedGroveDbOp> {
    if is_known_to_be_subtree == Some(MaybeTree::NotTree) {
        return vec![];
    }
    let mut subtree_path = path.to_vec();
    subtree_path.push(key.to_vec());
    copy_for_grovedb(
        pending_grove_operations(drive_operations).filter(|op| op.path.eq_path_vec(&subtree_path)),
    )
}

/// The pending grove operations `delete_operations_for_delete_up_tree_while_empty` reads when it
/// deletes `key` under `path` and then each tree above it that the delete leaves empty, up to
/// `stop_path_height`, taken from `drive_operations` and then `check_existing_operations`, the
/// order GroveDB received them in.
///
/// Every level is a `delete_operation_for_delete_internal` of the tree holding the level below
/// it (see [`pending_grove_operations_for_delete`]), so a level deleting under a path of length
/// `l` reads the operations at the first `l + 1` segments of `path` followed by `key`. GroveDB
/// deletes under `path` first, then under each shorter prefix, and stops before the prefix whose
/// length is `stop_path_height`; the deletes it adds between levels sit on those paths too.
pub(crate) fn pending_grove_operations_for_delete_up_tree(
    drive_operations: &[LowLevelDriveOperation],
    check_existing_operations: Option<&[LowLevelDriveOperation]>,
    path: &KeyInfoPath,
    key: &[u8],
    stop_path_height: Option<u16>,
) -> Vec<QualifiedGroveDbOp> {
    // GroveDB receives the path as `path.to_path_refs()`.
    let mut deleted_path: Vec<Vec<u8>> = path
        .to_path_refs()
        .into_iter()
        .map(|segment| segment.to_vec())
        .collect();
    deleted_path.push(key.to_vec());

    // The same check, in the same order, GroveDB makes before each level.
    let mut shortest_read_path = None;
    for level_path_length in (0..deleted_path.len()).rev() {
        if stop_path_height == Some(u16::try_from(level_path_length).unwrap_or(u16::MAX)) {
            break;
        }
        shortest_read_path = Some(level_path_length + 1);
    }
    let Some(shortest_read_path) = shortest_read_path else {
        return vec![];
    };

    let read_by_some_level = |op: &&QualifiedGroveDbOp| {
        let length = op.path.0.len();
        length >= shortest_read_path
            && deleted_path
                .get(..length)
                .is_some_and(|read_path| op.path.eq_path_vec(read_path))
    };
    copy_for_grovedb(
        pending_grove_operations(drive_operations)
            .chain(pending_grove_operations(
                check_existing_operations.unwrap_or_default(),
            ))
            .filter(read_by_some_level),
    )
}

#[cfg(test)]
mod tests;
