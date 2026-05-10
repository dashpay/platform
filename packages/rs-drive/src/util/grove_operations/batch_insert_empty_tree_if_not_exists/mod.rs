mod v0;

use crate::util::grove_operations::BatchInsertTreeApplyType;
use crate::util::object_size_info::PathKeyInfo;
use crate::util::storage_flags::StorageFlags;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;

use dpp::version::drive_versions::DriveVersion;
use grovedb::{TransactionArg, TreeType};

impl Drive {
    /// Pushes an "insert empty tree where path key does not yet exist" operation to `drive_operations`.
    /// Will also check the current drive operations
    /// Returns true if we inserted
    #[allow(clippy::too_many_arguments)]
    pub fn batch_insert_empty_tree_if_not_exists<const N: usize>(
        &self,
        path_key_info: PathKeyInfo<N>,
        tree_type: TreeType,
        storage_flags: Option<&StorageFlags>,
        apply_type: BatchInsertTreeApplyType,
        transaction: TransactionArg,
        check_existing_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<bool, Error> {
        match drive_version
            .grove_methods
            .batch
            .batch_insert_empty_tree_if_not_exists
        {
            0 => self.batch_insert_empty_tree_if_not_exists_v0(
                path_key_info,
                tree_type,
                false, // wrap_in_non_counted
                storage_flags,
                apply_type,
                transaction,
                check_existing_operations,
                drive_operations,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "batch_insert_empty_tree_if_not_exists".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Pushes an "insert empty `NormalTree` wrapped in `Element::NonCounted`"
    /// operation to `drive_operations`, but only if the path/key doesn't
    /// already exist (in current state OR in pending operations).
    ///
    /// Used by the index walker for sibling continuations that live inside a
    /// `range_countable` value tree (a `CountTree`). Without the `NonCounted`
    /// wrapper, an empty `NormalTree` child would contribute 1 to the parent
    /// `CountTree`'s aggregate (per grovedb's default
    /// `count_value_or_default()`); the wrapper makes it contribute 0 so the
    /// value tree's count cleanly reflects "documents at this value" rather
    /// than "documents + sibling-continuation-trees".
    #[allow(clippy::too_many_arguments)]
    pub fn batch_insert_empty_non_counted_normal_tree_if_not_exists<const N: usize>(
        &self,
        path_key_info: PathKeyInfo<N>,
        storage_flags: Option<&StorageFlags>,
        apply_type: BatchInsertTreeApplyType,
        transaction: TransactionArg,
        check_existing_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<bool, Error> {
        match drive_version
            .grove_methods
            .batch
            .batch_insert_empty_tree_if_not_exists
        {
            0 => self.batch_insert_empty_tree_if_not_exists_v0(
                path_key_info,
                TreeType::NormalTree,
                true, // wrap_in_non_counted
                storage_flags,
                apply_type,
                transaction,
                check_existing_operations,
                drive_operations,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "batch_insert_empty_non_counted_normal_tree_if_not_exists".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
