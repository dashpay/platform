mod v0;

use crate::util::grove_operations::BatchInsertTreeApplyType;
use crate::util::object_size_info::PathKeyInfo;
use crate::util::storage_flags::StorageFlags;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;

use dpp::version::drive_versions::DriveVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Pushes an "insert empty tree where path key does not yet exist" operation to `drive_operations`.
    /// Will also check the current drive operations
    /// Returns true if we inserted
    pub fn batch_insert_empty_tree_if_not_exists<const N: usize>(
        &self,
        path_key_info: PathKeyInfo<N>,
        use_sum_tree: bool,
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
                use_sum_tree,
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
}
